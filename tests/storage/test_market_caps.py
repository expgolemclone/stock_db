from __future__ import annotations

import sqlite3

from stock_db.storage.market_caps import (
    get_market_cap,
    upsert_market_cap,
)


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
