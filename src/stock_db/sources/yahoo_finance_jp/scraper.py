"""Scrape Yahoo Finance Japan quote pages using BrowserServiceClient."""

from __future__ import annotations

import logging
import random
import sqlite3
import time
from typing import TYPE_CHECKING

from stock_db.browser_client.client import BrowserServiceError
from stock_db.paths import magic_numbers
from stock_db.sources.yahoo_finance_jp.parser import QuoteData, is_quote_page, parse_quote_page
from stock_db.storage.prices import upsert_price
from stock_db.storage.stocks import upsert_yf_suffix

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.yahoo_finance_jp.scraper")

_QUOTE_URL = "https://finance.yahoo.co.jp/quote/{ticker}.{suffix}"
_SUFFIXES = ("T", "N", "S", "F")
_FETCH_RETRIES = 3
_RETRYABLE_HTTP_STATUSES = {408, 425, 429, 500, 502, 503, 504}


class YFScrapeError(RuntimeError):
    pass


def _quote_url(ticker: str, suffix: str) -> str:
    return _QUOTE_URL.format(ticker=ticker, suffix=suffix)


def _fetch_html(
    client: BrowserServiceClient,
    url: str,
) -> str:
    for attempt in range(1, _FETCH_RETRIES + 1):
        resp = client.fetch(url)
        if resp.error:
            if attempt < _FETCH_RETRIES:
                logger.warning(
                    "Fetch failed for %s (attempt %d/%d): %s",
                    url, attempt, _FETCH_RETRIES, resp.error,
                )
                continue
            raise YFScrapeError(f"Fetch error for {url}: {resp.error}")

        if resp.status != 200:
            if resp.status in _RETRYABLE_HTTP_STATUSES and attempt < _FETCH_RETRIES:
                logger.warning(
                    "HTTP %d for %s (attempt %d/%d)",
                    resp.status, url, attempt, _FETCH_RETRIES,
                )
                continue
            raise YFScrapeError(f"HTTP {resp.status} for {url}")

        if resp.html is None:
            if attempt < _FETCH_RETRIES:
                logger.warning(
                    "Empty HTML for %s (attempt %d/%d)",
                    url, attempt, _FETCH_RETRIES,
                )
                continue
            raise YFScrapeError(f"Empty HTML for {url}")

        return resp.html

    raise AssertionError("unreachable")


def _delay(base_interval: float) -> None:
    jittered = base_interval * random.uniform(0.5, 1.5)
    time.sleep(jittered)


def discover_suffix(
    client: BrowserServiceClient,
    ticker: str,
    *,
    interval: float = 1.0,
) -> str | None:
    """Try each exchange suffix and return the first quote page that resolves."""
    for suffix in _SUFFIXES:
        url = _quote_url(ticker, suffix)
        try:
            html = _fetch_html(client, url)
        except YFScrapeError:
            logger.debug("  %s.%s: fetch failed", ticker, suffix)
            continue
        if parse_quote_page(html) is not None:
            logger.debug("  %s.%s: found quote data", ticker, suffix)
            return suffix
        if is_quote_page(html):
            logger.debug("  %s.%s: found quote page without quote data", ticker, suffix)
            return suffix
        _delay(interval)

    logger.info("No valid suffix found for %s", ticker)
    return None


def fetch_price(
    client: BrowserServiceClient,
    ticker: str,
    suffix: str,
) -> QuoteData | None:
    """Fetch quote data for a ticker with a known suffix."""
    url = _quote_url(ticker, suffix)
    html = _fetch_html(client, url)
    return parse_quote_page(html)


def scrape_and_store(
    client: BrowserServiceClient,
    conn: sqlite3.Connection,
    tickers: list[str],
    *,
    skip_existing: bool = True,
) -> tuple[int, int]:
    """Scrape prices for tickers and store in DB.

    Returns: (ok_count, error_count)
    """
    cfg = magic_numbers().get("yahoo_finance_jp", {})
    interval = cfg.get("request_interval_seconds", 1.0)

    from stock_db.storage.stocks import get_ticker_suffix_map

    suffix_map = get_ticker_suffix_map(conn)

    if skip_existing:
        from stock_db.storage.prices import get_fresh_price_tickers

        fresh = get_fresh_price_tickers(conn, stale_days=1)
        before = len(tickers)
        tickers = [t for t in tickers if t not in fresh]
        skipped = before - len(tickers)
        if skipped:
            logger.info("Skipping %d tickers with fresh prices", skipped)

    ok = 0
    errors = 0

    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Processing %s", i, len(tickers), ticker)
        try:
            suffix = suffix_map.get(ticker)
            if suffix is None:
                suffix = discover_suffix(client, ticker, interval=interval)
                if suffix is None:
                    errors += 1
                    logger.warning("  %s: no valid suffix found", ticker)
                    continue
                upsert_yf_suffix(conn, ticker, suffix)
                conn.commit()

            quote = fetch_price(client, ticker, suffix)
            if quote is None:
                errors += 1
                logger.warning("  %s: no quote data returned", ticker)
                continue

            upsert_price(conn, ticker, quote.date or "", quote.close, quote.volume)
            conn.commit()
            logger.info(
                "  %s: close=%.0f date=%s volume=%s",
                ticker, quote.close, quote.date, quote.volume,
            )
            ok += 1
        except (YFScrapeError, BrowserServiceError, sqlite3.OperationalError) as exc:
            errors += 1
            logger.exception("Error processing %s: %s", ticker, exc)

        if i < len(tickers):
            _delay(interval)

    logger.info("Done: %d ok, %d errors", ok, errors)
    return ok, errors
