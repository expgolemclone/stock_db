from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from collections import defaultdict
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, cli_defaults
from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError
from stock_db.sources.edinet.xbrl_financials_parser import parse_xbrl_financials
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import replace_financial_items_for_ticker_sources

_SOURCE = "edinet_xbrl"
_REPLACED_SOURCES = (
    "edinet_xbrl",
    "irbank",
    "irbank_bs",
    "irbank_forecast",
    "xbrl_bs",
)


def main(argv: Sequence[str] | None = None) -> int:
    defaults = cli_defaults("parse_xbrl_financials")
    parser = argparse.ArgumentParser(description="Parse EDINET XBRL files and store financial data")
    parser.add_argument("--ticker", type=str, help="Single ticker to parse")
    parser.add_argument(
        "--skip-existing",
        action="store_true",
        default=defaults.get("skip_existing", True),
        help="Skip tickers with existing edinet_xbrl data (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    logger = logging.getLogger("parse_xbrl_financials")
    skip_existing = args.skip_existing and not args.force

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        rows = conn.execute(
            """
            SELECT ticker, fiscal_year, doc_id, xbrl_path, updated_at
            FROM sec_reports
            WHERE xbrl_path IS NOT NULL
            ORDER BY ticker, fiscal_year ASC, updated_at ASC, doc_id ASC
            """
        ).fetchall()

        grouped: dict[str, list[sqlite3.Row]] = defaultdict(list)
        for row in rows:
            grouped[str(row["ticker"])].append(row)

        tickers = sorted(grouped)
        if args.ticker:
            tickers = [ticker for ticker in tickers if ticker == args.ticker]

        if not tickers:
            print("No XBRL files to parse", file=sys.stderr)
            return 1

        if skip_existing:
            existing = {
                row[0]
                for row in conn.execute(
                    "SELECT DISTINCT ticker FROM financial_items WHERE source = ?",
                    (_SOURCE,),
                ).fetchall()
            }
            before = len(tickers)
            tickers = [ticker for ticker in tickers if ticker not in existing]
            skipped = before - len(tickers)
            if skipped:
                logger.info("Skipping %d tickers with existing %s data", skipped, _SOURCE)

        ok = 0
        errors = 0
        for i, ticker in enumerate(tickers, 1):
            logger.info("[%d/%d] Parsing %s", i, len(tickers), ticker)
            merged: dict[str, dict[str, dict[str, float | None]]] = {}

            try:
                for row in grouped[ticker]:
                    parsed = parse_xbrl_financials(str(row["xbrl_path"]))
                    for period, statements in parsed.items():
                        period_bucket = merged.setdefault(period, {})
                        for statement, items in statements.items():
                            statement_bucket = period_bucket.setdefault(statement, {})
                            statement_bucket.update(items)
            except InventoriesTagMismatchError as exc:
                logger.error("  %s: %s", ticker, exc)
                errors += 1
                continue

            if not merged:
                logger.info("  %s: no parseable financial facts", ticker)
                continue

            db_rows: list[dict[str, str | float | None]] = []
            for period, statements in sorted(merged.items(), reverse=True):
                for statement, items in sorted(statements.items()):
                    for item_name, value in sorted(items.items()):
                        db_rows.append(
                            {
                                "ticker": ticker,
                                "period": period,
                                "statement": statement,
                                "item_name": item_name,
                                "value": value,
                                "source": _SOURCE,
                            }
                        )

            replace_financial_items_for_ticker_sources(
                conn,
                ticker=ticker,
                sources=_REPLACED_SOURCES,
                rows=db_rows,
            )
            conn.commit()
            logger.info("  %s: %d items across %d periods", ticker, len(db_rows), len(merged))
            ok += 1

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        return 1 if errors > 0 else 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
