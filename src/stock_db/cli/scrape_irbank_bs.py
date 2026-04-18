"""CLI entry point for scraping IR BANK /bs pages."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from pathlib import Path

from stock_db.paths import STOCKS_DB_PATH, cli_defaults, magic_numbers
from stock_db.proxy_pool import ProxyPool
from stock_db.sources.irbank.bs_scraper import scrape_and_store
from stock_db.storage.connection import get_connection
from stock_db.storage.stocks import get_all_tickers


def _build_pool(proxy_arg: str) -> ProxyPool:
    if proxy_arg == "direct":
        return ProxyPool.make_direct()
    if proxy_arg.startswith("file:"):
        return ProxyPool.from_file(Path(proxy_arg.removeprefix("file:")))
    return ProxyPool.from_url(proxy_arg)


def main() -> None:
    defaults = cli_defaults("scrape_irbank_bs")
    browser_cfg = magic_numbers()["browser"]

    parser = argparse.ArgumentParser(description="Scrape IR BANK /bs pages for detailed BS items")
    parser.add_argument("--ticker", type=str, help="Single ticker to scrape")
    parser.add_argument(
        "--proxy", type=str, default=defaults["proxy"],
        help="direct | file:<path> | <proxy-url>",
    )
    parser.add_argument(
        "--skip-existing", action="store_true", default=defaults["skip_existing"],
        help="Skip tickers with existing irbank_bs data (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    skip_existing = args.skip_existing and not args.force

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        if args.ticker:
            tickers: list[str] = [args.ticker]
        else:
            tickers = get_all_tickers(conn)

        if not tickers:
            print("No tickers to scrape", file=sys.stderr)
            sys.exit(1)

        pool: ProxyPool = _build_pool(args.proxy)
        proxy_url: str | None = pool.get()

        client_cfg = {
            "pool_size": defaults.get("pool_size", 1),
            "page_timeout": browser_cfg.get("page_timeout", 30000),
            "idle_timeout": browser_cfg.get("idle_timeout", 60000),
            "startup_timeout": browser_cfg.get("startup_timeout", 30),
            "headless": defaults.get("headless", False),
            "disable_xvfb": defaults.get("disable_xvfb", True),
            "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
            "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
        }

        from stock_db.browser_client.client import BrowserServiceClient

        with BrowserServiceClient(config=client_cfg) as client:
            ok: int
            errors: int
            ok, errors = scrape_and_store(
                client, conn, tickers,
                proxy=proxy_url,
                skip_existing=skip_existing,
            )

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        sys.exit(1 if errors > 0 else 0)
    finally:
        conn.close()


if __name__ == "__main__":
    main()
