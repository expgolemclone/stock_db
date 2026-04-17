"""Scrape IR BANK /bs pages using BrowserServiceClient."""

from __future__ import annotations

import logging
import sqlite3
from typing import TYPE_CHECKING

from stock_db.proxy_pool import random_delay
from stock_db.paths import magic_numbers
from stock_db.sources.irbank.bs_parser import parse_bs_page
from stock_db.storage.financials import upsert_financial_items_bulk
from stock_db.storage.stocks import get_existing_tickers

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.irbank.bs_scraper")

_SOURCE = "irbank_bs"
_STATEMENT = "bs"
_BASE_URL = "https://irbank.net/{ticker}/bs"


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
    resp = client.fetch(url, proxy=proxy)
    if resp.error or resp.html is None:
        logger.warning("Fetch failed for %s: %s", ticker, resp.error)
        return {}
    if resp.status != 200:
        logger.warning("HTTP %d for %s", resp.status, ticker)
        return {}
    return parse_bs_page(resp.html)


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
        existing = get_existing_tickers(conn, _SOURCE)
        before = len(tickers)
        tickers = [t for t in tickers if t not in existing]
        skipped = before - len(tickers)
        if skipped:
            logger.info("Skipping %d tickers with existing irbank_bs data", skipped)

    ok = 0
    errors = 0
    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Scraping %s", i, len(tickers), ticker)
        try:
            parsed = scrape_bs_page(client, ticker, proxy=proxy)
            if not parsed:
                errors += 1
                continue
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
            if rows:
                upsert_financial_items_bulk(conn, rows)
                conn.commit()
            ok += 1
            logger.info("  %s: %d items across %d periods", ticker, len(rows), len(parsed))
        except Exception:
            errors += 1
            logger.exception("Error scraping %s", ticker)

        if i < len(tickers):
            random_delay(interval * 0.5, interval * 1.5)

    logger.info("Done: %d ok, %d errors", ok, errors)
    return ok, errors
