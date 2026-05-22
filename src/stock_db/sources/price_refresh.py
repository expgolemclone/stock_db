from __future__ import annotations

import sqlite3
import subprocess
import threading
from dataclasses import dataclass
from datetime import date
from pathlib import Path

from stock_db.browser_client.client import BrowserServiceClient, BrowserServiceError
from stock_db.paths import PROJECT_ROOT, STOCKS_DB_PATH, STOOQ_DIR, cli_defaults, magic_numbers
from stock_db.sources.stooq.updater import (
    StooqDailyPriceUpdateError,
    StooqDailyPriceUpdateResult,
    update_stooq_daily_prices,
)
from stock_db.sources.yahoo_finance_jp.scraper import (
    NON_TSE_SUFFIXES,
    YFScrapeError,
    scrape_and_store,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import (
    get_latest_price_date,
    get_previous_jpx_business_day,
    get_stale_price_tickers,
    has_recent_price_refresh_check,
    is_stooq_price_update_required,
    record_price_refresh_check,
)
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import get_ticker_suffix_map

_AUTO_UPDATE_LOCK = threading.Lock()


class PriceRefreshError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class PriceRefreshResult:
    target_date: date
    stale_before: int
    stale_after_stooq: int
    stale_after_yahoo: int
    unresolved_tickers: tuple[str, ...]
    stooq_result: StooqDailyPriceUpdateResult | None
    yahoo_ok: int
    yahoo_errors: int
    yahoo_skipped_reason: str | None = None
    yahoo_skipped_tickers: int = 0


@dataclass(frozen=True, slots=True)
class PriceRefreshCommandResult:
    stdout: str
    stderr: str


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


def _yahoo_browser_config(*, headless: bool | None = None) -> dict[str, object]:
    defaults = cli_defaults("scrape_yahoo_finance_prices")
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


def _is_refresh_required_for_path(
    db_path: Path,
    *,
    today: date | None = None,
) -> bool:
    target_date = get_previous_jpx_business_day(today=today)
    conn = get_connection(db_path)
    try:
        init_db(conn)
        stale_tickers = get_stale_price_tickers(conn, target_date=target_date)
        return bool(stale_tickers) and not has_recent_price_refresh_check(conn)
    finally:
        conn.close()


def _format_ticker_sample(tickers: list[str], *, limit: int = 20) -> str:
    sample = ", ".join(tickers[:limit])
    if len(tickers) > limit:
        return f"{sample}, ..."
    return sample


def _get_non_tse_yahoo_fallback_tickers(
    conn: sqlite3.Connection,
    tickers: list[str],
) -> list[str]:
    allowed_suffixes = set(NON_TSE_SUFFIXES)
    suffix_map = get_ticker_suffix_map(conn)
    return [
        ticker
        for ticker in tickers
        if (suffix_map.get(ticker) or "").upper() in allowed_suffixes
    ]


def refresh_prices(
    *,
    db_path: Path = STOCKS_DB_PATH,
    output_dir: Path = STOOQ_DIR,
    if_needed: bool = False,
    headless: bool | None = None,
    today: date | None = None,
) -> PriceRefreshResult | None:
    target_date = get_previous_jpx_business_day(today=today)
    conn = get_connection(db_path)
    try:
        init_db(conn)
        stale_before = get_stale_price_tickers(conn, target_date=target_date)
        if if_needed and not stale_before:
            return None
        if if_needed and has_recent_price_refresh_check(conn):
            return None
        stooq_required = is_stooq_price_update_required(conn, today=target_date)
    finally:
        conn.close()

    stooq_result: StooqDailyPriceUpdateResult | None = None
    if stooq_required:
        try:
            stooq_result = update_stooq_daily_prices(
                db_path=db_path,
                output_dir=output_dir,
                headless=headless,
            )
        except StooqDailyPriceUpdateError as exc:
            raise PriceRefreshError(f"Stooq price refresh failed: {exc}") from exc

    conn = get_connection(db_path)
    try:
        init_db(conn)
        stale_after_stooq = get_stale_price_tickers(conn, target_date=target_date)
        stale_after_yahoo: list[str] = []
        yahoo_ok = 0
        yahoo_errors = 0
        yahoo_skipped_reason: str | None = None
        yahoo_skipped_tickers = 0
        record_refresh_check = True

        if stale_after_stooq:
            latest_price_date = get_latest_price_date(conn)
            if latest_price_date is None or latest_price_date < target_date:
                record_refresh_check = False
                latest = (
                    latest_price_date.isoformat()
                    if latest_price_date is not None
                    else "none"
                )
                yahoo_skipped_reason = (
                    "stooq_latest_date="
                    f"{latest} is older than target_date={target_date.isoformat()}"
                )
                stale_after_yahoo = stale_after_stooq
            else:
                yahoo_tickers = _get_non_tse_yahoo_fallback_tickers(
                    conn,
                    stale_after_stooq,
                )
                yahoo_skipped_tickers = len(stale_after_stooq) - len(yahoo_tickers)
                if not yahoo_tickers:
                    yahoo_skipped_reason = "no non-TSE Yahoo fallback tickers"
                    stale_after_yahoo = stale_after_stooq
                else:
                    try:
                        with BrowserServiceClient(
                            config=_yahoo_browser_config(headless=headless),
                        ) as client:
                            yahoo_ok, yahoo_errors = scrape_and_store(
                                client,
                                conn,
                                yahoo_tickers,
                                skip_existing=False,
                                min_date=target_date.isoformat(),
                                fail_fast=False,
                                allowed_suffixes=NON_TSE_SUFFIXES,
                                discover_missing_suffix=False,
                            )
                    except (
                        BrowserServiceError,
                        YFScrapeError,
                        sqlite3.OperationalError,
                    ) as exc:
                        raise PriceRefreshError(
                            f"Yahoo Finance JP price refresh failed: {exc}",
                        ) from exc

                    stale_after_yahoo = get_stale_price_tickers(
                        conn,
                        target_date=target_date,
                    )

        if record_refresh_check:
            record_price_refresh_check(conn)
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()

    return PriceRefreshResult(
        target_date=target_date,
        stale_before=len(stale_before),
        stale_after_stooq=len(stale_after_stooq),
        stale_after_yahoo=len(stale_after_yahoo),
        unresolved_tickers=tuple(stale_after_yahoo),
        stooq_result=stooq_result,
        yahoo_ok=yahoo_ok,
        yahoo_errors=yahoo_errors,
        yahoo_skipped_reason=yahoo_skipped_reason,
        yahoo_skipped_tickers=yahoo_skipped_tickers,
    )


def describe_price_refresh_result(result: PriceRefreshResult | None) -> str:
    if result is None:
        return "Stock prices are fresh or were refreshed recently; no update needed"

    parts = [f"target_date={result.target_date.isoformat()}"]
    if result.stooq_result is not None:
        parts.append(
            "stooq="
            f"{result.stooq_result.imported} prices for {result.stooq_result.date}"
        )
    if result.stale_after_stooq:
        parts.append(f"yahoo={result.yahoo_ok} ok")
    if result.yahoo_errors:
        parts.append(f"yahoo_errors={result.yahoo_errors}")
    if result.yahoo_skipped_reason:
        parts.append(f"yahoo_skipped={result.yahoo_skipped_reason}")
    if result.yahoo_skipped_tickers:
        parts.append(f"yahoo_skipped_tickers={result.yahoo_skipped_tickers}")
    if result.unresolved_tickers:
        parts.append(
            "unresolved_stale="
            f"{result.stale_after_yahoo} ({_format_ticker_sample(list(result.unresolved_tickers))})"
        )
    if len(parts) == 1:
        parts.append("no changes")
    return "Refreshed stock prices: " + ", ".join(parts)


def run_price_refresh_command(
    *,
    cwd: Path = PROJECT_ROOT,
    db_path: Path | None = None,
    output_dir: Path | None = None,
    if_needed: bool = False,
    headless: bool | None = None,
    timeout: int = 7200,
    stream: bool = False,
) -> PriceRefreshCommandResult:
    command = ["uv", "run", "refresh-prices"]
    if if_needed:
        command.append("--if-needed")
    if db_path is not None:
        command.extend(["--db", str(db_path)])
    if output_dir is not None:
        command.extend(["--output-dir", str(output_dir)])
    if headless is True:
        command.append("--headless")
    elif headless is False:
        command.append("--no-headless")

    try:
        if stream:
            proc = subprocess.run(
                command,
                cwd=str(cwd),
                timeout=timeout,
            )
        else:
            proc = subprocess.run(
                command,
                cwd=str(cwd),
                capture_output=True,
                text=True,
                timeout=timeout,
            )
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError) as exc:
        raise PriceRefreshError(
            f"stock price refresh command failed: {type(exc).__name__}: {exc}"
        ) from exc

    if proc.returncode != 0:
        if stream:
            raise PriceRefreshError(
                f"stock price refresh command failed (exit={proc.returncode}); "
                "see refresh-prices output above"
            )
        message = (proc.stderr or proc.stdout or "").strip()
        raise PriceRefreshError(
            f"stock price refresh command failed (exit={proc.returncode}): {message}"
        )

    if stream:
        return PriceRefreshCommandResult(stdout="", stderr="")

    return PriceRefreshCommandResult(
        stdout=proc.stdout or "",
        stderr=proc.stderr or "",
    )


def ensure_prices_fresh_for_api(
    conn: sqlite3.Connection | None = None,
    *,
    db_path: Path | None = None,
    today: date | None = None,
    cwd: Path | None = None,
) -> PriceRefreshCommandResult | None:
    if _is_cwd_inside_project(cwd):
        return None

    resolved_db_path = db_path
    if resolved_db_path is None and conn is not None:
        resolved_db_path = _connection_db_path(conn)
    if resolved_db_path is None:
        return None

    if not _is_refresh_required_for_path(resolved_db_path, today=today):
        return None

    with _AUTO_UPDATE_LOCK:
        if not _is_refresh_required_for_path(resolved_db_path, today=today):
            return None
        return run_price_refresh_command(
            db_path=resolved_db_path,
            if_needed=True,
            headless=True,
            stream=True,
        )
