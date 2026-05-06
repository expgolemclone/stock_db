"""CLI entry point for parsing XBRL inventories into financial_items."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys

from stock_db.paths import STOCKS_DB_PATH
from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError, parse_xbrl_bs
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import replace_financial_items_for_source


_SOURCE = "xbrl_bs"
_STATEMENT = "bs"
_ITEM_NAME = "inventories"


def main() -> None:
    parser = argparse.ArgumentParser(description="Parse XBRL files and store BS data")
    parser.add_argument("--ticker", type=str, help="Single ticker to parse")
    parser.add_argument("--skip-existing", action="store_true", default=True, help="Skip tickers with existing xbrl_bs data")
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    logger = logging.getLogger("parse_xbrl_bs")

    skip_existing = args.skip_existing and not args.force

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        rows = conn.execute(
            "SELECT ticker, xbrl_path FROM sec_reports WHERE xbrl_path IS NOT NULL"
        ).fetchall()

        if args.ticker:
            rows = [r for r in rows if r["ticker"] == args.ticker]

        if not rows:
            print("No XBRL files to parse", file=sys.stderr)
            sys.exit(1)

        if skip_existing:
            existing = set(
                r[0]
                for r in conn.execute(
                    "SELECT DISTINCT ticker FROM financial_items WHERE source = ?",
                    (_SOURCE,),
                ).fetchall()
            )
            before = len(rows)
            rows = [r for r in rows if r["ticker"] not in existing]
            skipped = before - len(rows)
            if skipped:
                logger.info("Skipping %d tickers with existing xbrl_bs data", skipped)

        ok = 0
        errors = 0

        for i, row in enumerate(rows, 1):
            ticker: str = row["ticker"]
            xbrl_path: str = row["xbrl_path"]
            logger.info("[%d/%d] Parsing %s", i, len(rows), ticker)

            try:
                parsed = parse_xbrl_bs(xbrl_path)
            except InventoriesTagMismatchError as exc:
                logger.error("  %s: %s", ticker, exc)
                errors += 1
                continue

            if not parsed:
                logger.info("  %s: no detailed BS data", ticker)
                continue

            db_rows: list[dict[str, str | float | None]] = []
            for period, items in parsed.items():
                db_rows.append({
                    "ticker": ticker,
                    "period": period,
                    "statement": _STATEMENT,
                    "item_name": _ITEM_NAME,
                    "value": items.get(_ITEM_NAME),
                    "source": _SOURCE,
                })

            replace_financial_items_for_source(conn, ticker, _SOURCE, db_rows)
            conn.commit()
            logger.info("  %s: %d items across %d periods", ticker, len(db_rows), len(parsed))
            ok += 1

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        sys.exit(1 if errors > 0 else 0)
    finally:
        conn.close()


if __name__ == "__main__":
    main()
