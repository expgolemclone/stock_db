"""CLI entry point for downloading historical EDINET annual report XBRL packages."""

from __future__ import annotations

import argparse
import json
import logging
import sqlite3
import sys
import time
from collections.abc import Sequence
from datetime import UTC, date, datetime
from pathlib import Path

from stock_db.paths import STOCKS_DB_PATH, VAR_DIR, cli_defaults, magic_numbers
from stock_db.sources.edinet.api_client import (
    EdinetApiError,
    download_xbrl_package,
    require_edinet_api_key,
)
from stock_db.sources.edinet.document_list import discover_historical_reports
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report

logger = logging.getLogger("stock_db.cli.scrape_edinet_historical")

_EDINET_RAW_DIR = VAR_DIR / "raw" / "edinet"
_DISCOVERY_SCHEMA_VERSION = 1


class _RequestThrottle:
    def __init__(self, interval_seconds: float) -> None:
        self._interval_seconds = max(interval_seconds, 0.0)
        self._next_request_at = 0.0

    def wait(self) -> None:
        if self._interval_seconds <= 0:
            return

        now = time.monotonic()
        scheduled = max(now, self._next_request_at)
        self._next_request_at = scheduled + self._interval_seconds
        delay = scheduled - now
        if delay > 0:
            time.sleep(delay)


def _date_years_ago(value: date, years: int) -> date:
    try:
        return value.replace(year=value.year - years)
    except ValueError:
        return value.replace(year=value.year - years, day=28)


def _looks_like_xbrl_artifact(path: Path) -> bool:
    for suffix in ("*.xhtml", "*.html", "*.htm", "*.xbrl"):
        if next(path.rglob(suffix), None) is not None:
            return True
    return False


def _find_existing_xbrl_artifact(ticker: str, doc_id: str) -> Path | None:
    ticker_dir = _EDINET_RAW_DIR / "xbrl" / ticker
    extract_dir = ticker_dir / doc_id
    zip_path = ticker_dir / f"{doc_id}.zip"
    if (
        extract_dir.is_dir()
        and zip_path.is_file()
        and _looks_like_xbrl_artifact(extract_dir)
    ):
        return extract_dir.resolve()
    return None


def _default_discovery_json_path(from_date: str, to_date: str, ticker: str | None) -> Path:
    label = ticker or "all"
    return _EDINET_RAW_DIR / "discovery" / f"historical_{from_date}_{to_date}_{label}.json"


def _matched_reports(reports: dict[str, list[dict]]) -> dict[str, list[dict]]:
    return {ticker: docs for ticker, docs in sorted(reports.items()) if docs}


def _write_discovery_checkpoint(
    path: Path,
    *,
    from_date: str,
    to_date: str,
    target_tickers: set[str],
    completed_dates: set[str],
    reports: dict[str, list[dict]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": _DISCOVERY_SCHEMA_VERSION,
        "from_date": from_date,
        "to_date": to_date,
        "target_ticker_count": len(target_tickers),
        "completed_dates": sorted(completed_dates),
        "last_scanned_date": max(completed_dates) if completed_dates else None,
        "report_count": sum(len(docs) for docs in reports.values()),
        "reports": _matched_reports(reports),
        "updated_at": datetime.now(UTC).isoformat(),
    }
    temp_path = path.with_name(f"{path.name}.tmp")
    temp_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    temp_path.replace(path)


def _load_discovery_checkpoint(
    path: Path,
    *,
    from_date: str,
    to_date: str,
    target_tickers: set[str],
) -> tuple[dict[str, list[dict]], set[str]]:
    if not path.is_file():
        return {}, set()

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        logger.warning("Ignoring unreadable discovery checkpoint %s: %s", path, exc)
        return {}, set()
    if payload.get("schema_version") != _DISCOVERY_SCHEMA_VERSION:
        logger.warning("Ignoring discovery checkpoint with unsupported schema: %s", path)
        return {}, set()
    if payload.get("from_date") != from_date or payload.get("to_date") != to_date:
        logger.warning("Ignoring discovery checkpoint for a different date range: %s", path)
        return {}, set()

    reports: dict[str, list[dict]] = {}
    raw_reports = payload.get("reports") or {}
    for ticker, docs in raw_reports.items():
        if ticker not in target_tickers or not isinstance(docs, list):
            continue
        reports[ticker] = [
            doc for doc in docs if isinstance(doc, dict) and doc.get("doc_id")
        ]

    completed_dates = {
        str(value)
        for value in payload.get("completed_dates", [])
        if isinstance(value, str)
    }
    return reports, completed_dates


def _resolve_numeric_tickers(conn: sqlite3.Connection, ticker: str | None) -> set[str]:
    if ticker:
        return {ticker}
    rows = conn.execute("SELECT ticker FROM stocks").fetchall()
    return {r["ticker"] for r in rows if r["ticker"].isdigit()}


def _load_existing_doc_ids(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute("SELECT doc_id FROM sec_reports").fetchall()
    return {r["doc_id"] for r in rows}


def build_parser() -> argparse.ArgumentParser:
    defaults = cli_defaults("scrape_edinet_historical")
    default_years = int(defaults.get("years", 10))
    parser = argparse.ArgumentParser(
        description="Download historical EDINET annual report XBRL for the past N years",
    )
    parser.add_argument(
        "--from-date",
        default=defaults.get("from_date"),
        help=f"Start date (YYYY-MM-DD, default: --to-date minus {default_years} years)",
    )
    parser.add_argument(
        "--to-date",
        default=defaults.get("to_date", date.today().isoformat()),
        help="End date (YYYY-MM-DD, default: today)",
    )
    parser.add_argument(
        "--years",
        type=int,
        default=default_years,
        help=f"Look back this many years when --from-date is omitted (default: {default_years})",
    )
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
    parser.add_argument(
        "--discovery-json",
        type=Path,
        help="Discovery checkpoint JSON path (default: var/raw/edinet/discovery/historical_FROM_TO_SCOPE.json)",
    )
    parser.add_argument(
        "--no-resume-discovery",
        action="store_true",
        help="Do not reuse an existing discovery checkpoint JSON",
    )
    parser.add_argument(
        "--no-discovery-json",
        action="store_true",
        help="Disable discovery checkpoint JSON output",
    )
    parser.add_argument(
        "--skip-existing",
        action="store_true",
        default=defaults.get("skip_existing", True),
        help="Skip already downloaded docIDs (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    skip_existing = args.skip_existing and not args.force
    api_key = require_edinet_api_key()

    edinet_cfg = magic_numbers().get("edinet_historical", {})
    interval = edinet_cfg.get("interval_seconds", 0.5)
    to_date = date.fromisoformat(args.to_date)
    from_date = args.from_date or _date_years_ago(to_date, args.years).isoformat()
    discovery_json = args.discovery_json
    if discovery_json is None and not args.no_discovery_json:
        discovery_json = _default_discovery_json_path(from_date, args.to_date, args.ticker)

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    init_db(conn)
    try:
        target_tickers = _resolve_numeric_tickers(conn, args.ticker)
        if not target_tickers:
            print("No numeric tickers to process", file=sys.stderr)
            return 0

        existing_doc_ids = _load_existing_doc_ids(conn) if skip_existing else set()
        if skip_existing and existing_doc_ids:
            logger.info("Skipping %d already-downloaded docIDs", len(existing_doc_ids))

        initial_reports: dict[str, list[dict]] = {}
        completed_dates: set[str] = set()
        if discovery_json is not None and not args.no_resume_discovery:
            initial_reports, completed_dates = _load_discovery_checkpoint(
                discovery_json,
                from_date=from_date,
                to_date=args.to_date,
                target_tickers=target_tickers,
            )
            if completed_dates:
                logger.info(
                    "Loaded discovery checkpoint %s (%d completed dates, %d reports)",
                    discovery_json,
                    len(completed_dates),
                    sum(len(docs) for docs in initial_reports.values()),
                )

        def on_progress(current: int, total: int) -> None:
            if current % 50 == 0 or current == total:
                logger.info("[Discovery %d/%d] Scanning dates...", current, total)

        discovery_reports: dict[str, list[dict]] = {
            ticker: list(initial_reports.get(ticker, []))
            for ticker in target_tickers
        }

        def on_day_scanned(date_str: str, matches: list[tuple[str, dict]], total_annual: int) -> None:
            completed_dates.add(date_str)
            for ticker, doc_info in matches:
                discovery_reports.setdefault(ticker, []).append(doc_info)
            if discovery_json is not None:
                _write_discovery_checkpoint(
                    discovery_json,
                    from_date=from_date,
                    to_date=args.to_date,
                    target_tickers=target_tickers,
                    completed_dates=completed_dates,
                    reports=discovery_reports,
                )
            if total_annual and matches:
                logger.info(
                    "[Discovery %s] matched %d/%d annual reports",
                    date_str,
                    len(matches),
                    total_annual,
                )

        reports = discover_historical_reports(
            from_date=from_date,
            to_date=args.to_date,
            api_key=api_key,
            target_tickers=target_tickers,
            interval=interval,
            on_progress=on_progress,
            initial_reports=initial_reports,
            skip_dates=completed_dates,
            on_day_scanned=on_day_scanned,
        )

        if not reports:
            print("No new annual reports found", file=sys.stderr)
            return 0

        total_docs = sum(len(docs) for docs in reports.values())
        logger.info(
            "Found %d annual reports across %d tickers", total_docs, len(reports),
        )

        ok = 0
        errors = 0
        skipped = 0
        synced_existing = 0
        processed = 0
        throttle = _RequestThrottle(interval)
        for ticker in sorted(reports):
            for doc_info in reports[ticker]:
                processed += 1
                doc_id = doc_info["doc_id"]

                if skip_existing and doc_id in existing_doc_ids:
                    skipped += 1
                    continue

                existing_xbrl = _find_existing_xbrl_artifact(ticker, doc_id)
                if skip_existing and existing_xbrl is not None:
                    upsert_sec_report(
                        conn,
                        ticker=ticker,
                        fiscal_year=doc_info["fiscal_year"],
                        doc_id=doc_id,
                        xbrl_path=str(existing_xbrl),
                    )
                    synced_existing += 1
                    continue

                logger.info(
                    "[%d/%d] %s: downloading %s (FY=%s)",
                    processed,
                    total_docs,
                    ticker,
                    doc_id,
                    doc_info["fiscal_year"],
                )

                try:
                    xbrl_dest = download_xbrl_package(
                        doc_id,
                        _EDINET_RAW_DIR / "xbrl" / ticker,
                        api_key=api_key,
                        before_request=throttle.wait,
                    )
                    xbrl_path = str(xbrl_dest)
                except EdinetApiError as exc:
                    logger.warning("  XBRL download failed for %s (%s): %s", ticker, doc_id, exc)
                    errors += 1
                    continue

                upsert_sec_report(
                    conn,
                    ticker=ticker,
                    fiscal_year=doc_info["fiscal_year"],
                    doc_id=doc_id,
                    xbrl_path=xbrl_path,
                )
                ok += 1

        if ok > 0 or synced_existing > 0:
            conn.commit()
            logger.info(
                "Committed %d downloaded and %d existing sec_reports",
                ok,
                synced_existing,
            )

        print(
            f"Done: {ok} downloaded, {synced_existing} synced existing, "
            f"{skipped} skipped, {errors} errors",
            file=sys.stderr,
        )
        return 1 if errors > 0 else 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
