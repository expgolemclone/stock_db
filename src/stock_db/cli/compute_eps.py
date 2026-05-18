"""Compute and store EPS from net income and shares outstanding."""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_items_bulk
from stock_db.storage.schema import init_db

_SOURCE = "computed"


def _compute(conn: sqlite3.Connection) -> tuple[int, int]:
    # --- Historical EPS: pl.net_income / bs.shares_outstanding ---
    historical_rows = conn.execute(
        """
        SELECT ni.ticker, ni.period, ni.value AS net_income, sh.value AS shares
        FROM financial_items ni
        JOIN financial_items sh
          ON ni.ticker = sh.ticker
         AND ni.period = sh.period
         AND sh.statement = 'bs'
         AND sh.item_name = 'shares_outstanding'
        WHERE ni.statement = 'pl'
          AND ni.item_name = 'net_income'
          AND ni.value IS NOT NULL
          AND sh.value IS NOT NULL
          AND sh.value > 0
        """
    ).fetchall()

    db_rows: list[dict] = []
    tickers_historical: set[str] = set()
    for row in historical_rows:
        eps = row["net_income"] / row["shares"]
        db_rows.append(
            {
                "ticker": row["ticker"],
                "period": row["period"],
                "statement": "pl",
                "item_name": "eps",
                "value": eps,
                "source": _SOURCE,
            }
        )
        tickers_historical.add(row["ticker"])

    # --- Forecast EPS: forecast.net_income_current/next / stocks.shares_outstanding ---
    forecast_rows = conn.execute(
        """
        SELECT fi.ticker, fi.period, fi.item_name, fi.value AS net_income,
               s.shares_outstanding AS shares
        FROM financial_items fi
        JOIN stocks s ON fi.ticker = s.ticker
        WHERE fi.statement = 'forecast'
          AND fi.item_name IN ('net_income_current', 'net_income_next')
          AND fi.value IS NOT NULL
          AND s.shares_outstanding IS NOT NULL
          AND s.shares_outstanding > 0
        """
    ).fetchall()

    tickers_forecast: set[str] = set()
    for row in forecast_rows:
        eps = row["net_income"] / row["shares"]
        eps_name = row["item_name"].replace("net_income_", "eps_")
        db_rows.append(
            {
                "ticker": row["ticker"],
                "period": row["period"],
                "statement": "forecast",
                "item_name": eps_name,
                "value": eps,
                "source": _SOURCE,
            }
        )
        tickers_forecast.add(row["ticker"])

    if db_rows:
        # Delete existing computed EPS before re-inserting
        conn.execute(
            "DELETE FROM financial_items WHERE source = ? AND item_name IN ('eps', 'eps_current', 'eps_next')",
            (_SOURCE,),
        )
        upsert_financial_items_bulk(conn, db_rows)
        conn.commit()

    return len(tickers_historical | tickers_forecast), len(db_rows)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Compute EPS from net income and shares")
    parser.add_argument(
        "--db",
        default=str(STOCKS_DB_PATH),
        help="Path to stocks.db",
    )
    args = parser.parse_args(argv)

    conn: sqlite3.Connection = get_connection(Path(args.db))
    try:
        init_db(conn)
        tickers, items = _compute(conn)
        print(f"Computed EPS for {tickers} tickers ({items} items)", file=sys.stderr)
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
