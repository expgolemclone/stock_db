from __future__ import annotations

import sqlite3

from stock_db.storage.financials import get_financial_dict, upsert_financial_item
from stock_db.storage.stocks import upsert_stock


def test_canonical_financial_items_use_explicit_sources(
    db_conn: sqlite3.Connection,
) -> None:
    upsert_stock(db_conn, "1234", "Example", "", "")
    for statement, item_name, value, source in [
        ("pl", "net_income", 100.0, "edinet_xbrl"),
        ("pl", "net_income", 999.0, "other"),
        ("pl", "eps", 10.0, "edinet_xbrl"),
        ("pl", "eps", 12.0, "computed"),
        ("forecast", "net_income_current", 200.0, "edinet_xbrl"),
        ("forecast", "net_income_current", 300.0, "shikiho"),
        ("forecast", "eps_current", 20.0, "computed"),
    ]:
        upsert_financial_item(
            db_conn,
            "1234",
            "2026-03",
            statement,
            item_name,
            value,
            source,
        )

    financials = get_financial_dict(db_conn, "1234")

    assert financials["pl"] == {"net_income": 100.0, "eps": 12.0}
    assert financials["forecast"] == {
        "net_income_current": 300.0,
        "eps_current": 20.0,
    }
