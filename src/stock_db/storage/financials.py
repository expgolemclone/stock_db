from __future__ import annotations

import sqlite3

from stock_db.storage._refs import ensure_data_source, ensure_financial_item
from stock_db.storage._util import utc_now_iso


def upsert_financial_item(
    conn: sqlite3.Connection,
    ticker: str,
    period: str,
    statement: str,
    item_name: str,
    value: float | None,
    source: str,
) -> None:
    source_id = ensure_data_source(conn, source)
    item_id = ensure_financial_item(conn, statement, item_name)
    conn.execute(
        """
        INSERT INTO financial_values
            (ticker, period, item_id, source_id, value, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, period, item_id, source_id) DO UPDATE SET
            value=excluded.value,
            updated_at=excluded.updated_at
        """,
        (ticker, period, item_id, source_id, value, utc_now_iso()),
    )


def upsert_financial_items_bulk(
    conn: sqlite3.Connection,
    rows: list[dict],
) -> None:
    now = utc_now_iso()
    for row in rows:
        source_id = ensure_data_source(conn, str(row["source"]))
        item_id = ensure_financial_item(
            conn,
            str(row["statement"]),
            str(row["item_name"]),
        )
        conn.execute(
            """
            INSERT INTO financial_values
                (ticker, period, item_id, source_id, value, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(ticker, period, item_id, source_id) DO UPDATE SET
                value=excluded.value,
                updated_at=excluded.updated_at
            """,
            (row["ticker"], row["period"], item_id, source_id, row["value"], now),
        )


def replace_financial_items_for_source(
    conn: sqlite3.Connection,
    ticker: str,
    source: str,
    rows: list[dict],
) -> None:
    conn.execute(
        """
        DELETE FROM financial_values
        WHERE ticker = ?
          AND source_id = (
              SELECT source_id FROM data_sources WHERE source_code = ?
          )
        """,
        (ticker, source),
    )
    if rows:
        upsert_financial_items_bulk(conn, rows)


def replace_financial_items_for_ticker_sources(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    sources: tuple[str, ...],
    rows: list[dict],
) -> None:
    placeholders = ",".join("?" for _ in sources)
    conn.execute(
        f"""
        DELETE FROM financial_values
        WHERE ticker = ?
          AND source_id IN (
              SELECT source_id FROM data_sources
              WHERE source_code IN ({placeholders})
          )
        """,
        (ticker, *sources),
    )
    if rows:
        upsert_financial_items_bulk(conn, rows)


def purge_financial_items_for_source(
    conn: sqlite3.Connection,
    source: str,
) -> int:
    cursor = conn.execute(
        """
        DELETE FROM financial_values
        WHERE source_id = (
            SELECT source_id FROM data_sources WHERE source_code = ?
        )
        """,
        (source,),
    )
    return cursor.rowcount


def purge_financial_items_for_source_like(
    conn: sqlite3.Connection,
    source_pattern: str,
) -> int:
    cursor = conn.execute(
        """
        DELETE FROM financial_values
        WHERE source_id IN (
            SELECT source_id FROM data_sources WHERE source_code LIKE ?
        )
        """,
        (source_pattern,),
    )
    return cursor.rowcount


def get_financial_dict(
    conn: sqlite3.Connection,
    ticker: str,
    period: str | None = None,
) -> dict[str, dict[str, float | None]]:
    if period is None:
        row = conn.execute(
            """
            SELECT period FROM canonical_financial_items
            WHERE ticker = ? AND statement = 'pl'
            ORDER BY period DESC LIMIT 1
            """,
            (ticker,),
        ).fetchone()
        if row is None:
            return {}
        period = row["period"]

    rows = conn.execute(
        """
        SELECT statement, item_name, value
        FROM canonical_financial_items
        WHERE ticker = ? AND period = ?
          AND statement IN ('bs', 'pl', 'cf')
        """,
        (ticker, period),
    ).fetchall()

    result: dict[str, dict[str, float | None]] = {}
    for r in rows:
        result.setdefault(r["statement"], {})[r["item_name"]] = r["value"]

    forecast_rows = conn.execute(
        """
        SELECT item_name, value FROM canonical_financial_items
        WHERE ticker = ? AND statement = 'forecast'
          AND period = (
              SELECT MAX(period) FROM canonical_financial_items
              WHERE ticker = ? AND statement = 'forecast'
          )
        """,
        (ticker, ticker),
    ).fetchall()
    for r in forecast_rows:
        result.setdefault("forecast", {})[r["item_name"]] = r["value"]

    shikiho_div_rows = conn.execute(
        """
        SELECT item_name, value FROM canonical_financial_items
        WHERE ticker = ? AND statement = 'dividend'
          AND period = (
              SELECT MAX(period) FROM canonical_financial_items
              WHERE ticker = ? AND statement = 'dividend'
          )
        """,
        (ticker, ticker),
    ).fetchall()
    for r in shikiho_div_rows:
        result.setdefault("dividend", {})[r["item_name"]] = r["value"]

    return result


def get_historical_items(
    conn: sqlite3.Connection,
    ticker: str,
    statement: str,
    n_periods: int = 5,
) -> list[tuple[str, dict[str, float | None]]]:
    rows = conn.execute(
        """
        SELECT period, item_name, value FROM canonical_financial_items
        WHERE ticker = ? AND statement = ?
          AND period IN (
              SELECT DISTINCT period FROM canonical_financial_items
              WHERE ticker = ? AND statement = ?
              ORDER BY period DESC LIMIT ?
          )
        ORDER BY period DESC
        """,
        (ticker, statement, ticker, statement, n_periods),
    ).fetchall()

    grouped: dict[str, dict[str, float | None]] = {}
    for r in rows:
        grouped.setdefault(r["period"], {})[r["item_name"]] = r["value"]

    return [(period, items) for period, items in sorted(grouped.items(), reverse=True)]


def get_cached_periods(
    conn: sqlite3.Connection,
    ticker: str,
    statement: str,
) -> set[str]:
    rows = conn.execute(
        """
        SELECT DISTINCT period FROM canonical_financial_items
        WHERE ticker = ? AND statement = ?
        """,
        (ticker, statement),
    ).fetchall()
    return {r["period"] for r in rows}


def get_items_by_source(
    conn: sqlite3.Connection,
    ticker: str,
    source: str,
) -> list[sqlite3.Row]:
    """Return all financial_items rows for a ticker/source, ordered by period DESC."""
    rows = conn.execute(
        """
        SELECT period, statement, item_name, value
        FROM financial_items
        WHERE ticker = ? AND source = ?
        ORDER BY period DESC, statement, item_name
        """,
        (ticker, source),
    ).fetchall()
    return list(rows)
