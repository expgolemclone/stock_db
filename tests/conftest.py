from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


@pytest.fixture()
def db_conn(tmp_path: Path) -> sqlite3.Connection:
    db_path = tmp_path / "test_stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    yield conn
    conn.close()
