from __future__ import annotations

import sqlite3
import subprocess
import threading
from dataclasses import dataclass
from datetime import date
from pathlib import Path

from stock_db.browser_client.client import BrowserServiceClient, BrowserServiceError
from stock_db.paths import PROJECT_ROOT, STOCKS_DB_PATH, STOOQ_DIR, cli_defaults, magic_numbers
from stock_db.sources.stooq.downloader import DownloadedStooqDailyFile, download_latest_daily_file
from stock_db.sources.stooq.exceptions import (
    StooqCaptchaError,
    StooqDownloadError,
    StooqParseError,
)
from stock_db.sources.stooq.parser import ingest_daily_prices
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import (
    is_stooq_price_update_required,
    record_stooq_price_update_check,
)
from stock_db.storage.schema import init_db

_AUTO_UPDATE_LOCK = threading.Lock()


class StooqDailyPriceUpdateError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class StooqDailyPriceUpdateResult:
    imported: int
    date: str
    label: str
    file_path: Path


@dataclass(frozen=True, slots=True)
class StooqPriceUpdateCommandResult:
    stdout: str
    stderr: str


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
        "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 250),
        "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 1000),
    }


def _to_result(downloaded: DownloadedStooqDailyFile, imported: int) -> StooqDailyPriceUpdateResult:
    return StooqDailyPriceUpdateResult(
        imported=imported,
        date=downloaded.date,
        label=downloaded.label,
        file_path=downloaded.file_path,
    )


def _is_cwd_inside_project(cwd: Path | None = None) -> bool:
    current = (cwd or Path.cwd()).resolve()
    project_root = PROJECT_ROOT.resolve()
    return current == project_root or current.is_relative_to(project_root)


def _connection_db_path(conn: sqlite3.Connection) -> Path | None:
    rows = conn.execute("PRAGMA database_list").fetchall()
    for row in rows:
        name = row["name"] if isinstance(row, sqlite3.Row) else row[1]
        if name != "main":
            continue
        filename = row["file"] if isinstance(row, sqlite3.Row) else row[2]
        if not filename:
            return None
        return Path(filename)
    return None


def _is_update_required_for_path(
    db_path: Path,
    *,
    today: date | None = None,
) -> bool:
    conn = get_connection(db_path)
    try:
        init_db(conn)
        return is_stooq_price_update_required(conn, today=today)
    finally:
        conn.close()


def ensure_stooq_prices_fresh_for_api(
    conn: sqlite3.Connection | None = None,
    *,
    db_path: Path | None = None,
    today: date | None = None,
    cwd: Path | None = None,
) -> StooqPriceUpdateCommandResult | None:
    if _is_cwd_inside_project(cwd):
        return None

    resolved_db_path = db_path
    if resolved_db_path is None and conn is not None:
        resolved_db_path = _connection_db_path(conn)
    if resolved_db_path is None:
        return None

    if conn is not None and not is_stooq_price_update_required(conn, today=today):
        return None
    if conn is None and not _is_update_required_for_path(resolved_db_path, today=today):
        return None

    with _AUTO_UPDATE_LOCK:
        if not _is_update_required_for_path(resolved_db_path, today=today):
            return None
        return run_stooq_price_update_command(
            db_path=resolved_db_path,
            if_needed=True,
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
            record_stooq_price_update_check(conn)

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


def run_stooq_price_update_command(
    *,
    cwd: Path = PROJECT_ROOT,
    db_path: Path | None = None,
    output_dir: Path | None = None,
    if_needed: bool = False,
    timeout: int = 300,
) -> StooqPriceUpdateCommandResult:
    command = ["uv", "run", "scrape-stooq-prices"]
    if if_needed:
        command.append("--if-needed")
    if db_path is not None:
        command.extend(["--db", str(db_path)])
    if output_dir is not None:
        command.extend(["--output-dir", str(output_dir)])

    try:
        proc = subprocess.run(
            command,
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError) as exc:
        raise StooqDailyPriceUpdateError(
            f"stooq price update command failed: {type(exc).__name__}: {exc}"
        ) from exc

    if proc.returncode != 0:
        message = (proc.stderr or proc.stdout or "").strip()
        raise StooqDailyPriceUpdateError(
            f"stooq price update command failed (exit={proc.returncode}): {message}"
        )

    return StooqPriceUpdateCommandResult(
        stdout=proc.stdout or "",
        stderr=proc.stderr or "",
    )
