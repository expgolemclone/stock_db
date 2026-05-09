"""Sync net income forecasts from japan_company_handbook into financial_items."""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import replace_financial_items_for_source

_SHIKIHO_DB_PATH = (
    STOCKS_DB_PATH.parent.parent.parent.parent
    / "japan_company_handbook"
    / "data"
    / "stock_performance.db"
)
_SOURCE = "shikiho"


def _sync(conn: sqlite3.Connection, shikiho_db_path: str) -> tuple[int, int]:
    if not Path(shikiho_db_path).exists():
        print(f"Shikiho DB not found: {shikiho_db_path}", file=sys.stderr)
        return 0, 0

    shikiho_conn = sqlite3.connect(shikiho_db_path)
    shikiho_conn.row_factory = sqlite3.Row
    try:
        rows = shikiho_conn.execute(
            """
            SELECT stock_code, period, net_income
            FROM stock_forecasts
            WHERE forecast_type = 'shikiho'
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
    for ticker, forecasts in sorted(grouped.items()):
        if len(forecasts) < 1 or forecasts[0]["net_income"] is None:
            skipped += 1
            continue

        period = forecasts[0]["period"]
        db_rows = []

        ni_current = forecasts[0]["net_income"]
        if ni_current is not None:
            db_rows.append(
                {
                    "ticker": ticker,
                    "period": period,
                    "statement": "forecast",
                    "item_name": "net_income_current",
                    "value": ni_current * 1_000_000,
                    "source": _SOURCE,
                }
            )

        if len(forecasts) >= 2 and forecasts[1]["net_income"] is not None:
            db_rows.append(
                {
                    "ticker": ticker,
                    "period": period,
                    "statement": "forecast",
                    "item_name": "net_income_next",
                    "value": forecasts[1]["net_income"] * 1_000_000,
                    "source": _SOURCE,
                }
            )

        if db_rows:
            replace_financial_items_for_source(conn, ticker, _SOURCE, db_rows)
            ok += 1

    conn.commit()
    return ok, skipped


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Sync net income forecasts from japan_company_handbook"
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
