"""CLI entry point for downloading EDINET securities report XBRL and scraping search results."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
import threading
import time
from collections.abc import Sequence
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Literal

import requests

from stock_db.browser_client.client import BrowserServiceClient
from stock_db.paths import STOCKS_DB_PATH, VAR_DIR, cli_defaults, magic_numbers
from stock_db.sources.edinet.api_client import build_pdf_url, doc_id_from_url, download_xbrl
from stock_db.sources.edinet.search_scraper import (
    DEFAULT_INTERVAL_SECONDS,
    DocIdExtractionError,
    EdinetBlockError,
    EdinetNoRecordsError,
    search_annual_reports,
)
from stock_db.sources.edinet.xbrl_bs_parser import is_valid_xbrl_path
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import (
    get_processed_report_keys,
    sync_edinet_raw_to_db,
    upsert_sec_report,
)
from stock_db.storage.stocks import get_all_tickers, get_edinet_code, get_stock_names, upsert_company_metadata, upsert_stock

logger = logging.getLogger("stock_db.cli.scrape_edinet_reports")

_EDINET_RAW_DIR = VAR_DIR / "raw" / "edinet"
_WORKERS = 1

_CLI_MODE_ALL = "all"
_CLI_MODE_STEP1 = "step1"
_CLI_MODE_STEP2 = "step2"
_CliMode = Literal["all", "step1", "step2"]


@dataclass(frozen=True, slots=True)
class _ExistingReportArtifacts:
    xbrl_path: str | None


@dataclass(frozen=True, slots=True)
class _DownloadedReportArtifacts:
    ticker: str
    url: str
    doc_id: str
    xbrl_path: str | None


@dataclass(frozen=True, slots=True)
class _Phase1Result:
    searched: int
    found: int
    not_found: int
    errors: int


@dataclass(frozen=True, slots=True)
class _Phase2Result:
    ok: int
    errors: int
    xbrl_failures: int
    skipped_missing_url: int


class _RequestThrottle:
    def __init__(self, interval_seconds: float) -> None:
        self._interval_seconds = max(interval_seconds, 0.0)
        self._lock = threading.Lock()
        self._next_request_at = 0.0

    def wait(self) -> None:
        if self._interval_seconds <= 0:
            return

        with self._lock:
            now = time.monotonic()
            scheduled = max(now, self._next_request_at)
            self._next_request_at = scheduled + self._interval_seconds

        delay = scheduled - now
        if delay > 0:
            time.sleep(delay)


def _load_existing_report_artifacts(
    conn: sqlite3.Connection,
    report_keys: set[tuple[str, str]],
) -> dict[tuple[str, str], _ExistingReportArtifacts]:
    if not report_keys:
        return {}

    pairs_sql = " OR ".join("(ticker = ? AND doc_id = ?)" for _ in report_keys)
    params = tuple(
        value
        for ticker, doc_id in sorted(report_keys)
        for value in (ticker, doc_id)
    )
    rows = conn.execute(
        f"""
        SELECT doc_id, xbrl_path
             , ticker
        FROM sec_reports
        WHERE {pairs_sql}
        """,
        params,
    ).fetchall()
    return {
        (row["ticker"], row["doc_id"]): _ExistingReportArtifacts(
            xbrl_path=row["xbrl_path"],
        )
        for row in rows
    }


def _load_valid_xbrl_report_keys(conn: sqlite3.Connection) -> set[tuple[str, str]]:
    rows = conn.execute(
        """
        SELECT ticker, doc_id, xbrl_path
        FROM sec_reports
        WHERE xbrl_path IS NOT NULL
        """
    ).fetchall()

    valid_keys: set[tuple[str, str]] = set()
    for row in rows:
        key = (row["ticker"], row["doc_id"])
        if is_valid_xbrl_path(row["xbrl_path"]):
            valid_keys.add(key)
            continue
        logger.info(
            "Phase 2: will refresh invalid saved XBRL for %s (doc_id=%s)",
            row["ticker"],
            row["doc_id"],
        )
    return valid_keys


def _load_securities_report_urls(
    conn: sqlite3.Connection,
    tickers: Sequence[str],
) -> dict[str, str]:
    ticker_set = set(tickers)
    if not ticker_set:
        return {}
    rows = conn.execute(
        "SELECT ticker, securities_report_url FROM stocks "
        "WHERE securities_report_url IS NOT NULL"
    ).fetchall()
    return {
        row["ticker"]: row["securities_report_url"]
        for row in rows
        if row["ticker"] in ticker_set
    }


def _build_client_config(defaults: dict, browser_cfg: dict) -> dict:
    return {
        "pool_size": _WORKERS,
        "page_timeout": browser_cfg.get("page_timeout", 30000),
        "idle_timeout": browser_cfg.get("idle_timeout", 60000),
        "startup_timeout": browser_cfg.get("startup_timeout", 30),
        "headless": defaults.get("headless", True),
        "disable_xvfb": defaults.get("disable_xvfb", True),
        "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
        "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
    }


def _resolve_tickers(
    conn: sqlite3.Connection,
    ticker: str | None,
) -> list[str]:
    if ticker:
        return [ticker]
    return get_all_tickers(conn)


def _process_one(
    ticker: str,
    url: str,
    *,
    client: BrowserServiceClient | None = None,
    skip_xbrl: bool = False,
    existing: _ExistingReportArtifacts | None = None,
    keep_existing_xbrl: bool = True,
    before_request: Callable[[], None] | None = None,
) -> _DownloadedReportArtifacts:
    """Download XBRL and return the artifacts for main-thread persistence."""
    doc_id = doc_id_from_url(url)
    if doc_id is None:
        raise ValueError(f"Cannot extract docID from URL: {url}")

    xbrl_path = existing.xbrl_path if existing is not None and keep_existing_xbrl else None

    if not skip_xbrl and client is not None:
        xbrl_dest = download_xbrl(
            client,
            doc_id,
            _EDINET_RAW_DIR / "xbrl" / ticker,
            before_request=before_request,
        )
        if xbrl_dest is not None:
            xbrl_path = str(xbrl_dest)
        else:
            logger.warning("  %s: XBRL download failed (doc_id=%s)", ticker, doc_id)

    return _DownloadedReportArtifacts(
        ticker=ticker,
        url=url,
        doc_id=doc_id,
        xbrl_path=xbrl_path,
    )


def scrape_edinet_phase1(
    conn: sqlite3.Connection,
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    interval: float = DEFAULT_INTERVAL_SECONDS,
) -> _Phase1Result:
    """Discover EDINET annual report URLs for tickers without securities_report_url."""
    has_url = _load_securities_report_urls(conn, tickers)
    no_url_tickers = [ticker for ticker in tickers if ticker not in has_url]
    if not no_url_tickers:
        logger.info("Phase 1: all %d tickers already have securities_report_url", len(tickers))
        return _Phase1Result(searched=0, found=0, not_found=0, errors=0)

    throttle = _RequestThrottle(interval)
    names = get_stock_names(conn)
    edinet_codes = {ticker: get_edinet_code(conn, ticker) for ticker in no_url_tickers}

    logger.info(
        "Phase 1: Discovering %d tickers via EDINET search (%d workers)",
        len(no_url_tickers),
        _WORKERS,
    )

    found = 0
    not_found = 0

    def _search_worker(ticker: str) -> tuple[str, str | None, str | None]:
        return ticker, *search_annual_reports(
            client,
            ticker,
            edinet_code=edinet_codes.get(ticker),
            company_name=names.get(ticker),
            before_request=throttle.wait,
        )

    with ThreadPoolExecutor(max_workers=_WORKERS) as executor:
        futures = {executor.submit(_search_worker, ticker): ticker for ticker in no_url_tickers}
        done_count = 0
        for future in as_completed(futures):
            done_count += 1
            ticker, doc_id, found_edinet = future.result()
            logger.info(
                "[Phase1 %d/%d] %s: docID=%s edinet=%s",
                done_count,
                len(no_url_tickers),
                ticker,
                doc_id,
                found_edinet,
            )
            if found_edinet:
                try:
                    upsert_stock(conn, ticker, names.get(ticker, ""), "", "", edinet_code=found_edinet)
                except sqlite3.IntegrityError:
                    logger.warning(
                        "  EDINET code %s already assigned to another ticker, skipping",
                        found_edinet,
                    )
            if doc_id:
                upsert_company_metadata(conn, ticker, securities_report_url=build_pdf_url(doc_id))
                found += 1
            else:
                not_found += 1
                logger.info("  No annual report found for %s", ticker)
            conn.commit()

    logger.info(
        "Phase 1 complete: searched=%d found=%d not_found=%d",
        len(no_url_tickers),
        found,
        not_found,
    )
    return _Phase1Result(
        searched=len(no_url_tickers),
        found=found,
        not_found=not_found,
        errors=0,
    )


def scrape_edinet_phase2(
    conn: sqlite3.Connection,
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    skip_existing: bool = True,
    interval: float = DEFAULT_INTERVAL_SECONDS,
) -> _Phase2Result:
    """Download XBRL for tickers that already have securities_report_url."""
    has_url = _load_securities_report_urls(conn, tickers)
    missing_url_tickers = [ticker for ticker in tickers if ticker not in has_url]
    if missing_url_tickers:
        logger.info(
            "Phase 2: skipping %d tickers without securities_report_url",
            len(missing_url_tickers),
        )

    existing_report_keys: set[tuple[str, str]] = set()
    xbrl_done_keys: set[tuple[str, str]] = set()
    if skip_existing:
        existing_report_keys = get_processed_report_keys(conn)
        xbrl_done_keys = _load_valid_xbrl_report_keys(conn)

    url_targets = [
        (ticker, url)
        for ticker, url in has_url.items()
        if (ticker, doc_id_from_url(url) or "") not in existing_report_keys
        or (ticker, doc_id_from_url(url) or "") not in xbrl_done_keys
    ]
    existing_artifacts = _load_existing_report_artifacts(
        conn,
        {
            (ticker, doc_id)
            for ticker, url in url_targets
            if (doc_id := doc_id_from_url(url)) is not None
        },
    )
    if not url_targets:
        logger.info(
            "Phase 2: no URL-backed tickers require processing (skip_existing=%s)",
            skip_existing,
        )
        return _Phase2Result(
            ok=0,
            errors=0,
            xbrl_failures=0,
            skipped_missing_url=len(missing_url_tickers),
        )

    xbrl_todo = sum(
        1
        for ticker, url in url_targets
        if (ticker, doc_id_from_url(url) or "") not in xbrl_done_keys
    )
    logger.info(
        "Phase 2: Processing %d tickers (XBRL todo: %d, workers: %d)",
        len(url_targets),
        xbrl_todo,
        _WORKERS,
    )

    throttle = _RequestThrottle(interval)

    def _download_worker(ticker: str, url: str) -> _DownloadedReportArtifacts:
        doc_id = doc_id_from_url(url)
        report_key = (ticker, doc_id or "")
        skip_xbrl = report_key in xbrl_done_keys
        logger.info("[Phase2] Processing %s (skip_xbrl=%s)", ticker, skip_xbrl)
        return _process_one(
            ticker,
            url,
            client=client,
            skip_xbrl=skip_xbrl,
            keep_existing_xbrl=skip_xbrl,
            existing=existing_artifacts.get((ticker, doc_id)) if doc_id is not None else None,
            before_request=throttle.wait,
        )

    ok = 0
    errors = 0
    xbrl_failures = 0
    with ThreadPoolExecutor(max_workers=_WORKERS) as executor:
        futures = {executor.submit(_download_worker, ticker, url): (ticker, url) for ticker, url in url_targets}
        done_count = 0
        for future in as_completed(futures):
            done_count += 1
            ticker, url = futures[future]
            try:
                result = future.result()
            except (requests.RequestException, OSError, ValueError) as exc:
                logger.exception("[Phase2 %d/%d] %s failed: %s", done_count, len(url_targets), ticker, exc)
                errors += 1
                continue

            upsert_sec_report(
                conn,
                ticker=result.ticker,
                fiscal_year="latest",
                doc_id=result.doc_id,
                xbrl_path=result.xbrl_path,
            )
            upsert_company_metadata(conn, result.ticker, securities_report_url=result.url)
            conn.commit()
            ok += 1
            if result.xbrl_path is None and (ticker, result.doc_id) not in xbrl_done_keys:
                xbrl_failures += 1

    logger.info(
        "Phase 2 complete: %d ok, %d errors, %d xbrl_failed, %d skipped_without_url",
        ok,
        errors,
        xbrl_failures,
        len(missing_url_tickers),
    )
    return _Phase2Result(
        ok=ok,
        errors=errors,
        xbrl_failures=xbrl_failures,
        skipped_missing_url=len(missing_url_tickers),
    )


def scrape_all_edinet_reports(
    conn: sqlite3.Connection,
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    proxy: str | None = None,
    skip_existing: bool = True,
    interval: float = DEFAULT_INTERVAL_SECONDS,
) -> tuple[int, int]:
    """全銘柄の有報を取得・抽出。

    1. URLなし銘柄: browser serviceでEDINET検索→docID発見
    2. 全銘柄(URLあり+発見済み): XBRLダウンロード

    Raises EdinetBlockError if EDINET blocks the search.

    Returns: (ok_count, error_count)
    """
    del proxy

    phase1 = scrape_edinet_phase1(conn, client, tickers, interval=interval)
    phase2 = scrape_edinet_phase2(
        conn,
        client,
        tickers,
        skip_existing=skip_existing,
        interval=interval,
    )
    logger.info(
        "Combined run complete: phase1 searched=%d found=%d not_found=%d; phase2 ok=%d errors=%d",
        phase1.searched,
        phase1.found,
        phase1.not_found,
        phase2.ok,
        phase2.errors,
    )
    return phase2.ok, phase2.errors


def build_parser(mode: _CliMode) -> argparse.ArgumentParser:
    descriptions = {
        _CLI_MODE_ALL: "Download and extract EDINET securities reports for all listed companies",
        _CLI_MODE_STEP1: "Discover EDINET securities report URLs for tickers without securities_report_url",
        _CLI_MODE_STEP2: "Download EDINET XBRL artifacts for tickers with securities_report_url",
    }
    defaults = cli_defaults("scrape_edinet_reports")
    parser = argparse.ArgumentParser(description=descriptions[mode])
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
    if mode in {_CLI_MODE_ALL, _CLI_MODE_STEP2}:
        parser.add_argument(
            "--skip-existing",
            action="store_true",
            default=defaults.get("skip_existing", True),
            help="Skip already processed documents (default)",
        )
        parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    return parser


def _print_summary(mode: _CliMode, *, phase1: _Phase1Result | None = None, phase2: _Phase2Result | None = None) -> None:
    if mode == _CLI_MODE_STEP1 and phase1 is not None:
        print(
            (
                "Done: "
                f"{phase1.searched} searched, {phase1.found} found, "
                f"{phase1.not_found} not found, {phase1.errors} errors"
            ),
            file=sys.stderr,
        )
        return

    if phase2 is not None:
        suffix = ""
        if mode == _CLI_MODE_STEP2:
            suffix = f", {phase2.skipped_missing_url} skipped without URL"
        print(f"Done: {phase2.ok} ok, {phase2.errors} errors{suffix}", file=sys.stderr)


def _run_selected_mode(
    mode: _CliMode,
    conn: sqlite3.Connection,
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    skip_existing: bool,
    interval: float,
) -> int:
    if mode == _CLI_MODE_STEP1:
        phase1 = scrape_edinet_phase1(conn, client, tickers, interval=interval)
        _print_summary(mode, phase1=phase1)
        return 1 if phase1.errors > 0 else 0

    if mode == _CLI_MODE_STEP2:
        phase2 = scrape_edinet_phase2(
            conn,
            client,
            tickers,
            skip_existing=skip_existing,
            interval=interval,
        )
        _print_summary(mode, phase2=phase2)
        return 1 if phase2.errors > 0 else 0

    ok, errors = scrape_all_edinet_reports(
        conn,
        client,
        tickers,
        skip_existing=skip_existing,
        interval=interval,
    )
    _print_summary(mode, phase2=_Phase2Result(ok=ok, errors=errors, xbrl_failures=0, skipped_missing_url=0))
    return 1 if errors > 0 else 0


def _main(argv: Sequence[str] | None, *, mode: _CliMode) -> int:
    defaults = cli_defaults("scrape_edinet_reports")
    browser_cfg = magic_numbers()["browser"]
    edinet_cfg = magic_numbers().get("edinet", {})
    interval = edinet_cfg.get("interval_seconds", DEFAULT_INTERVAL_SECONDS)

    parser = build_parser(mode)
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    skip_existing = getattr(args, "skip_existing", True) and not getattr(args, "force", False)

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    init_db(conn)
    try:
        synced_reports, synced_urls = sync_edinet_raw_to_db(conn, _EDINET_RAW_DIR)
        if synced_reports or synced_urls:
            conn.commit()
            logger.info(
                "Recovered %d sec_reports rows and %d securities_report_url values from raw EDINET files",
                synced_reports,
                synced_urls,
            )

        tickers = _resolve_tickers(conn, args.ticker)
        if not tickers:
            print("No tickers to process", file=sys.stderr)
            return 0

        client_cfg = _build_client_config(defaults, browser_cfg)
        with BrowserServiceClient(config=client_cfg) as client:
            return _run_selected_mode(
                mode,
                conn,
                client,
                tickers,
                skip_existing=skip_existing,
                interval=interval,
            )
    except (EdinetBlockError, DocIdExtractionError, EdinetNoRecordsError) as exc:
        logger.error("Scrape failed: %s", exc)
        print(f"FAILED: {exc}", file=sys.stderr)
        return 1
    finally:
        conn.close()


def main(argv: Sequence[str] | None = None) -> int:
    return _main(argv, mode=_CLI_MODE_ALL)


def main_step1(argv: Sequence[str] | None = None) -> int:
    return _main(argv, mode=_CLI_MODE_STEP1)


def main_step2(argv: Sequence[str] | None = None) -> int:
    return _main(argv, mode=_CLI_MODE_STEP2)


if __name__ == "__main__":
    raise SystemExit(main())
