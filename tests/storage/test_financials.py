from __future__ import annotations

import sqlite3

from stock_db.storage.financials import (
    get_cached_periods,
    get_financial_dict,
    get_historical_items,
    upsert_financial_item,
    upsert_financial_items_bulk,
)


class TestFinancialItems:
    def test_upsert_and_get_dict(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 1000.0, "irbank")
        upsert_financial_item(db_conn, "1234", "2024", "bs", "total_assets", 5000.0, "irbank")
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234", "2024")

        assert result["pl"]["revenue"] == 1000.0
        assert result["bs"]["total_assets"] == 5000.0

    def test_get_dict_uses_latest_period(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2023", "pl", "revenue", 900.0, "irbank")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 1000.0, "irbank")
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234")

        assert result["pl"]["revenue"] == 1000.0

    def test_get_dict_returns_empty_for_missing(self, db_conn: sqlite3.Connection) -> None:
        result = get_financial_dict(db_conn, "9999")

        assert result == {}

    def test_bulk_upsert(self, db_conn: sqlite3.Connection) -> None:
        rows = [
            {"ticker": "1234", "period": "2024", "statement": "pl", "item_name": "revenue", "value": 100.0, "source": "irbank"},
            {"ticker": "1234", "period": "2024", "statement": "pl", "item_name": "net_income", "value": 10.0, "source": "irbank"},
        ]
        upsert_financial_items_bulk(db_conn, rows)
        db_conn.commit()

        result = get_financial_dict(db_conn, "1234", "2024")
        assert result["pl"]["revenue"] == 100.0
        assert result["pl"]["net_income"] == 10.0

    def test_get_historical_items(self, db_conn: sqlite3.Connection) -> None:
        for year in range(2020, 2025):
            upsert_financial_item(db_conn, "1234", str(year), "pl", "revenue", float(year), "irbank")
        db_conn.commit()

        result = get_historical_items(db_conn, "1234", "pl", n_periods=3)

        assert len(result) == 3
        assert result[0][0] == "2024"
        assert result[0][1]["revenue"] == 2024.0

    def test_get_cached_periods(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2023", "pl", "revenue", 1.0, "irbank")
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 2.0, "irbank")
        db_conn.commit()

        result = get_cached_periods(db_conn, "1234", "pl")

        assert result == {"2023", "2024"}
