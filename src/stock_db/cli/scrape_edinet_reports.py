"""CLI entry point for downloading and extracting EDINET securities reports."""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

import requests

from stock_db.browser_client.client import BrowserServiceClient
from stock_db.paths import STOCKS_DB_PATH, VAR_DIR, cli_defaults, magic_numbers
from stock_db.sources.edinet.api_client import build_pdf_url, doc_id_from_url, download_pdf, download_xbrl
from stock_db.sources.edinet.pdf_extractor import extract_markdown
from stock_db.sources.edinet.search_scraper import (
    DEFAULT_INTERVAL_SECONDS,
    DocIdExtractionError,
    EdinetBlockError,
    EdinetNoRecordsError,
    search_annual_reports,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import (
    get_processed_doc_ids,
    sync_edinet_raw_to_db,
    upsert_sec_report,
)
from stock_db.storage.stocks import get_all_tickers, get_edinet_code, get_stock_names, upsert_company_metadata, upsert_stock

logger = logging.getLogger("stock_db.cli.scrape_edinet_reports")

_EDINET_RAW_DIR = VAR_DIR / "raw" / "edinet"
_WORKERS = 3
_db_lock = threading.Lock()


def _process_one(conn: sqlite3.Connection, ticker: str, url: str, *, client: BrowserServiceClient | None = None, proxy: str | None = None, skip_pdf: bool = False, skip_xbrl: bool = False) -> tuple[int, int]:
    """Download PDF/XBRL, save. Returns (ok, errors)."""
    try:
        doc_id = url.rsplit("/", 1)[-1].replace(".pdf", "")
        md_path: str | None = None
        xbrl_path: str | None = None
        page_count: int | None = None
        char_count: int | None = None

        if not skip_pdf:
            pdf_path = download_pdf(url, _EDINET_RAW_DIR / "pdf" / ticker)
            markdown = extract_markdown(pdf_path)

            md_dir = _EDINET_RAW_DIR / ticker
            md_dir.mkdir(parents=True, exist_ok=True)
            md_path = str(md_dir / "latest.md")
            Path(md_path).write_text(markdown, encoding="utf-8")
            page_count = len(markdown.split("\n\n"))
            char_count = len(markdown)
            logger.info("  %s: saved %s (%d chars)", ticker, md_path, len(markdown))
        else:
            existing = conn.execute(
                "SELECT file_path, page_count, char_count FROM sec_reports WHERE doc_id = ?",
                (doc_id,),
            ).fetchone()
            if existing:
                md_path = existing["file_path"]
                page_count = existing["page_count"]
                char_count = existing["char_count"]

        if not skip_xbrl and client is not None:
            xbrl_dest = download_xbrl(client, doc_id, _EDINET_RAW_DIR / "xbrl" / ticker, proxy=proxy)
            if xbrl_dest is not None:
                xbrl_path = str(xbrl_dest)

        with _db_lock:
            upsert_sec_report(
                conn,
                ticker=ticker,
                fiscal_year="latest",
                doc_id=doc_id,
                file_path=md_path or "",
                xbrl_path=xbrl_path,
                page_count=page_count,
                char_count=char_count,
            )
            upsert_company_metadata(conn, ticker, securities_report_url=url)
            conn.commit()
        return 1, 0
    except (requests.RequestException, OSError, ValueError) as exc:
        logger.exception("  Error processing %s: %s", ticker, exc)
        return 0, 1


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
    2. 全銘柄(URLあり+発見済み): PDF/XBRLダウンロード

    Raises EdinetBlockError if EDINET blocks the search.

    Returns: (ok_count, error_count)
    """
    existing_ids = get_processed_doc_ids(conn) if skip_existing else set()
    xbrl_done_ids: set[str] = set()
    if skip_existing:
        xbrl_done_ids = {
            r["doc_id"] for r in conn.execute(
                "SELECT doc_id FROM sec_reports WHERE xbrl_path IS NOT NULL"
            ).fetchall()
        }

    # URLあり・なしを仕分け
    has_url: dict[str, str] = {}
    for row in conn.execute(
        "SELECT ticker, securities_report_url FROM stocks "
        "WHERE securities_report_url IS NOT NULL"
    ).fetchall():
        if row["ticker"] in set(tickers):
            has_url[row["ticker"]] = row["securities_report_url"]

    no_url_tickers = [t for t in tickers if t not in has_url]

    logger.info(
        "%d tickers total: %d with URL, %d without URL",
        len(tickers), len(has_url), len(no_url_tickers),
    )

    ok = 0
    errors = 0

    # Phase 1: URLなし銘柄をEDINET検索でdocID発見
    if no_url_tickers:
        logger.info("Phase 1: Discovering %d tickers via EDINET search (%d workers)", len(no_url_tickers), _WORKERS)
        names = get_stock_names(conn)
        edinet_codes = {t: get_edinet_code(conn, t) for t in no_url_tickers}

        def _search_worker(ticker: str) -> tuple[str, str | None, str | None]:
            return ticker, *search_annual_reports(
                client, ticker, proxy=proxy,
                edinet_code=edinet_codes.get(ticker),
                company_name=names.get(ticker),
            )

        with ThreadPoolExecutor(max_workers=_WORKERS) as executor:
            futures = {executor.submit(_search_worker, t): t for t in no_url_tickers}
            done_count = 0
            for future in as_completed(futures):
                done_count += 1
                ticker, doc_id, found_edinet = future.result()
                logger.info("[Phase1 %d/%d] %s: docID=%s edinet=%s",
                            done_count, len(no_url_tickers), ticker, doc_id, found_edinet)
                with _db_lock:
                    if found_edinet:
                        try:
                            upsert_stock(conn, ticker, names.get(ticker, ""), "", "", edinet_code=found_edinet)
                        except sqlite3.IntegrityError:
                            logger.warning("  EDINET code %s already assigned to another ticker, skipping", found_edinet)
                    if doc_id:
                        url = build_pdf_url(doc_id)
                        upsert_company_metadata(conn, ticker, securities_report_url=url)
                    conn.commit()
                if not doc_id:
                    logger.info("  No annual report found for %s", ticker)

    # URLあり銘柄を再取得（Phase 1で新規発見分を含む）
    has_url = {}
    for row in conn.execute(
        "SELECT ticker, securities_report_url FROM stocks "
        "WHERE securities_report_url IS NOT NULL"
    ).fetchall():
        if row["ticker"] in set(tickers):
            has_url[row["ticker"]] = row["securities_report_url"]

    # Phase 2: 全URLあり銘柄のPDF/XBRLダウンロード
    url_targets = [
        (t, url) for t, url in has_url.items()
        if doc_id_from_url(url) not in existing_ids
        or doc_id_from_url(url) not in xbrl_done_ids
    ]
    if url_targets:
        pdf_todo = sum(1 for _, u in url_targets if doc_id_from_url(u) not in existing_ids)
        xbrl_todo = sum(1 for _, u in url_targets if doc_id_from_url(u) not in xbrl_done_ids)
        logger.info("Phase 2: Processing %d tickers (PDF todo: %d, XBRL todo: %d, workers: %d)",
                     len(url_targets), pdf_todo, xbrl_todo, _WORKERS)

    def _download_worker(ticker: str, url: str) -> tuple[int, int]:
        doc_id = doc_id_from_url(url)
        skip_pdf = doc_id in existing_ids
        skip_xbrl = doc_id in xbrl_done_ids
        logger.info("[Phase2] Processing %s (skip_pdf=%s, skip_xbrl=%s)", ticker, skip_pdf, skip_xbrl)
        return _process_one(conn, ticker, url, client=client, proxy=proxy,
                            skip_pdf=skip_pdf, skip_xbrl=skip_xbrl)

    with ThreadPoolExecutor(max_workers=_WORKERS) as executor:
        futures = {executor.submit(_download_worker, t, u): (t, u) for t, u in url_targets}
        done_count = 0
        for future in as_completed(futures):
            done_count += 1
            ticker, url = futures[future]
            try:
                ok_delta, err_delta = future.result()
            except (requests.RequestException, OSError, ValueError) as exc:
                logger.exception("[Phase2 %d/%d] %s failed: %s", done_count, len(url_targets), ticker, exc)
                err_delta = 1
                ok_delta = 0
            with _db_lock:
                ok += ok_delta
                errors += err_delta

    logger.info("Total: %d ok, %d errors", ok, errors)
    return ok, errors


def main() -> None:
    defaults = cli_defaults("scrape_edinet_reports")
    browser_cfg = magic_numbers()["browser"]
    edinet_cfg = magic_numbers().get("edinet", {})
    interval = edinet_cfg.get("interval_seconds", DEFAULT_INTERVAL_SECONDS)

    parser = argparse.ArgumentParser(description="Download and extract EDINET securities reports for all listed companies")
    parser.add_argument("--ticker", type=str, help="Single ticker to process")
    parser.add_argument(
        "--proxy", type=str, default=defaults["proxy"],
        help="direct | file:<path> | <proxy-url>",
    )
    parser.add_argument(
        "--skip-existing", action="store_true", default=defaults.get("skip_existing", True),
        help="Skip already processed documents (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    skip_existing = args.skip_existing and not args.force

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

        if args.ticker:
            tickers: list[str] = [args.ticker]
        else:
            tickers = get_all_tickers(conn)

        if not tickers:
            print("No tickers to process", file=sys.stderr)
            sys.exit(1)

        client_cfg = {
            "pool_size": _WORKERS,
            "page_timeout": browser_cfg.get("page_timeout", 30000),
            "idle_timeout": browser_cfg.get("idle_timeout", 60000),
            "startup_timeout": browser_cfg.get("startup_timeout", 30),
            "headless": defaults.get("headless", True),
            "disable_xvfb": defaults.get("disable_xvfb", True),
            "challenge_poll_interval_ms": browser_cfg.get("challenge_poll_interval_ms", 500),
            "challenge_clear_stable_ms": browser_cfg.get("challenge_clear_stable_ms", 2000),
        }

        with BrowserServiceClient(config=client_cfg) as client:
            ok, errors = scrape_all_edinet_reports(
                conn, client, tickers,
                proxy=args.proxy if args.proxy != "direct" else None,
                skip_existing=skip_existing,
                interval=interval,
            )

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        sys.exit(1 if errors > 0 else 0)
    except (EdinetBlockError, DocIdExtractionError, EdinetNoRecordsError) as exc:
        logger.error("Scrape failed: %s", exc)
        print(f"FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
    finally:
        conn.close()


if __name__ == "__main__":
    main()
