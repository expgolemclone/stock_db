from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, STOOQ_DIR, cli_defaults
from stock_db.sources.price_refresh import (
    PriceRefreshError,
    describe_price_refresh_result,
    refresh_prices,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Refresh Japanese stock prices using Stooq and Yahoo Finance JP fallback",
    )
    parser.add_argument(
        "--db",
        type=Path,
        default=STOCKS_DB_PATH,
        help=f"Path to sqlite DB (default: {STOCKS_DB_PATH})",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=STOOQ_DIR,
        help=f"Directory for raw Stooq downloads (default: {STOOQ_DIR})",
    )
    parser.add_argument(
        "--headless",
        action=argparse.BooleanOptionalAction,
        default=None,
        help="Override browser headless mode",
    )
    parser.add_argument(
        "--if-needed",
        action="store_true",
        help="Skip refresh when all DB tickers already have the target price date",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    defaults = cli_defaults("scrape_stooq_prices")
    parser = build_parser()
    parser.set_defaults(
        output_dir=Path(defaults.get("output_dir", str(STOOQ_DIR))),
    )
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    try:
        result = refresh_prices(
            db_path=args.db,
            output_dir=args.output_dir,
            if_needed=args.if_needed,
            headless=args.headless,
        )
    except (PriceRefreshError, ValueError) as exc:
        print(str(exc), file=sys.stderr)
        return 1

    print(describe_price_refresh_result(result), file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
