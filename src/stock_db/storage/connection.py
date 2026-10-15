from __future__ import annotations

import sqlite3
import time
from pathlib import Path

_CONNECT_RETRIES = 3
_CONNECT_RETRY_DELAY = 2.0


def get_connection(db_path: Path) -> sqlite3.Connection:
    db_path.parent.mkdir(parents=True, exist_ok=True)
    last_error: Exception | None = None
    for attempt in range(_CONNECT_RETRIES):
        try:
            conn = sqlite3.connect(str(db_path), timeout=60)
            mode = conn.execute("PRAGMA journal_mode").fetchone()[0]
            if mode.lower() != "wal":
                conn.execute("PRAGMA journal_mode=WAL")
            conn.execute("PRAGMA foreign_keys=ON")
            conn.row_factory = sqlite3.Row
            return conn
        except sqlite3.OperationalError as exc:
            last_error = exc
            if attempt < _CONNECT_RETRIES - 1:
                time.sleep(_CONNECT_RETRY_DELAY)
    raise last_error  # type: ignore[misc]
