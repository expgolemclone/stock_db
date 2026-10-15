from __future__ import annotations

import sqlite3

import pytest

from stock_db.storage.schema import SCHEMA_VERSION, init_db


def test_init_db_is_versioned_and_idempotent() -> None:
    conn = sqlite3.connect(":memory:")

    init_db(conn)
    init_db(conn)

    version = conn.execute("PRAGMA user_version").fetchone()[0]
    assert version == SCHEMA_VERSION == 4
    assert conn.execute(
        "SELECT type FROM sqlite_master WHERE name = 'price_observations'"
    ).fetchone() == ("table",)
    assert conn.execute(
        "SELECT type FROM sqlite_master WHERE name = 'prices'"
    ).fetchone() == ("view",)
    assert conn.execute(
        "SELECT type FROM sqlite_master WHERE name = 'financial_statement_contexts'"
    ).fetchone() == ("view",)
    assert conn.execute(
        "SELECT type FROM sqlite_master WHERE name = 'canonical_financial_items'"
    ).fetchone() == ("view",)


def test_raw_xbrl_tables_preserve_accounting_context_metadata() -> None:
    conn = sqlite3.connect(":memory:")
    init_db(conn)

    expected = {
        "period_start",
        "period_end",
        "duration_days",
        "consolidation_scope",
        "unit",
        "unit_ref",
        "decimals",
    }
    for table in (
        "xbrl_balance_sheet_facts",
        "xbrl_profit_loss_facts",
        "xbrl_cash_flow_facts",
    ):
        columns = {row[1] for row in conn.execute(f"PRAGMA table_info({table})")}
        assert expected <= columns


def test_reported_xbrl_eps_is_canonical_and_computed_historical_eps_is_not() -> None:
    conn = sqlite3.connect(":memory:")
    init_db(conn)
    conn.execute("INSERT INTO stock_entities (ticker, name) VALUES ('1234', 'test')")
    for source in ("edinet_xbrl", "computed"):
        conn.execute("INSERT INTO data_sources (source_code) VALUES (?)", (source,))
    for item_name in ("earnings_per_share", "diluted_earnings_per_share", "eps"):
        conn.execute(
            "INSERT INTO financial_item_defs (statement, item_name) VALUES ('pl', ?)",
            (item_name,),
        )

    source_ids = dict(conn.execute("SELECT source_code, source_id FROM data_sources"))
    item_ids = dict(
        conn.execute(
            "SELECT item_name, item_id FROM financial_item_defs WHERE statement = 'pl'"
        )
    )
    context_key = "2025-03|2024-04-01|2025-03-31|CurrentYearDuration"
    for item_name, value in (
        ("earnings_per_share", 123.45),
        ("diluted_earnings_per_share", 120.0),
    ):
        conn.execute(
            """
            INSERT INTO financial_values
                (ticker, period, item_id, source_id, value, updated_at)
            VALUES ('1234', ?, ?, ?, ?, '2026-01-01')
            """,
            (context_key, item_ids[item_name], source_ids["edinet_xbrl"], value),
        )
    conn.execute(
        """
        INSERT INTO financial_values
            (ticker, period, item_id, source_id, value, updated_at)
        VALUES ('1234', '2025-03', ?, ?, 999.0, '2026-01-01')
        """,
        (item_ids["eps"], source_ids["computed"]),
    )
    conn.execute(
        """
        INSERT INTO xbrl_profit_loss_facts
            (ticker, doc_id, period, period_start, period_end, duration_days,
             source_id, consolidation_scope, context_id, dimensions_json,
             concept_namespace, concept_name, canonical_statement,
             canonical_item_name, taxonomy_status, role_uri, unit, unit_ref,
             decimals, value, updated_at)
        VALUES
            ('1234', 'DOC1', '2025-03', '2024-04-01', '2025-03-31', 365,
             ?, 'consolidated', 'CurrentYearDuration', '[]',
             'urn:test', 'BasicEarningsLossPerShare', 'pl',
             'earnings_per_share', 'semantic', '', 'JPY/share', 'JPYPerShare',
             '2', 123.45, '2026-01-01')
        """,
        (source_ids["edinet_xbrl"],),
    )

    rows = conn.execute(
        """
        SELECT item_name, value, source, period_start, period_end,
               duration_days, reporting_scope, context_id
        FROM canonical_financial_items
        WHERE ticker = '1234' AND period = '2025-03' AND statement = 'pl'
        ORDER BY item_name
        """
    ).fetchall()
    assert rows == [
        (
            "diluted_eps",
            120.0,
            "edinet_xbrl",
            "2024-04-01",
            "2025-03-31",
            365,
            "consolidated",
            "CurrentYearDuration",
        ),
        (
            "eps",
            123.45,
            "edinet_xbrl",
            "2024-04-01",
            "2025-03-31",
            365,
            "consolidated",
            "CurrentYearDuration",
        ),
    ]


def test_statement_contexts_preserve_distinct_durations() -> None:
    conn = sqlite3.connect(":memory:")
    init_db(conn)
    conn.execute("INSERT INTO stock_entities (ticker, name) VALUES ('1234', 'test')")
    conn.execute("INSERT INTO data_sources (source_code) VALUES ('edinet_xbrl')")
    source_id = conn.execute(
        "SELECT source_id FROM data_sources WHERE source_code = 'edinet_xbrl'"
    ).fetchone()[0]

    for context_id, period_start, duration_days, scope in (
        ("NineMonths", "2024-07-01", 274, "consolidated"),
        ("FullYear", "2024-04-01", 365, "consolidated"),
        ("Standalone", "2024-04-01", 365, "non_consolidated"),
    ):
        conn.execute(
            """
            INSERT INTO xbrl_profit_loss_facts
                (ticker, doc_id, period, period_start, period_end, duration_days,
                 source_id, consolidation_scope, context_id, dimensions_json,
                 concept_namespace, concept_name, taxonomy_status, role_uri,
                 unit, unit_ref, decimals, value, updated_at)
            VALUES
                ('1234', 'DOC1', '2025-03', ?, '2025-03-31', ?, ?, ?, ?, '[]',
                 'urn:test', 'Revenue', 'semantic', '', 'JPY', 'JPY', '-3', 100,
                 '2026-01-01')
            """,
            (period_start, duration_days, source_id, scope, context_id),
        )

    rows = conn.execute(
        """
        SELECT context_id, period_start, period_end, duration_days, reporting_scope,
               context_key
        FROM financial_statement_contexts
        WHERE ticker = '1234' AND period = '2025-03' AND statement = 'pl'
        ORDER BY duration_days, context_id
        """
    ).fetchall()
    assert rows == [
        (
            "NineMonths",
            "2024-07-01",
            "2025-03-31",
            274,
            "consolidated",
            "2025-03|2024-07-01|2025-03-31|NineMonths",
        ),
        (
            "FullYear",
            "2024-04-01",
            "2025-03-31",
            365,
            "consolidated",
            "2025-03|2024-04-01|2025-03-31|FullYear",
        ),
    ]


def test_canonical_values_keep_same_month_durations_separate() -> None:
    conn = sqlite3.connect(":memory:")
    init_db(conn)
    conn.execute("INSERT INTO stock_entities (ticker, name) VALUES ('1234', 'test')")
    conn.execute("INSERT INTO data_sources (source_code) VALUES ('edinet_xbrl')")
    conn.execute(
        "INSERT INTO financial_item_defs (statement, item_name) VALUES ('pl', 'revenue')"
    )
    source_id = conn.execute(
        "SELECT source_id FROM data_sources WHERE source_code = 'edinet_xbrl'"
    ).fetchone()[0]
    item_id = conn.execute(
        "SELECT item_id FROM financial_item_defs WHERE statement = 'pl' AND item_name = 'revenue'"
    ).fetchone()[0]

    contexts = (
        ("Quarter", "2025-01-01", 90, 300.0),
        ("FullYear", "2024-04-01", 365, 1000.0),
    )
    for context_id, period_start, duration_days, value in contexts:
        context_key = f"2025-03|{period_start}|2025-03-31|{context_id}"
        conn.execute(
            """
            INSERT INTO financial_values
                (ticker, period, item_id, source_id, value, updated_at)
            VALUES ('1234', ?, ?, ?, ?, '2026-01-01')
            """,
            (context_key, item_id, source_id, value),
        )
        conn.execute(
            """
            INSERT INTO xbrl_profit_loss_facts
                (ticker, doc_id, period, period_start, period_end, duration_days,
                 source_id, consolidation_scope, context_id, dimensions_json,
                 concept_namespace, concept_name, canonical_statement,
                 canonical_item_name, taxonomy_status, role_uri, unit, unit_ref,
                 decimals, value, updated_at)
            VALUES
                ('1234', 'DOC1', '2025-03', ?, '2025-03-31', ?, ?,
                 'consolidated', ?, '[]', 'urn:test', 'Revenue', 'pl', 'revenue',
                 'semantic', '', 'JPY', 'JPY', '0', ?, '2026-01-01')
            """,
            (period_start, duration_days, source_id, context_id, value),
        )

    rows = conn.execute(
        """
        SELECT period, value, period_start, period_end, duration_days, context_id
        FROM canonical_financial_items
        WHERE ticker = '1234' AND statement = 'pl' AND item_name = 'revenue'
        ORDER BY duration_days
        """
    ).fetchall()
    assert rows == [
        ("2025-03", 300.0, "2025-01-01", "2025-03-31", 90, "Quarter"),
        ("2025-03", 1000.0, "2024-04-01", "2025-03-31", 365, "FullYear"),
    ]


def test_init_db_rejects_unversioned_existing_database() -> None:
    conn = sqlite3.connect(":memory:")
    conn.execute("CREATE TABLE legacy (id INTEGER PRIMARY KEY)")
    conn.commit()

    with pytest.raises(RuntimeError, match="unversioned stocks DB"):
        init_db(conn)

    assert conn.execute("PRAGMA user_version").fetchone()[0] == 0
    assert conn.execute(
        "SELECT COUNT(*) FROM sqlite_master WHERE name = 'stock_entities'"
    ).fetchone()[0] == 0


def test_ticker_foreign_keys_reject_orphan_rows() -> None:
    conn = sqlite3.connect(":memory:")
    init_db(conn)

    with pytest.raises(sqlite3.IntegrityError):
        conn.execute(
            """
            INSERT INTO stock_share_counts
                (ticker, shares_outstanding, shares_updated_at, updated_at)
            VALUES ('9999', 100, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
            """
        )
