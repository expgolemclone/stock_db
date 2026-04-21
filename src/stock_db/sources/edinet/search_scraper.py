"""Scrape EDINET search results to find securities report docIDs via browser service."""

from __future__ import annotations

import logging
import re
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.edinet.search_scraper")

_SEARCH_URL = "https://disclosure2.edinet-fsa.go.jp/EKW01Z01/wk110000"
_DOC_LINK_RE = re.compile(r"/EKW01Z01/wk110000\?pEkwCatg=01&pSsn=\d+&pDocID=([A-Za-z0-9]+)")
_TICKER_RE = re.compile(r"\((\d{4}[A-Z]?)\)")
_BLOCK_INDICATORS = ("規定外操作", "エラー画面", "Message code")
DEFAULT_INTERVAL_SECONDS = 2.0


class EdinetBlockError(RuntimeError):
    """Raised when EDINET returns a block/error page."""


def search_annual_reports(
    client: BrowserServiceClient,
    ticker: str,
    *,
    proxy: str | None = None,
) -> str | None:
    """Search EDINET for the latest annual report of a given ticker.

    Returns the docID if found, None if not found.
    Raises EdinetBlockError if EDINET blocks the request.
    """
    from bs4 import BeautifulSoup

    url = f"{_SEARCH_URL}?pKbn=01&pSsn=99&pTky={ticker}"
    resp = client.fetch(url, proxy=proxy)
    if resp.error:
        logger.warning("Search failed for %s: %s", ticker, resp.error)
        return None
    if resp.html is None:
        logger.warning("Empty HTML for %s", ticker)
        return None

    # ブロック検知: EDINETエラー画面を確認
    for indicator in _BLOCK_INDICATORS:
        if indicator in resp.html:
            raise EdinetBlockError(
                f"EDINET blocked the request for ticker {ticker}: '{indicator}' detected"
            )

    soup = BeautifulSoup(resp.html, "html.parser")

    # 検索結果から有価証券報告書のdocIDを探す
    for link in soup.find_all("a", href=True):
        href = link["href"]
        text = link.get_text(strip=True)

        # 有価証券報告書のリンクを探す
        if "有価証券報告書" in text or "pDocID=" in href:
            m = _DOC_LINK_RE.search(href)
            if m:
                doc_id = m.group(1)
                logger.info("Found docID %s for ticker %s", doc_id, ticker)
                return doc_id

    # フォールバック: テーブル行からdocIDを探す
    for row in soup.find_all("tr"):
        cells = row.find_all("td")
        if not cells:
            continue
        row_text = row.get_text()
        if ticker in row_text and "有価証券報告書" in row_text:
            for link in row.find_all("a", href=True):
                m = _DOC_LINK_RE.search(link["href"])
                if m:
                    return m.group(1)

    logger.info("No annual report found for ticker %s", ticker)
    return None


def batch_search_doc_ids(
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    proxy: str | None = None,
    interval: float = DEFAULT_INTERVAL_SECONDS,
) -> dict[str, str]:
    """Search EDINET for multiple tickers and return {ticker: docID} mapping.

    Only returns results for tickers where a docID was found.
    Raises EdinetBlockError immediately if EDINET blocks any request.
    """
    from stock_db.proxy_pool import random_delay

    results: dict[str, str] = {}
    consecutive_misses = 0
    max_consecutive_misses = 5

    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Searching %s", i, len(tickers), ticker)
        doc_id = search_annual_reports(client, ticker, proxy=proxy)
        if doc_id:
            results[ticker] = doc_id
            consecutive_misses = 0
        else:
            consecutive_misses += 1

        if i < len(tickers):
            random_delay(interval * 0.75, interval * 1.25)

    return results
