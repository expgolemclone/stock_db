from __future__ import annotations

import argparse
import logging
from pathlib import Path
import sqlite3
import sys
from typing import Sequence

from stock_db.browser_client.client import BrowserServiceClient, BrowserServiceError
from stock_db.paths import STOCKS_DB_PATH, STOOQ_DIR, cli_defaults, magic_numbers
from stock_db.sources.stooq import (
    DownloadedStooqDailyFile,
    StooqCaptchaError,
    StooqDownloadError,
    StooqParseError,
    download_latest_daily_file,
    ingest_daily_prices,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


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
    browser_cfg = magic_numbers().get("browser", {})
    parser = build_parser()
    parser.set_defaults(
        output_dir=Path(defaults.get("output_dir", str(STOOQ_DIR))),
    )
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    conn: sqlite3.Connection = get_connection(args.db)
    init_db(conn)
    try:
        client_cfg = {
            "pool_size": defaults.get("pool_size", 1),
            "page_timeout": browser_cfg.get("page_timeout", 30000),
            "idle_timeout": browser_cfg.get("idle_timeout", 300),
            "startup_timeout": browser_cfg.get("startup_timeout", 30),
            "headless": defaults.get("headless", False) if args.headless is None else args.headless,
            "disable_xvfb": defaults.get("disable_xvfb", True),
            "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
            "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
        }

        with BrowserServiceClient(config=client_cfg) as client:
            downloaded = download_latest_daily_file(
                client,
                args.output_dir,
                timeout=client_cfg["page_timeout"],
            )
            imported = ingest_daily_prices(conn, downloaded.file_path)

        conn.commit()
    except (
        BrowserServiceError,
        OSError,
        StooqCaptchaError,
        StooqDownloadError,
        StooqParseError,
        ValueError,
    ) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    finally:
        conn.close()

    print(
        f"Imported {imported} JP prices for {downloaded.date} from {downloaded.file_path}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
