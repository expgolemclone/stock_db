"""Sync dividend data from japan_company_handbook into financial_items."""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_items_bulk

_SHIKIHO_DB_PATH = (
    STOCKS_DB_PATH.parent.parent.parent.parent
    / "japan_company_handbook"
    / "data"
    / "stock_performance.db"
)
_SOURCE = "shikiho"
_STATEMENT = "dividend"


def _sync(conn: sqlite3.Connection, shikiho_db_path: str) -> tuple[int, int]:
    if not Path(shikiho_db_path).exists():
        print(f"Shikiho DB not found: {shikiho_db_path}", file=sys.stderr)
        return 0, 0

    shikiho_conn = sqlite3.connect(shikiho_db_path)
    shikiho_conn.row_factory = sqlite3.Row
    try:
        rows = shikiho_conn.execute(
            """
            SELECT stock_code, period, dividend
            FROM stock_dividends
            WHERE dividend IS NOT NULL
            ORDER BY stock_code, period ASC
            """
        ).fetchall()
    finally:
        shikiho_conn.close()

    grouped: dict[str, list[sqlite3.Row]] = {}
    for row in rows:
        grouped.setdefault(row["stock_code"], []).append(row)

    ok = 0
    skipped = 0
    for ticker, dividends in sorted(grouped.items()):
        if not dividends:
            skipped += 1
            continue

        db_rows = [
            {
                "ticker": ticker,
                "period": div["period"],
                "statement": _STATEMENT,
                "item_name": "dps",
                "value": div["dividend"],
                "source": _SOURCE,
            }
            for div in dividends
        ]

        conn.execute(
            "DELETE FROM financial_items WHERE ticker = ? AND source = ? AND statement = ?",
            (ticker, _SOURCE, _STATEMENT),
        )
        upsert_financial_items_bulk(conn, db_rows)
        ok += 1

    conn.commit()
    return ok, skipped


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Sync dividend data from japan_company_handbook"
    )
    parser.add_argument(
        "--shikiho-db",
        default=str(_SHIKIHO_DB_PATH),
        help="Path to stock_performance.db",
    )
    parser.add_argument(
        "--db",
        default=str(STOCKS_DB_PATH),
        help="Path to stocks.db",
    )
    args = parser.parse_args(argv)

    conn: sqlite3.Connection = get_connection(Path(args.db))
    try:
        ok, skipped = _sync(conn, args.shikiho_db)
        print(f"Synced {ok} tickers ({skipped} skipped)", file=sys.stderr)
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
