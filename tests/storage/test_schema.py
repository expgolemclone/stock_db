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

        assert tables == {
            "stocks",
            "financial_items",
            "share_classes",
            "prices",
            "source_refresh_log",
            "sec_reports",
        }

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

    def test_share_classes_columns(self, db_conn: sqlite3.Connection) -> None:
        cols = _column_names(db_conn, "share_classes")

        assert {
            "ticker",
            "period",
            "source",
            "class_key",
            "class_name",
            "shares",
            "is_preferred",
            "source_kind",
            "updated_at",
        } <= cols

    def test_drops_legacy_prices_shares_outstanding_column(self, tmp_path: Path) -> None:
        db_path = tmp_path / "legacy_prices.db"
        conn = get_connection(db_path)
        conn.executescript(
            """
            CREATE TABLE prices (
                ticker TEXT NOT NULL,
                date TEXT NOT NULL,
                close REAL,
                volume INTEGER,
                shares_outstanding INTEGER,
                updated_at TEXT,
                PRIMARY KEY (ticker, date)
            );
            INSERT INTO prices (ticker, date, close, volume, shares_outstanding, updated_at)
            VALUES ('1234', '2024-01-01', 100.0, 10, NULL, '2024-01-02T00:00:00+00:00');
            """
        )
        conn.commit()

        init_db(conn)

        cols = _column_names(conn, "prices")
        row = conn.execute("SELECT ticker, date, close, volume, updated_at FROM prices").fetchone()
        assert "shares_outstanding" not in cols
        assert row["ticker"] == "1234"
        assert row["updated_at"] == "2024-01-02T00:00:00+00:00"
        conn.close()

    def test_drops_legacy_market_cap_table(self, tmp_path: Path) -> None:
        db_path = tmp_path / "legacy_market_cap.db"
        conn = get_connection(db_path)
        conn.executescript(
            """
            CREATE TABLE market_cap (
                ticker TEXT NOT NULL,
                source TEXT NOT NULL,
                value_yen INTEGER,
                fetched_at TEXT NOT NULL,
                PRIMARY KEY (ticker, source)
            );
            INSERT INTO market_cap (ticker, source, value_yen, fetched_at)
            VALUES ('1234', 'kabutan', 1000, '2024-01-01');
            """
        )
        conn.commit()

        init_db(conn)

        assert "market_cap" not in _table_names(conn)
        conn.close()

    def test_drops_legacy_address_source_urls_column(self, tmp_path: Path) -> None:
        db_path = tmp_path / "legacy_stocks.db"
        conn = get_connection(db_path)
        conn.executescript(
            """
            CREATE TABLE stocks (
                ticker TEXT PRIMARY KEY,
                edinet_code TEXT,
                name TEXT,
                sector TEXT,
                market TEXT,
                shares_outstanding INTEGER,
                shares_updated_at TEXT,
                securities_report_url TEXT,
                address_source_urls TEXT,
                updated_at TEXT
            );
            INSERT INTO stocks (
                ticker,
                edinet_code,
                name,
                sector,
                market,
                shares_outstanding,
                shares_updated_at,
                securities_report_url,
                address_source_urls,
                updated_at
            )
            VALUES (
                '1234',
                'E12345',
                'テスト',
                '情報通信',
                'Prime',
                1000,
                '2024-01-01T00:00:00+00:00',
                'https://example.test/report.pdf',
                '["https://example.test/1234/ir"]',
                '2024-01-02T00:00:00+00:00'
            );
            """
        )
        conn.commit()

        init_db(conn)

        cols = _column_names(conn, "stocks")
        row = conn.execute(
            """
            SELECT ticker, edinet_code, securities_report_url, shares_outstanding
            FROM stocks
            WHERE ticker = '1234'
            """
        ).fetchone()
        assert "address_source_urls" not in cols
        assert row["edinet_code"] == "E12345"
        assert row["securities_report_url"] == "https://example.test/report.pdf"
        assert row["shares_outstanding"] == 1000
        conn.close()
