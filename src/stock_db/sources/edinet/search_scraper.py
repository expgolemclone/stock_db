"""Scrape EDINET search results to find securities report docIDs via browser service."""

from __future__ import annotations

import base64
import logging
import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.edinet.search_scraper")

_SEARCH_BASE_URL = "https://disclosure2.edinet-fsa.go.jp/weee0050.aspx"
_BLOCK_INDICATORS = ("規定外操作", "エラー画面", "Message code")
DEFAULT_INTERVAL_SECONDS = 2.0
_DOCID_EXTRACTION_TIMEOUT_MS = 60000

_EXTRACT_DOC_ID_JS = """
(async () => {
  let capturedUrl = null;
  const originalOpen = window.open;
  window.open = (url) => { capturedUrl = url; return null; };

  const links = document.querySelectorAll('a[onclick]');
  for (const link of links) {
    const text = link.textContent.trim();
    if (text.includes('有価証券報告書')) {
      link.click();
      break;
    }
  }

  await new Promise(r => setTimeout(r, 5000));
  window.open = originalOpen;
  return capturedUrl;
})()
"""


class EdinetBlockError(RuntimeError):
    """Raised when EDINET returns a block/error page."""


class DocIdExtractionError(RuntimeError):
    """Raised when annual report link exists but docID extraction fails."""


def _build_search_url(ticker: str) -> str:
    params = f"scc={ticker}&pfs=6&kbn=2&p=1"
    encoded = base64.b64encode(params.encode()).decode()
    return f"{_SEARCH_BASE_URL}?{encoded}"


def _extract_doc_id_from_url(url: str | None) -> str | None:
    """docIDをWZEK0040.aspxのURLから抽出。例: WZEK0040.aspx?S100VWVY,, → S100VWVY"""
    if not url:
        return None
    match = re.match(r"^[^?]*\?([A-Za-z0-9]+)", url)
    return match.group(1) if match else None


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
    url = _build_search_url(ticker)

    # まずHTMLを取得してブロック検知
    resp = client.fetch(url, proxy=proxy)
    if resp.error:
        logger.warning("Search failed for %s: %s", ticker, resp.error)
        return None
    if resp.html is None:
        logger.warning("Empty HTML for %s", ticker)
        return None

    for indicator in _BLOCK_INDICATORS:
        if indicator in resp.html:
            raise EdinetBlockError(
                f"EDINET blocked the request for ticker {ticker}: '{indicator}' detected"
            )

    # 検索結果なし
    if "レコードがありません" in resp.html:
        logger.info("No records found for ticker %s", ticker)
        return None

    # 有価証券報告書のリンクがなければ終了
    import re
    if not re.search(r'TeisyutuSyorui_Click[^>]*>[^<]*有価証券報告書', resp.html):
        logger.info("No annual report link found for ticker %s", ticker)
        return None

    # evaluate でクリック→docID抽出
    try:
        result_url = client.evaluate(url, _EXTRACT_DOC_ID_JS, proxy=proxy, timeout=_DOCID_EXTRACTION_TIMEOUT_MS)
    except (ValueError, RuntimeError, OSError) as exc:
        logger.warning("Evaluate failed for %s: %s", ticker, exc)
        return None

    doc_id = _extract_doc_id_from_url(result_url)
    if doc_id:
        logger.info("Found docID %s for ticker %s", doc_id, ticker)
        return doc_id
    raise DocIdExtractionError(
        f"Annual report link found for {ticker} but docID extraction failed (result_url={result_url})"
    )


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

    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Searching %s", i, len(tickers), ticker)
        doc_id = search_annual_reports(client, ticker, proxy=proxy)
        if doc_id:
            results[ticker] = doc_id

        if i < len(tickers):
            random_delay(interval * 0.75, interval * 1.25)

    return results
