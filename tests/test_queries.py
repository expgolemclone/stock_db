from __future__ import annotations

import sqlite3

from stock_db.db.queries import (
    get_all_tickers,
    get_cached_periods,
    get_edinet_code,
    get_existing_tickers,
    get_financial_dict,
    get_fresh_price_tickers,
    get_historical_items,
    get_latest_price,
    get_latest_price_with_shares,
    get_market_cap,
    get_stock_names,
    get_ticker_edinet_map,
    get_tickers_with_shares,
    is_price_stale,
    upsert_company_metadata,
    upsert_financial_item,
    upsert_financial_items_bulk,
    upsert_market_cap,
    upsert_price,
    upsert_shares_outstanding,
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
        upsert_financial_item(db_conn, "1234", "2024", "pl", "revenue", 100.0, "irbank")
        upsert_financial_item(db_conn, "5678", "2024", "pl", "revenue", 200.0, "other")
        db_conn.commit()

        result = get_existing_tickers(db_conn, "irbank")

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


class TestPrices:
    def test_upsert_and_get_latest(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2024-01-01", 100.0, 1000)
        upsert_price(db_conn, "1234", "2024-01-02", 110.0, 2000)
        db_conn.commit()

        result = get_latest_price(db_conn, "1234")

        assert result == 110.0

    def test_returns_none_for_missing(self, db_conn: sqlite3.Connection) -> None:
        assert get_latest_price(db_conn, "9999") is None


class TestShares:
    def test_upsert_shares(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "")
        upsert_shares_outstanding(db_conn, "1234", 1_000_000)
        db_conn.commit()

        result = get_tickers_with_shares(db_conn)
        assert "1234" in result

    def test_upsert_shares_creates_bare_row(self, db_conn: sqlite3.Connection) -> None:
        upsert_shares_outstanding(db_conn, "9999", 500_000)
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='9999'").fetchone()
        assert row["shares_outstanding"] == 500_000
        assert row["name"] == ""

    def test_get_latest_price_with_shares(self, db_conn: sqlite3.Connection) -> None:
        upsert_shares_outstanding(db_conn, "1234", 1_000_000)
        upsert_price(db_conn, "1234", "2024-01-01", 500.0, 100)
        db_conn.commit()

        result = get_latest_price_with_shares(db_conn, "1234")

        assert result["price"] == 500.0
        assert result["shares_outstanding"] == 1_000_000


class TestIsPriceStale:
    def test_none_is_stale(self) -> None:
        assert is_price_stale(None, stale_days=1) is True

    def test_recent_is_not_stale(self) -> None:
        from datetime import datetime, timezone
        now = datetime.now(timezone.utc).isoformat()

        assert is_price_stale(now, stale_days=1) is False


class TestGetFreshPriceTickers:
    def test_returns_fresh_tickers(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2024-01-01", 100.0, 1000)
        db_conn.commit()

        result = get_fresh_price_tickers(db_conn, stale_days=99999)

        assert "1234" in result


class TestMarketCap:
    def test_upsert_and_get(self, db_conn: sqlite3.Connection) -> None:
        upsert_market_cap(db_conn, "1234", "kabutan", 10_000_000_000, "2024-04-01")
        db_conn.commit()

        result = get_market_cap(db_conn, "1234")

        assert result["value_yen"] == 10_000_000_000
        assert result["source"] == "kabutan"

    def test_returns_none_for_missing(self, db_conn: sqlite3.Connection) -> None:
        assert get_market_cap(db_conn, "9999") is None

    def test_latest_source_wins(self, db_conn: sqlite3.Connection) -> None:
        upsert_market_cap(db_conn, "1234", "irbank", 5_000_000_000, "2024-03-01")
        upsert_market_cap(db_conn, "1234", "kabutan", 6_000_000_000, "2024-04-01")
        db_conn.commit()

        result = get_market_cap(db_conn, "1234")

        assert result["value_yen"] == 6_000_000_000
        assert result["source"] == "kabutan"


class TestCompanyMetadata:
    def test_upsert_metadata(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "")
        upsert_company_metadata(
            db_conn, "1234",
            securities_report_url="https://example.com/report.pdf",
            address_source_urls='["https://irbank.net/1234/ir"]',
        )
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='1234'").fetchone()
        assert row["securities_report_url"] == "https://example.com/report.pdf"
        assert row["address_source_urls"] == '["https://irbank.net/1234/ir"]'

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
