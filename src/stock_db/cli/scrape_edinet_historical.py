"""CLI entry point for downloading historical EDINET annual report XBRL packages."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from collections.abc import Sequence
from datetime import date

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
    parser = argparse.ArgumentParser(
        description="Download historical EDINET annual report XBRL for the past N years",
    )
    parser.add_argument(
        "--from-date",
        default=defaults.get("from_date", "2016-06-01"),
        help="Start date (YYYY-MM-DD, default: 2016-06-01)",
    )
    parser.add_argument(
        "--to-date",
        default=defaults.get("to_date", date.today().isoformat()),
        help="End date (YYYY-MM-DD, default: today)",
    )
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
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

        def on_progress(current: int, total: int) -> None:
            if current % 50 == 0 or current == total:
                logger.info("[Discovery %d/%d] Scanning dates...", current, total)

        reports = discover_historical_reports(
            from_date=args.from_date,
            to_date=args.to_date,
            api_key=api_key,
            target_tickers=target_tickers,
            interval=interval,
            on_progress=on_progress,
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
        processed = 0
        for ticker in sorted(reports):
            for doc_info in reports[ticker]:
                processed += 1
                doc_id = doc_info["doc_id"]

                if skip_existing and doc_id in existing_doc_ids:
                    skipped += 1
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

        if ok > 0:
            conn.commit()
            logger.info("Committed %d new sec_reports", ok)

        print(
            f"Done: {ok} downloaded, {skipped} skipped, {errors} errors",
            file=sys.stderr,
        )
        return 1 if errors > 0 else 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
