from __future__ import annotations

from pathlib import Path

import stock_db.api as api
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.prices import upsert_price, upsert_shares_outstanding
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import upsert_company_metadata, upsert_stock


def _seed_screening_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    try:
        init_db(conn)
        upsert_stock(conn, "1111", "Alpha", "", "")
        upsert_shares_outstanding(conn, "1111", 100)
        upsert_company_metadata(conn, "1111", securities_report_url="https://example.test/a.pdf")
        upsert_price(conn, "1111", "2026-05-20", 100.0, 1)
        upsert_financial_item(conn, "1111", "2025-03", "pl", "net_income", 1000.0, "edinet_xbrl")
        upsert_financial_item(conn, "1111", "2025-03", "bs", "total_assets", 9000.0, "edinet_xbrl")
        upsert_financial_item(conn, "1111", "2025-03", "cf", "free_cf", 500.0, "edinet_xbrl")
        upsert_financial_item(
            conn,
            "1111",
            "2025-03",
            "cf",
            "treasury_stock_purchase",
            -200.0,
            "edinet_xbrl",
        )
        conn.commit()
    finally:
        conn.close()


def test_load_screening_stocks_returns_public_contract(
    tmp_path: Path,
    monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _seed_screening_db(db_path)
    monkeypatch.setattr(api, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(api, "ensure_prices_fresh", lambda: None)

    rows = api.load_screening_stocks(["1111"], fcf_periods=1, pl_periods=1)

    assert rows == [
        {
            "ticker": "1111",
            "name": "Alpha",
            "price": 100.0,
            "price_date": "2026-05-20",
            "shares_outstanding": 100,
            "financials": {
                "pl": {"net_income": 1000.0},
                "bs": {"total_assets": 9000.0},
                "cf": {"free_cf": 500.0, "treasury_stock_purchase": -200.0},
            },
            "cf_history": [("2025-03", {"free_cf": 500.0, "treasury_stock_purchase": -200.0})],
            "pl_history": [("2025-03", {"net_income": 1000.0})],
        }
    ]


def test_get_screening_tickers_filters_to_data_ready_stocks(
    tmp_path: Path,
    monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _seed_screening_db(db_path)
    conn = get_connection(db_path)
    try:
        init_db(conn)
        upsert_stock(conn, "2222", "No financials", "", "")
        upsert_price(conn, "2222", "2026-05-20", 200.0, 1)
        conn.commit()
    finally:
        conn.close()
    monkeypatch.setattr(api, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(api, "ensure_prices_fresh", lambda: None)

    assert api.get_screening_tickers(limit=10) == ["1111"]


def test_validation_and_balance_sheet_api_hide_internal_rows(
    tmp_path: Path,
    monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _seed_screening_db(db_path)
    monkeypatch.setattr(api, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(api, "ensure_prices_fresh", lambda: None)

    assert api.get_validation_targets(1) == [
        {
            "ticker": "1111",
            "name": "Alpha",
            "securities_report_url": "https://example.test/a.pdf",
            "price": 100.0,
            "shares_outstanding": 100,
        }
    ]
    assert api.get_latest_balance_sheet("1111") == (
        "2025-03",
        {"total_assets": 9000.0},
        None,
    )
