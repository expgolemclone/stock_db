"""CLI entry point for downloading and extracting EDINET securities reports."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from datetime import date, timedelta
from pathlib import Path

from stock_db.paths import STOCKS_DB_PATH, VAR_DIR, cli_defaults, magic_numbers
from stock_db.proxy_pool import random_delay
from stock_db.sources.edinet.api_client import (
    EdinetAPIError,
    download_pdf,
    filter_annual_reports,
    list_documents,
)
from stock_db.sources.edinet.pdf_extractor import extract_markdown
from stock_db.storage.connection import get_connection
from stock_db.storage.sec_reports import get_processed_doc_ids, upsert_sec_report
from stock_db.storage.stocks import get_all_tickers

logger = logging.getLogger("stock_db.cli.scrape_edinet_reports")

_EDINET_RAW_DIR = VAR_DIR / "raw" / "edinet"


def _sec_code_to_ticker(sec_code: str) -> str | None:
    """EDINET secCode (5-digit) -> 4-digit ticker."""
    if len(sec_code) != 5:
        return None
    return sec_code[:4]


def scrape_edinet_reports(
    conn: sqlite3.Connection,
    *,
    start_date: str,
    end_date: str,
    target_tickers: set[str] | None = None,
    skip_existing: bool = True,
    interval: float = 2.0,
) -> tuple[int, int]:
    """Download and extract EDINET reports for a date range.

    Returns: (ok_count, error_count)
    """
    existing_ids = get_processed_doc_ids(conn) if skip_existing else set()

    current = date.fromisoformat(start_date)
    end = date.fromisoformat(end_date)
    ok = 0
    errors = 0

    while current <= end:
        date_str = current.isoformat()
        logger.info("Fetching document list for %s", date_str)

        try:
            all_docs = list_documents(date_str)
        except (EdinetAPIError, ConnectionError) as exc:
            logger.exception("Failed to list documents for %s: %s", date_str, exc)
            errors += 1
            current += timedelta(days=1)
            continue

        annual = filter_annual_reports(all_docs)
        logger.info("  %d annual reports found for %s", len(annual), date_str)

        for doc in annual:
            doc_id = doc["docID"]
            if doc_id in existing_ids:
                logger.debug("  Skipping existing doc %s", doc_id)
                continue

            sec_code = doc.get("secCode", "")
            ticker = _sec_code_to_ticker(sec_code)
            if ticker is None:
                logger.warning("  Skipping doc %s with invalid secCode: %s", doc_id, sec_code)
                continue

            if target_tickers is not None and ticker not in target_tickers:
                continue

            period_end = doc.get("periodEnd", "")
            fiscal_year = period_end[:4] if len(period_end) >= 4 else "unknown"

            try:
                pdf_path = download_pdf(doc_id, _EDINET_RAW_DIR / "pdf" / ticker)
                markdown = extract_markdown(pdf_path)

                md_dir = _EDINET_RAW_DIR / ticker
                md_dir.mkdir(parents=True, exist_ok=True)
                md_path = md_dir / f"{fiscal_year}.md"
                md_path.write_text(markdown, encoding="utf-8")

                upsert_sec_report(
                    conn,
                    ticker=ticker,
                    fiscal_year=fiscal_year,
                    doc_id=doc_id,
                    file_path=str(md_path),
                    page_count=markdown.count("\n\n") + 1,
                    char_count=len(markdown),
                )
                conn.commit()
                logger.info("  %s (%s): saved %s", ticker, fiscal_year, md_path)
                ok += 1
                existing_ids.add(doc_id)
            except (OSError, ValueError) as exc:
                logger.exception("  Error processing doc %s for %s: %s", doc_id, ticker, exc)
                errors += 1

            random_delay(interval * 0.5, interval * 1.5)

        current += timedelta(days=1)

    logger.info("Done: %d ok, %d errors", ok, errors)
    return ok, errors


def main() -> None:
    defaults = cli_defaults("scrape_edinet_reports")
    edinet_cfg = magic_numbers().get("edinet", {})
    interval = edinet_cfg.get("interval_seconds", 2.0)

    parser = argparse.ArgumentParser(description="Download and extract EDINET securities reports")
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
    parser.add_argument("--date", type=str, help="Specific date (YYYY-MM-DD)")
    parser.add_argument("--days", type=int, default=30, help="Number of days to look back (default: 30)")
    parser.add_argument(
        "--skip-existing", action="store_true", default=defaults.get("skip_existing", True),
        help="Skip already processed documents (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    skip_existing = args.skip_existing and not args.force

    today = date.today()
    if args.date:
        start_date = args.date
        end_date = args.date
    else:
        end_date = today.isoformat()
        start_date = (today - timedelta(days=args.days)).isoformat()

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        target_tickers: set[str] | None = None
        if args.ticker:
            target_tickers = {args.ticker}

        ok, errors = scrape_edinet_reports(
            conn,
            start_date=start_date,
            end_date=end_date,
            target_tickers=target_tickers,
            skip_existing=skip_existing,
            interval=interval,
        )

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        sys.exit(1 if errors > 0 else 0)
    finally:
        conn.close()


if __name__ == "__main__":
    main()
