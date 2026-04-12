from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db.db.connection import get_connection
from stock_db.db.schema import init_db


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

        assert tables == {"stocks", "financial_items", "prices", "market_cap"}

    def test_stocks_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "stocks")

        assert cols == {
            "ticker", "edinet_code", "name", "sector", "market",
            "shares_outstanding", "shares_updated_at",
            "securities_report_url", "address_source_urls", "updated_at",
        }

    def test_financial_items_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "financial_items")

        assert cols == {
            "ticker", "period", "statement", "item_name",
            "value", "source", "updated_at",
        }

    def test_prices_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "prices")

        assert cols == {"ticker", "date", "close", "volume", "updated_at"}

    def test_market_cap_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "market_cap")

        assert cols == {"ticker", "source", "value_yen", "fetched_at"}

    def test_idempotent(self, db_conn: sqlite3.Connection) -> None:
        init_db(db_conn)
        init_db(db_conn)

        tables = _table_names(db_conn)
        assert "stocks" in tables


class TestMigration:
    def test_adds_missing_columns_to_legacy_stocks(self, tmp_path: Path) -> None:
        db_path = tmp_path / "legacy.db"
        conn = get_connection(db_path)
        conn.execute(
            "CREATE TABLE stocks ("
            "  ticker TEXT PRIMARY KEY, edinet_code TEXT, name TEXT,"
            "  sector TEXT, market TEXT, updated_at TEXT"
            ")"
        )
        conn.commit()

        init_db(conn)

        cols = _column_names(conn, "stocks")
        assert "shares_outstanding" in cols
        assert "securities_report_url" in cols
        assert "address_source_urls" in cols
        conn.close()

    def test_adds_updated_at_to_legacy_prices(self, tmp_path: Path) -> None:
        db_path = tmp_path / "legacy.db"
        conn = get_connection(db_path)
        conn.execute(
            "CREATE TABLE prices ("
            "  ticker TEXT NOT NULL, date TEXT NOT NULL, close REAL, volume INTEGER,"
            "  PRIMARY KEY (ticker, date)"
            ")"
        )
        conn.commit()

        init_db(conn)

        cols = _column_names(conn, "prices")
        assert "updated_at" in cols
        conn.close()
