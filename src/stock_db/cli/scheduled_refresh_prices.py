from __future__ import annotations

import argparse
import logging
import sys
from datetime import date, datetime
from typing import Sequence
from zoneinfo import ZoneInfo

from stock_db.market_calendar import is_jpx_business_day
from stock_db.sources.price_refresh import (
    describe_price_refresh_result,
    refresh_prices,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Scheduled price refresh: runs only on JPX business days after 16:00 JST",
    )
    parser.add_argument(
        "--headless",
        action=argparse.BooleanOptionalAction,
        default=None,
        help="Override browser headless mode",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    now_jst = datetime.now(ZoneInfo("Asia/Tokyo"))
    today = now_jst.date()

    if now_jst.hour < 16:
        print("before 16:00 JST; no-op", file=sys.stderr)
        return 0

    try:
        business_day = is_jpx_business_day(today)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    if not business_day:
        print(f"{today} is not a JPX business day; no-op", file=sys.stderr)
        return 0

    try:
        result = refresh_prices(
            target_date=today,
            if_needed=True,
            headless=args.headless,
        )
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1

    print(describe_price_refresh_result(result), file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
