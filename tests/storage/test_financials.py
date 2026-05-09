from __future__ import annotations

import sqlite3

from stock_db.storage.financials import (
    get_cached_periods,
    get_financial_dict,
    get_historical_items,
    get_items_by_source,
    purge_financial_items_for_source,
    replace_financial_items_for_ticker_sources,
    upsert_financial_item,
    upsert_financial_items_bulk,
)


class TestFinancialItems:
    def test_upsert_and_get_dict(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 1000.0, "test_source")
        upsert_financial_item(db_conn, "1234", "2024", "bs", "total_assets", 5000.0, "test_source")
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234", "2024")

        assert result["pl"]["revenue"] == 1000.0
        assert result["bs"]["total_assets"] == 5000.0

    def test_get_dict_uses_latest_period(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2023", "pl", "revenue", 900.0, "test_source")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 1000.0, "test_source")
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234")

        assert result["pl"]["revenue"] == 1000.0

    def test_get_dict_returns_empty_for_missing(self, db_conn: sqlite3.Connection) -> None:
        result = get_financial_dict(db_conn, "9999")

        assert result == {}

    def test_bulk_upsert(self, db_conn: sqlite3.Connection) -> None:
        rows = [
            {"ticker": "1234", "period": "2024", "statement": "pl", "item_name": "revenue", "value": 100.0, "source": "test_source"},
            {"ticker": "1234", "period": "2024", "statement": "pl", "item_name": "net_income", "value": 10.0, "source": "test_source"},
        ]
        upsert_financial_items_bulk(db_conn, rows)
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234", "2024")
        assert result["pl"]["revenue"] == 100.0
        assert result["pl"]["net_income"] == 10.0

    def test_get_historical_items(self, db_conn: sqlite3.Connection) -> None:
        for year in range(2020, 2025):
            upsert_financial_item(db_conn, "1234", str(year), "pl", "revenue", float(year), "test_source")
        db_conn.commit()

        result = get_historical_items(db_conn, "1234", "pl", n_periods=3)

        assert len(result) == 3
        assert result[0][0] == "2024"
        assert result[0][1]["revenue"] == 2024.0

    def test_get_cached_periods(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2023", "pl", "revenue", 1.0, "test_source")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 2.0, "test_source")
        db_conn.commit()

        result = get_cached_periods(db_conn, "1234", "pl")

        assert result == {"2023", "2024"}

    def test_get_historical_items_empty(self, db_conn: sqlite3.Connection) -> None:
        result = get_historical_items(db_conn, "9999", "pl")

        assert result == []

    def test_get_dict_includes_new_statement_type(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "custom_stmt", "metric", 42.0, "test_source")
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234", "2024")

        assert result["custom_stmt"]["metric"] == 42.0

    def test_purge_financial_items_for_source(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "bs", "x", 1.0, "test_source_alt")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "y", 2.0, "test_source")
        db_conn.commit()

        deleted = purge_financial_items_for_source(db_conn, "test_source_alt")
        db_conn.commit()

        assert deleted == 1
        rows = db_conn.execute(
            "SELECT statement, item_name, source FROM financial_items ORDER BY statement, item_name"
        ).fetchall()
        assert [tuple(row) for row in rows] == [("pl", "y", "test_source")]


class TestGetItemsBySource:
    def test_returns_rows_for_matching_source(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "8888", "2025-03", "bs", "current_assets", 38675872000.0, "xbrl_bs")
        upsert_financial_item(db_conn, "8888", "2025-03", "bs", "inventories", 32974467000.0, "xbrl_bs")
        upsert_financial_item(db_conn, "8888", "2025-03", "pl", "revenue", 1000.0, "other_source")
        db_conn.commit()

        rows = get_items_by_source(db_conn, "8888", "xbrl_bs")
        assert len(rows) == 2
        assert rows[0]["item_name"] == "current_assets"

    def test_returns_empty_for_no_match(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 100.0, "other")
        db_conn.commit()

        rows = get_items_by_source(db_conn, "1234", "xbrl_bs")
        assert rows == []

    def test_includes_status_rows(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "7000", "2025-03", "_status", "blocked", None, "xbrl_bs")
        db_conn.commit()

        rows = get_items_by_source(db_conn, "7000", "xbrl_bs")
        assert len(rows) == 1
        assert rows[0]["statement"] == "_status"
        assert rows[0]["item_name"] == "blocked"

    def test_orders_by_period_desc(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2023", "bs", "x", 1.0, "xbrl_bs")
        upsert_financial_item(db_conn, "1234", "2025", "bs", "x", 3.0, "xbrl_bs")
        upsert_financial_item(db_conn, "1234", "2024", "bs", "x", 2.0, "xbrl_bs")
        db_conn.commit()

        rows = get_items_by_source(db_conn, "1234", "xbrl_bs")
        periods = [r["period"] for r in rows]
        assert periods == ["2025", "2024", "2023"]


class TestReplaceFinancialItemsForTickerSources:
    def test_replaces_only_requested_sources(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "bs", "current_assets", 100.0, "irbank_bs")
        upsert_financial_item(db_conn, "1234", "2024", "bs", "inventories", 20.0, "xbrl_bs")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 300.0, "manual")
        db_conn.commit()

        replace_financial_items_for_ticker_sources(
            db_conn,
            ticker="1234",
            sources=("irbank_bs", "xbrl_bs", "edinet_xbrl"),
            rows=[
                {
                    "ticker": "1234",
                    "period": "2024",
                    "statement": "bs",
                    "item_name": "current_assets",
                    "value": 111.0,
                    "source": "edinet_xbrl",
                }
            ],
        )
        db_conn.commit()

        rows = db_conn.execute(
            """
            SELECT statement, item_name, value, source
            FROM financial_items
            WHERE ticker = '1234'
            ORDER BY statement, item_name
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [
            ("bs", "current_assets", 111.0, "edinet_xbrl"),
            ("pl", "revenue", 300.0, "manual"),
        ]
