"""Scrape IR BANK /bs pages using BrowserServiceClient."""

from __future__ import annotations

import logging
import re
import sqlite3
from collections import Counter
from typing import TYPE_CHECKING
from urllib.parse import urljoin

from stock_db.browser_client.client import BrowserServiceError
from stock_db.proxy_pool import random_delay
from stock_db.paths import magic_numbers
from stock_db.sources.irbank.bs_parser import parse_latest_annual_bs_page
from stock_db.storage.financials import replace_financial_items_for_source

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.irbank.bs_scraper")

_SOURCE = "irbank_bs"
_STATEMENT = "bs"
_BASE_URL = "https://irbank.net/{ticker}/bs"
_STATUS_STATEMENT = "_status"
_NO_DATA_ITEM = "no_data"
_STATUS_PERIOD = "0000-00"
_FETCH_RETRIES = 3
_RETRYABLE_HTTP_STATUSES = {408, 425, 429, 500, 502, 503, 504}
_PERIOD_LINK_RE = re.compile(r"^(\d{4})/(\d{2})$")
_NO_ANNUAL_DETAIL_LINK = "no_annual_detail_link"
_AMBIGUOUS_ANNUAL_DETAIL_LINK = "ambiguous_annual_detail_link"
_NO_ANNUAL_DETAIL_DATA = "no_annual_detail_data"


class BSPageFetchError(RuntimeError):
    def __init__(self, status_item: str, detail: str) -> None:
        super().__init__(detail)
        self.status_item = status_item
        self.detail = detail


def _find_latest_annual_detail_url(summary_html: str, requested_url: str) -> str:
    from bs4 import BeautifulSoup

    soup = BeautifulSoup(summary_html, "html.parser")
    canonical = soup.find("link", rel="canonical")
    base_url = requested_url
    if canonical is not None and canonical.get("href"):
        base_url = str(canonical["href"])

    candidates: list[tuple[str, str, str]] = []
    for anchor in soup.find_all("a", href=True):
        text = anchor.get_text(strip=True)
        href = anchor["href"]
        match = _PERIOD_LINK_RE.fullmatch(text)
        if match is None:
            continue
        if not href.endswith("/bs"):
            continue
        month = match.group(2)
        candidates.append((text, month, urljoin(base_url, href)))

    if not candidates:
        raise BSPageFetchError(_NO_ANNUAL_DETAIL_LINK, "No annual detail link found")

    month_counts = Counter(month for _, month, _ in candidates)
    max_count = max(month_counts.values())
    annual_months = sorted(month for month, count in month_counts.items() if count == max_count)
    if len(annual_months) != 1:
        months = ",".join(annual_months)
        raise BSPageFetchError(
            _AMBIGUOUS_ANNUAL_DETAIL_LINK,
            f"Unable to infer annual month from summary page: {months}",
        )

    annual_month = annual_months[0]
    annual_candidates = [(period, url) for period, month, url in candidates if month == annual_month]
    if not annual_candidates:
        raise BSPageFetchError(_NO_ANNUAL_DETAIL_LINK, "No annual detail link found")
    return max(annual_candidates, key=lambda item: item[0])[1]


def _fetch_html(
    client: BrowserServiceClient,
    url: str,
    *,
    proxy: str | None = None,
) -> str:
    for attempt in range(1, _FETCH_RETRIES + 1):
        resp = client.fetch(url, proxy=proxy)
        if resp.error:
            if attempt < _FETCH_RETRIES:
                logger.warning(
                    "Fetch failed for %s (attempt %d/%d): %s",
                    url,
                    attempt,
                    _FETCH_RETRIES,
                    resp.error,
                )
                continue
            raise BSPageFetchError(_build_status_item("fetch_error", resp.error), resp.error)

        if resp.status != 200:
            detail = f"HTTP {resp.status}"
            if resp.status in _RETRYABLE_HTTP_STATUSES and attempt < _FETCH_RETRIES:
                logger.warning(
                    "HTTP %d for %s (attempt %d/%d)",
                    resp.status,
                    url,
                    attempt,
                    _FETCH_RETRIES,
                )
                continue
            raise BSPageFetchError(_build_status_item("http_error", str(resp.status)), detail)

        if resp.html is None:
            detail = "Empty HTML response"
            if attempt < _FETCH_RETRIES:
                logger.warning(
                    "Empty HTML for %s (attempt %d/%d)",
                    url,
                    attempt,
                    _FETCH_RETRIES,
                )
                continue
            raise BSPageFetchError(_build_status_item("fetch_error", "empty_html"), detail)

        return resp.html

    raise AssertionError("unreachable")


def scrape_bs_page(
    client: BrowserServiceClient,
    ticker: str,
    *,
    proxy: str | None = None,
) -> dict[str, dict[str, float | None]]:
    """Fetch and parse a single ticker's /bs page.

    Returns: {period: {item_name: value}}
    """
    url = _BASE_URL.format(ticker=ticker)
    summary_html = _fetch_html(client, url, proxy=proxy)
    annual_detail_url = _find_latest_annual_detail_url(summary_html, url)

    detailed_html = _fetch_html(client, annual_detail_url, proxy=proxy)
    parsed = parse_latest_annual_bs_page(detailed_html)
    if not parsed:
        raise BSPageFetchError(_NO_ANNUAL_DETAIL_DATA, f"No annual detail data parsed for {ticker}")
    return parsed


def _build_status_item(kind: str, detail: str | None = None) -> str:
    if detail is None:
        return kind

    head = detail.split(" at ", 1)[0].strip().lower()
    normalized = re.sub(r"[^a-z0-9]+", "_", head).strip("_")
    if not normalized:
        return kind
    return f"{kind}.{normalized[:48]}"


def _build_status_row(ticker: str, item_name: str) -> dict[str, str | float | None]:
    return {
        "ticker": ticker,
        "period": _STATUS_PERIOD,
        "statement": _STATUS_STATEMENT,
        "item_name": item_name,
        "value": None,
        "source": _SOURCE,
    }


def _replace_ticker_rows(
    conn: sqlite3.Connection,
    ticker: str,
    rows: list[dict[str, str | float | None]],
) -> None:
    replace_financial_items_for_source(conn, ticker, _SOURCE, rows)
    conn.commit()


def _get_processed_tickers(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute(
        """
        SELECT DISTINCT ticker
        FROM financial_items
        WHERE source = ?
          AND NOT (statement = ? AND item_name != ?)
        """,
        (_SOURCE, _STATUS_STATEMENT, _NO_DATA_ITEM),
    ).fetchall()
    return {r[0] for r in rows}


def scrape_and_store(
    client: BrowserServiceClient,
    conn: sqlite3.Connection,
    tickers: list[str],
    *,
    proxy: str | None = None,
    skip_existing: bool = True,
) -> tuple[int, int]:
    """Scrape multiple tickers and store results in the DB.

    Returns: (ok_count, error_count)
    """
    cfg = magic_numbers().get("irbank_bs", {})
    interval = cfg.get("request_interval_seconds", 2.0)

    if skip_existing:
        existing = _get_processed_tickers(conn)
        before = len(tickers)
        tickers = [t for t in tickers if t not in existing]
        skipped = before - len(tickers)
        if skipped:
            logger.info("Skipping %d tickers with existing irbank_bs data", skipped)

    ok = 0
    errors = 0

    def _persist_result(
        ticker: str,
        parsed: dict[str, dict[str, float | None]],
    ) -> tuple[int, int]:
        if not parsed:
            _replace_ticker_rows(conn, ticker, [_build_status_row(ticker, _NO_DATA_ITEM)])
            logger.info("  %s: no detailed BS data; marked as no_data", ticker)
            return 1, 0

        rows = []
        for period, items in parsed.items():
            for item_name, value in items.items():
                rows.append({
                    "ticker": ticker,
                    "period": period,
                    "statement": _STATEMENT,
                    "item_name": item_name,
                    "value": value,
                    "source": _SOURCE,
                })
        _replace_ticker_rows(conn, ticker, rows)
        logger.info("  %s: %d items across %d periods", ticker, len(rows), len(parsed))
        return 1, 0

    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Scraping %s", i, len(tickers), ticker)
        try:
            parsed = scrape_bs_page(client, ticker, proxy=proxy)
            ok_delta, error_delta = _persist_result(ticker, parsed)
            ok += ok_delta
            errors += error_delta
        except BSPageFetchError as exc:
            try:
                _replace_ticker_rows(conn, ticker, [_build_status_row(ticker, exc.status_item)])
            except sqlite3.Error as db_exc:
                errors += 1
                logger.exception("Error storing fetch status for %s: %s", ticker, db_exc)
            else:
                errors += 1
                logger.warning(
                    "  %s: fetch failed after retries; marked as %s",
                    ticker,
                    exc.status_item,
                )
        except (BrowserServiceError, sqlite3.Error, ValueError) as exc:
            errors += 1
            logger.exception("Error scraping %s: %s", ticker, exc)

        if i < len(tickers):
            random_delay(interval * 0.5, interval * 1.5)

    logger.info("Done: %d ok, %d errors", ok, errors)
    return ok, errors
