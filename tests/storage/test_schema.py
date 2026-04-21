from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


def _table_names(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    ).fetchall()
    return {r[0] for r in rows}


def _column_names(conn: sqlite3.Connection, table: str) -> set[str]:
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()
    return {r[1] for r in rows}


class TestInitDb:
    def test_creates_all_tables(self, db_conn: sqlite3.Connection) -> None:
        tables = _table_names(db_conn)

        assert tables == {"stocks", "financial_items", "prices", "market_cap", "sec_reports"}

    def test_stocks_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "stocks")

        assert "ticker" in cols
        assert "edinet_code" in cols
        assert "shares_outstanding" in cols

    def test_idempotent(self, tmp_path: Path) -> None:
        db_path = tmp_path / "test.db"
        conn = get_connection(db_path)

        init_db(conn)
        init_db(conn)

        tables = _table_names(conn)
        assert "stocks" in tables
        conn.close()

    def test_prices_has_updated_at(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "prices")

        assert "updated_at" in cols

    def test_market_cap_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "market_cap")

        assert "value_yen" in cols
        assert "fetched_at" in cols
