"""Delete all cached IR BANK BS rows from the shared DB."""

from __future__ import annotations

import argparse
import sqlite3
import sys

from stock_db.paths import STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import purge_financial_items_for_source

_SOURCE = "irbank_bs"


def main() -> None:
    parser = argparse.ArgumentParser(description="Delete cached irbank_bs rows from stocks.db")
    parser.add_argument(
        "--all",
        action="store_true",
        help="Required safety flag. Deletes every financial_items row whose source is irbank_bs.",
    )
    args = parser.parse_args()

    if not args.all:
        print("Refusing to run without --all", file=sys.stderr)
        sys.exit(2)

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        deleted = purge_financial_items_for_source(conn, _SOURCE)
        conn.commit()
    finally:
        conn.close()

    print(f"Deleted {deleted} irbank_bs rows", file=sys.stderr)


if __name__ == "__main__":
    main()
