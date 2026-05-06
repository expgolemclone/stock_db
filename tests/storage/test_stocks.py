from __future__ import annotations

import sqlite3

from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.stocks import (
    get_all_tickers,
    get_edinet_code,
    get_existing_tickers,
    get_stock_names,
    get_ticker_edinet_map,
    get_ticker_suffix_map,
    get_validation_targets,
    upsert_company_metadata,
    upsert_stock,
)


class TestUpsertStock:
    def test_insert_new_stock(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト株式", "情報通信", "東証プライム")
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["name"] == "テスト株式"
        assert row["sector"] == "情報通信"

    def test_update_existing_stock(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "旧名", "旧業種", "旧市場")
        upsert_stock(db_conn, "1234", "新名", "新業種", "新市場")
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["name"] == "新名"

    def test_preserves_non_empty_fields_on_empty_update(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "情報通信", "東証プライム")
        upsert_stock(db_conn, "1234", "", "", "")
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["name"] == "テスト"
        assert row["sector"] == "情報通信"

    def test_sets_edinet_code(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "", edinet_code="E12345")
        db_conn.commit()

        row = db_conn.execute("SELECT edinet_code FROM stocks WHERE ticker='1234'").fetchone()
        assert row["edinet_code"] == "E12345"


class TestGetAllTickers:
    def test_returns_sorted_tickers(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "5678", "B社", "", "")
        upsert_stock(db_conn, "1234", "A社", "", "")
        db_conn.commit()

        result = get_all_tickers(db_conn)

        assert result == ["1234", "5678"]


class TestGetExistingTickers:
    def test_returns_tickers_with_source(self, db_conn: sqlite3.Connection) -> None:
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 100.0, "test_source")
        upsert_financial_item(db_conn, "5678", "2024", "pl", "revenue", 200.0, "other")
        db_conn.commit()

        result = get_existing_tickers(db_conn, "test_source")

        assert result == {"1234"}


class TestGetEdinet:
    def test_returns_edinet_code(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "", edinet_code="E99")
        db_conn.commit()

        assert get_edinet_code(db_conn, "1234") == "E99"

    def test_returns_none_for_missing(self, db_conn: sqlite3.Connection) -> None:
        assert get_edinet_code(db_conn, "9999") is None

    def test_ticker_edinet_map(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "", "", "", edinet_code="E1")
        upsert_stock(db_conn, "5678", "", "", "")
        db_conn.commit()

        result = get_ticker_edinet_map(db_conn)

        assert result == {"1234": "E1"}


class TestGetStockNames:
    def test_returns_name_map(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "A社", "", "")
        upsert_stock(db_conn, "5678", "B社", "", "")
        db_conn.commit()

        result = get_stock_names(db_conn)

        assert result == {"1234": "A社", "5678": "B社"}


class TestCompanyMetadata:
    def test_upsert_metadata(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "")
        upsert_company_metadata(
            db_conn, "1234",
            securities_report_url="https://example.com/report.pdf",
        )
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["securities_report_url"] == "https://example.com/report.pdf"

    def test_upsert_metadata_preserves_existing_fields(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "情報通信", "東証プライム")
        upsert_company_metadata(
            db_conn, "1234",
            securities_report_url="https://example.com/report.pdf",
        )
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["name"] == "テスト"
        assert row["sector"] == "情報通信"


class TestGetValidationTargets:
    def test_returns_stocks_ordered_by_market_cap(self, db_conn: sqlite3.Connection) -> None:
        from stock_db.storage.prices import upsert_price, upsert_shares_outstanding

        upsert_stock(db_conn, "1111", "Alpha", "", "")
        upsert_shares_outstanding(db_conn, "1111", 100)
        upsert_company_metadata(db_conn, "1111", securities_report_url="https://a.pdf")
        upsert_price(db_conn, "1111", "2026-04-20", 100.0, 1)

        upsert_stock(db_conn, "2222", "Beta", "", "")
        upsert_shares_outstanding(db_conn, "2222", 50)
        upsert_company_metadata(db_conn, "2222", securities_report_url="https://b.pdf")
        upsert_price(db_conn, "2222", "2026-04-20", 300.0, 1)

        db_conn.commit()

        rows = get_validation_targets(db_conn, 2)
        assert len(rows) == 2
        assert rows[0]["ticker"] == "2222"  # 300*50=15000
        assert rows[1]["ticker"] == "1111"  # 100*100=10000

    def test_excludes_stocks_without_url_or_shares(self, db_conn: sqlite3.Connection) -> None:
        from stock_db.storage.prices import upsert_price, upsert_shares_outstanding

        upsert_stock(db_conn, "1111", "HasUrl", "", "")
        upsert_shares_outstanding(db_conn, "1111", 100)
        upsert_company_metadata(db_conn, "1111", securities_report_url="https://a.pdf")
        upsert_price(db_conn, "1111", "2026-04-20", 100.0, 1)

        upsert_stock(db_conn, "2222", "NoUrl", "", "")
        upsert_shares_outstanding(db_conn, "2222", 100)
        # No securities_report_url for 2222
        upsert_price(db_conn, "2222", "2026-04-20", 500.0, 1)

        upsert_stock(db_conn, "3333", "NoShares", "", "")
        upsert_company_metadata(db_conn, "3333", securities_report_url="https://c.pdf")
        # No shares_outstanding for 3333
        upsert_price(db_conn, "3333", "2026-04-20", 999.0, 1)

        db_conn.commit()

        rows = get_validation_targets(db_conn, 10)
        assert [r["ticker"] for r in rows] == ["1111"]
