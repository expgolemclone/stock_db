"""CLI entry point for scraping Yahoo Finance Japan prices."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from typing import Sequence

from stock_db.browser_client.client import BrowserServiceClient, BrowserServiceError
from stock_db.paths import STOCKS_DB_PATH, cli_defaults, magic_numbers
from stock_db.sources.yahoo_finance_jp.scraper import (
    NON_TSE_SUFFIXES,
    YFScrapeError,
    scrape_and_store,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import get_all_tickers


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Scrape Yahoo Finance Japan for non-TSE stock prices",
    )
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
    parser.add_argument(
        "--force",
        action="store_true",
        help="Re-scrape even if price data is fresh",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    defaults = cli_defaults("scrape_yahoo_finance_prices")
    browser_cfg = magic_numbers().get("browser", {})
    parser = build_parser()
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    init_db(conn)
    try:
        tickers = [args.ticker] if args.ticker else get_all_tickers(conn)
        if not tickers:
            print("No tickers to process", file=sys.stderr)
            return 0

        client_cfg = {
            "pool_size": defaults.get("pool_size", 1),
            "page_timeout": browser_cfg.get("page_timeout", 30000),
            "idle_timeout": browser_cfg.get("idle_timeout", 300),
            "startup_timeout": browser_cfg.get("startup_timeout", 30),
            "headless": defaults.get("headless", True),
            "disable_xvfb": defaults.get("disable_xvfb", True),
            "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
            "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
        }

        with BrowserServiceClient(config=client_cfg) as client:
            ok, errors = scrape_and_store(
                client,
                conn,
                tickers,
                skip_existing=not args.force,
                allowed_suffixes=NON_TSE_SUFFIXES,
                discover_missing_suffix=False,
            )

        conn.commit()
    except (BrowserServiceError, YFScrapeError, sqlite3.OperationalError) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    finally:
        conn.close()

    print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
    return 1 if errors > 0 else 0


if __name__ == "__main__":
    raise SystemExit(main())
