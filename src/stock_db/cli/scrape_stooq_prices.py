from __future__ import annotations

import argparse
import logging
from pathlib import Path
import sys
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, STOOQ_DIR, cli_defaults
from stock_db.sources.stooq import (
    StooqDailyPriceUpdateError,
    StooqDailyPriceUpdateResult,
    update_stooq_daily_prices,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Download the latest Stooq daily JP prices and upsert them into stocks.db"
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
        result: StooqDailyPriceUpdateResult = update_stooq_daily_prices(
            db_path=args.db,
            output_dir=args.output_dir,
            headless=args.headless,
        )
    except StooqDailyPriceUpdateError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    print(
        f"Imported {result.imported} JP prices for {result.date} from {result.file_path}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
