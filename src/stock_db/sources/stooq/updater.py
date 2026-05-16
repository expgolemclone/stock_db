from __future__ import annotations

import sqlite3
from dataclasses import dataclass
from pathlib import Path

from stock_db.browser_client.client import BrowserServiceClient, BrowserServiceError
from stock_db.paths import STOCKS_DB_PATH, STOOQ_DIR, cli_defaults, magic_numbers
from stock_db.sources.stooq.downloader import DownloadedStooqDailyFile, download_latest_daily_file
from stock_db.sources.stooq.exceptions import (
    StooqCaptchaError,
    StooqDownloadError,
    StooqParseError,
)
from stock_db.sources.stooq.parser import ingest_daily_prices
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


class StooqDailyPriceUpdateError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class StooqDailyPriceUpdateResult:
    imported: int
    date: str
    label: str
    file_path: Path


def build_stooq_browser_config(*, headless: bool | None = None) -> dict[str, object]:
    defaults = cli_defaults("scrape_stooq_prices")
    browser_cfg = magic_numbers().get("browser", {})
    return {
        "pool_size": defaults.get("pool_size", 1),
        "page_timeout": browser_cfg.get("page_timeout", 30000),
        "idle_timeout": browser_cfg.get("idle_timeout", 300),
        "startup_timeout": browser_cfg.get("startup_timeout", 30),
        "headless": defaults.get("headless", False) if headless is None else headless,
        "disable_xvfb": defaults.get("disable_xvfb", True),
        "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
        "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
    }


def _to_result(downloaded: DownloadedStooqDailyFile, imported: int) -> StooqDailyPriceUpdateResult:
    return StooqDailyPriceUpdateResult(
        imported=imported,
        date=downloaded.date,
        label=downloaded.label,
        file_path=downloaded.file_path,
    )


def update_stooq_daily_prices(
    *,
    db_path: Path = STOCKS_DB_PATH,
    output_dir: Path = STOOQ_DIR,
    headless: bool | None = None,
) -> StooqDailyPriceUpdateResult:
    client_cfg = build_stooq_browser_config(headless=headless)
    conn: sqlite3.Connection = get_connection(db_path)
    init_db(conn)
    try:
        with BrowserServiceClient(config=client_cfg) as client:
            downloaded = download_latest_daily_file(
                client,
                output_dir,
                timeout=client_cfg["page_timeout"],
            )
            imported = ingest_daily_prices(conn, downloaded.file_path)

        conn.commit()
        return _to_result(downloaded, imported)
    except (
        BrowserServiceError,
        OSError,
        StooqCaptchaError,
        StooqDownloadError,
        StooqParseError,
        ValueError,
    ) as exc:
        conn.rollback()
        raise StooqDailyPriceUpdateError(str(exc)) from exc
    finally:
        conn.close()
