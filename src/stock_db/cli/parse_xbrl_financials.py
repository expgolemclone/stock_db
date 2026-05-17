from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path
from typing import Sequence

from stock_db._edinet_xbrl import parse_xbrl_financials_to_db as _rust_parse_xbrl_financials_to_db
from stock_db.paths import STOCKS_DB_PATH, cli_defaults


def parse_xbrl_financials_to_db(
    db_path: Path,
    *,
    ticker: str | None,
    from_ticker: str | None,
    skip_existing: bool,
) -> dict:
    """Parse EDINET XBRL artifacts and replace DB rows via the Rust core."""
    return _rust_parse_xbrl_financials_to_db(
        str(db_path),
        ticker=ticker,
        from_ticker=from_ticker,
        skip_existing=skip_existing,
    )


def main(argv: Sequence[str] | None = None) -> int:
    defaults = cli_defaults("parse_xbrl_financials")
    parser = argparse.ArgumentParser(description="Parse EDINET XBRL files and store financial data")
    parser.add_argument("--ticker", type=str, help="Single ticker to parse")
    parser.add_argument(
        "--from-ticker",
        type=str,
        help="Resume from this ticker in sorted ticker order (inclusive)",
    )
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

    summary = parse_xbrl_financials_to_db(
        STOCKS_DB_PATH,
        ticker=args.ticker,
        from_ticker=args.from_ticker,
        skip_existing=skip_existing,
    )

    if summary["no_xbrl_files"]:
        print("No XBRL files to parse", file=sys.stderr)
        return 1

    if summary["skipped"]:
        logger.info(
            "Skipping %d tickers with existing edinet_xbrl data",
            summary["skipped"],
        )

    results = summary["results"]
    total = len(results)
    for i, result in enumerate(results, 1):
        ticker = result["ticker"]
        status = result["status"]
        if status == "ok":
            logger.info(
                "[%d/%d] %s: %d items across %d periods, %d share classes",
                i,
                total,
                ticker,
                result["financial_rows"],
                result["period_count"],
                result["share_class_rows"],
            )
        elif status == "no_facts":
            logger.info(
                "[%d/%d] %s: no parseable financial or share-class facts",
                i,
                total,
                ticker,
            )
        else:
            logger.error("[%d/%d] %s: %s", i, total, ticker, result["message"])

    print(f"Done: {summary['ok']} ok, {summary['errors']} errors", file=sys.stderr)
    return 1 if summary["errors"] > 0 else 0


if __name__ == "__main__":
    raise SystemExit(main())
