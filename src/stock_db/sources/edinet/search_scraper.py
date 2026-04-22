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


class EdinetNoRecordsError(RuntimeError):
    """Raised when EDINET returns consecutive 'no records' — search may be broken."""


class DocIdExtractionError(RuntimeError):
    """Raised when annual report link exists but docID extraction fails."""


_MAX_CONSECUTIVE_NO_RECORDS = 10
_consecutive_no_records = 0


def _build_search_url(ticker: str) -> str:
    """証券コードで検索"""
    params = f"scc={ticker}&pfs=6&kbn=2&p=1"
    encoded = base64.b64encode(params.encode()).decode()
    return f"{_SEARCH_BASE_URL}?{encoded}"


def _build_search_url_edinet(edinet_code: str) -> str:
    """EDINETコードで検索"""
    params = f"edc={edinet_code}&pfs=6&kbn=2&p=1"
    encoded = base64.b64encode(params.encode()).decode()
    return f"{_SEARCH_BASE_URL}?{encoded}"


def _build_search_url_name(name: str) -> str:
    """提出者名称で検索"""
    params = f"nam={name}&pfs=6&kbn=2&p=1"
    encoded = base64.b64encode(params.encode()).decode()
    return f"{_SEARCH_BASE_URL}?{encoded}"


_EDINET_CODE_RE = re.compile(r"E\d{5}")


def _extract_edinet_code(html: str) -> str | None:
    """検索結果HTMLからEDINETコード(E05453等)を抽出"""
    m = _EDINET_CODE_RE.search(html)
    return m.group(0) if m else None


def _extract_doc_id_from_url(url: str | None) -> str | None:
    """docIDをWZEK0040.aspxのURLから抽出。例: WZEK0040.aspx?S100VWVY,, → S100VWVY"""
    if not url:
        return None
    match = re.match(r"^[^?]*\?([A-Za-z0-9]+)", url)
    return match.group(1) if match else None


def _fetch_and_check(
    client: BrowserServiceClient,
    url: str,
    ticker: str,
    *,
    proxy: str | None = None,
) -> tuple[object, str | None]:
    """URLをfetchしてブロック/レコードなしを検知。 (resp, edinet_code_or_None)"""
    global _consecutive_no_records

    resp = client.fetch(url, proxy=proxy)
    if resp.error:
        logger.warning("Search failed for %s: %s", ticker, resp.error)
        return resp, None
    if resp.html is None:
        logger.warning("Empty HTML for %s", ticker)
        return resp, None

    for indicator in _BLOCK_INDICATORS:
        if indicator in resp.html:
            raise EdinetBlockError(
                f"EDINET blocked the request for ticker {ticker}: '{indicator}' detected"
            )

    if "レコードがありません" in resp.html:
        _consecutive_no_records += 1
        logger.info("No records (consecutive: %d) for %s", _consecutive_no_records, ticker)
        if _consecutive_no_records >= _MAX_CONSECUTIVE_NO_RECORDS:
            raise EdinetNoRecordsError(
                f"{_consecutive_no_records} consecutive 'no records' — search may be broken"
            )
        return resp, None

    _consecutive_no_records = 0
    found_edinet = _extract_edinet_code(resp.html) if resp.html else None
    return resp, found_edinet


def _try_extract_doc_id(
    client: BrowserServiceClient,
    url: str,
    ticker: str,
    html: str,
    *,
    proxy: str | None = None,
) -> str | None:
    """検索結果HTMLから有価証券報告書リンクをクリックしてdocID抽出"""
    if not re.search(r'TeisyutuSyorui_Click[^>]*>[^<]*有価証券報告書', html):
        logger.info("No annual report link found for ticker %s", ticker)
        return None

    try:
        result_url = client.evaluate(url, _EXTRACT_DOC_ID_JS, proxy=proxy, timeout=_DOCID_EXTRACTION_TIMEOUT_MS)
    except (ValueError, RuntimeError, OSError) as exc:
        logger.warning("Evaluate failed for %s: %s", ticker, exc)
        return None

    doc_id = _extract_doc_id_from_url(result_url)
    if doc_id:
        return doc_id
    raise DocIdExtractionError(
        f"Annual report link found for {ticker} but docID extraction failed (result_url={result_url})"
    )


def search_annual_reports(
    client: BrowserServiceClient,
    ticker: str,
    *,
    proxy: str | None = None,
    edinet_code: str | None = None,
    company_name: str | None = None,
) -> tuple[str | None, str | None]:
    """Search EDINET for the latest annual report of a given ticker.

    Returns (doc_id, edinet_code_or_None).
    Raises EdinetBlockError / EdinetNoRecordsError / DocIdExtractionError.
    """
    # 1. EDINETコードで検索 (DBにあれば)
    if edinet_code:
        url = _build_search_url_edinet(edinet_code)
        resp, found_edinet = _fetch_and_check(client, url, ticker, proxy=proxy)
        if resp.html and "レコードがありません" not in (resp.html or ""):
            doc_id = _try_extract_doc_id(client, url, ticker, resp.html, proxy=proxy)
            if doc_id:
                logger.info("Found docID %s for ticker %s via EDINET code", doc_id, ticker)
                return doc_id, found_edinet or edinet_code

    # 2. 証券コードで検索
    url = _build_search_url(ticker)
    resp, found_edinet = _fetch_and_check(client, url, ticker, proxy=proxy)
    if resp.html and "レコードがありません" not in (resp.html or ""):
        doc_id = _try_extract_doc_id(client, url, ticker, resp.html, proxy=proxy)
        if doc_id:
            logger.info("Found docID %s for ticker %s via ticker code", doc_id, ticker)
            return doc_id, found_edinet

    # 3. 提出者名称でフォールバック
    if company_name and resp.html and "レコードがありません" in (resp.html or ""):
        url = _build_search_url_name(company_name)
        resp, found_edinet = _fetch_and_check(client, url, ticker, proxy=proxy)
        if resp.html and "レコードがありません" not in (resp.html or ""):
            doc_id = _try_extract_doc_id(client, url, ticker, resp.html, proxy=proxy)
            if doc_id:
                logger.info("Found docID %s for ticker %s via company name", doc_id, ticker)
                return doc_id, found_edinet

    return None, found_edinet


def batch_search_doc_ids(
    client: BrowserServiceClient,
    tickers: list[str],
    *,
    proxy: str | None = None,
    interval: float = DEFAULT_INTERVAL_SECONDS,
) -> tuple[dict[str, str], dict[str, str]]:
    """Search EDINET for multiple tickers.

    Returns ({ticker: docID}, {ticker: edinet_code}) mappings.
    Only returns entries where a value was found.
    Raises EdinetBlockError immediately if EDINET blocks any request.
    """
    from stock_db.proxy_pool import random_delay

    doc_ids: dict[str, str] = {}
    edinet_codes: dict[str, str] = {}

    for i, ticker in enumerate(tickers, 1):
        logger.info("[%d/%d] Searching %s", i, len(tickers), ticker)
        doc_id, found_edinet = search_annual_reports(client, ticker, proxy=proxy)
        if doc_id:
            doc_ids[ticker] = doc_id
        if found_edinet:
            edinet_codes[ticker] = found_edinet

        if i < len(tickers):
            random_delay(interval * 0.75, interval * 1.25)

    return doc_ids, edinet_codes
