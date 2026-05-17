from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db import _edinet_xbrl
from stock_db.sources.stooq.exceptions import StooqParseError
from stock_db.storage.prices import upsert_price


def ingest_daily_prices(conn: sqlite3.Connection, file_path: Path) -> int:
    try:
        rows: list[tuple[str, str, float]] = _edinet_xbrl.parse_stooq_daily_file(str(file_path))
    except RuntimeError as exc:
        raise StooqParseError(str(exc)) from exc

    for ticker, normalized_date, close in rows:
        upsert_price(conn, ticker, normalized_date, close, None)
    return len(rows)
