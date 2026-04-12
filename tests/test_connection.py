from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from stock_db.db.connection import get_connection


class TestGetConnection:
    def test_returns_sqlite3_connection(self, tmp_path: Path) -> None:
        conn = get_connection(tmp_path / "test.db")

        assert isinstance(conn, sqlite3.Connection)
        conn.close()

    def test_creates_parent_directories(self, tmp_path: Path) -> None:
        db_path = tmp_path / "nested" / "dir" / "test.db"

        conn = get_connection(db_path)

        assert db_path.parent.is_dir()
        conn.close()

    def test_enables_wal_journal_mode(self, tmp_path: Path) -> None:
        conn = get_connection(tmp_path / "test.db")

        mode = conn.execute("PRAGMA journal_mode").fetchone()[0]
        assert mode == "wal"
        conn.close()

    def test_enables_foreign_keys(self, tmp_path: Path) -> None:
        conn = get_connection(tmp_path / "test.db")

        fk = conn.execute("PRAGMA foreign_keys").fetchone()[0]
        assert fk == 1
        conn.close()

    def test_row_factory_is_sqlite3_row(self, tmp_path: Path) -> None:
        conn = get_connection(tmp_path / "test.db")
        conn.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        conn.execute("INSERT INTO t VALUES (1, 'a')")

        row = conn.execute("SELECT * FROM t").fetchone()

        assert row["id"] == 1
        assert row["name"] == "a"
        conn.close()
