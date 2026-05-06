"""Scrape EDINET search results to find securities report docIDs via browser service.

Uses GeneXus form interaction (input → checkbox → search button → PostBack)
then clicks the annual report link to capture the docID URL — all within a
single evaluate() call so the PostBack results are not lost.
"""

from __future__ import annotations

import html
import json
import logging
import re
import threading
import unicodedata
from collections.abc import Callable
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.edinet.search_scraper")

_SEARCH_FORM_URL = "https://disclosure2.edinet-fsa.go.jp/weee0050.aspx"
_BLOCK_INDICATORS = ("規定外操作", "エラー画面", "Message code")
DEFAULT_INTERVAL_SECONDS = 2.0
_POSTBACK_WAIT_MS = 25000
_DOCID_CAPTURE_WAIT_MS = 5000
_DOC_TYPE_TOGGLE_WAIT_MS = 500

_EDINET_CODE_RE = re.compile(r">\s*(E\d{5})\s*<")


class EdinetBlockError(RuntimeError):
    """Raised when EDINET returns a block/error page."""


class EdinetNoRecordsError(RuntimeError):
    """Raised when EDINET returns consecutive 'no records' — search may be broken."""


class DocIdExtractionError(RuntimeError):
    """Raised when annual report link exists but docID extraction fails."""


_MAX_CONSECUTIVE_NO_RECORDS = 100
_consecutive_no_records = 0
_no_records_lock = threading.Lock()


def _increment_no_records() -> int:
    global _consecutive_no_records
    with _no_records_lock:
        _consecutive_no_records += 1
        return _consecutive_no_records


def _reset_no_records() -> None:
    global _consecutive_no_records
    with _no_records_lock:
        _consecutive_no_records = 0


def _normalize_company_name(name: str | None) -> str | None:
    if name is None:
        return None
    normalized = html.unescape(name).strip()
    if not normalized:
        return None
    normalized = unicodedata.normalize("NFKC", normalized)
    normalized = re.sub(r"\s+", " ", normalized)
    normalized = normalized.replace("（株）", "株式会社").replace("(株)", "株式会社")
    return normalized.strip() or None


def _dedupe_preserve_order(names: list[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for name in names:
        if name not in seen:
            seen.add(name)
            ordered.append(name)
    return ordered


def build_company_name_candidates(*names: str | None) -> list[str]:
    candidates: list[str] = []
    for raw_name in names:
        name = _normalize_company_name(raw_name)
        if not name:
            continue
        variants = [name]
        if name.startswith("株式会社"):
            stripped = name.removeprefix("株式会社").strip()
            if stripped:
                variants.append(stripped)
                variants.append(f"{stripped}株式会社")
        elif name.endswith("株式会社"):
            stripped = name.removesuffix("株式会社").strip()
            if stripped:
                variants.append(stripped)
                variants.append(f"株式会社{stripped}")
        else:
            variants.append(f"株式会社{name}")
            variants.append(f"{name}株式会社")
        candidates.extend(variants)
    return _dedupe_preserve_order([candidate for candidate in candidates if candidate])


def _supports_ticker_code_search(ticker: str) -> bool:
    return ticker.isdigit()


def _build_search_and_extract_js(
    *,
    search_ticker: str | None = None,
    edinet_code: str | None = None,
    company_name: str | None = None,
) -> str:
    """Build JS that: fills form → clicks search → waits PostBack → clicks yuhou link.

    Returns JSON {capturedUrl, edinetCode, noRecords, blocked}.
    """
    if edinet_code:
        field_id = "vD_TEISYUTUSYA_EDINET"
        value = edinet_code
    elif search_ticker:
        field_id = "vD_TEISYUTUSYA_SYOUKEN"
        value = search_ticker
    elif company_name:
        field_id = "vD_TEISYUTUSYA_MEISYOU"
        value = html.unescape(company_name)
    else:
        msg = "At least one of ticker, edinet_code, or company_name is required"
        raise ValueError(msg)

    escaped = value.replace("\\", "\\\\").replace("'", "\\'")

    return rf"""
(async () => {{
    const input = document.querySelector('#{field_id}');
    input.focus();
    input.value = '{escaped}';
    input.dispatchEvent(new Event('focus', {{ bubbles: true }}));
    input.dispatchEvent(new Event('input', {{ bubbles: true }}));
    input.dispatchEvent(new Event('change', {{ bubbles: true }}));
    input.dispatchEvent(new Event('blur', {{ bubbles: true }}));

    const cb = document.querySelector('#W0277vCHKSYORUI1');
    const syoruiRadio = document.querySelector('#vD_SYORUI2');
    if (syoruiRadio && !syoruiRadio.checked) {{
        syoruiRadio.click();
        syoruiRadio.dispatchEvent(new Event('change', {{ bubbles: true }}));
    }}
    await new Promise(r => setTimeout(r, {_DOC_TYPE_TOGGLE_WAIT_MS}));

    if (cb && !cb.checked) {{
        cb.click();
        cb.dispatchEvent(new Event('change', {{ bubbles: true }}));
    }}

    // EDINET defaults to "past 1 year", which misses delisted/merged issuers
    // whose latest annual report is older. Force "all period" before searching.
    const kikan = document.querySelector('#vD_KIKAN');
    if (kikan) {{
        kikan.value = '7';
        kikan.dispatchEvent(new Event('change', {{ bubbles: true }}));
        kikan.dispatchEvent(new Event('blur', {{ bubbles: true }}));
    }}

    document.querySelector('#BTNBTNSEARCHTEISYUTUSYA').click();
    await new Promise(r => setTimeout(r, {_POSTBACK_WAIT_MS}));

    // レコードなし確認
    if (document.body.innerText.includes('\u30EC\u30B3\u30FC\u30C9\u304C\u3042\u308A\u307E\u305B\u3093')) {{
        return JSON.stringify({{noRecords: true}});
    }}

    // ブロック確認
    const blockWords = ['\u898F\u5B9A\u5916\u64CD\u4F5C', '\u30A8\u30E9\u30FC\u753B\u9762', 'Message code'];
    for (const w of blockWords) {{
        if (document.body.innerText.includes(w)) {{
            return JSON.stringify({{blocked: w}});
        }}
    }}

    // EDINETコード抽出
    const edinetMatch = document.body.innerHTML.match(/>\\s*(E\\d{{5}})\\s*</);
    const edinetCode = edinetMatch ? edinetMatch[1] : null;

    // 有価証券報告書リンクをクリックしてdocID URLをキャプチャ
    let capturedUrl = null;
    const originalOpen = window.open;
    window.open = (url) => {{ capturedUrl = url; return null; }};

    const links = document.querySelectorAll('a[onclick]');
    for (const link of links) {{
        const text = link.textContent.trim();
        if (text.includes('\u6709\u4FA1\u8A3C\u5238\u5831\u544A\u66F8') && !text.includes('\u8A02\u6B63')) {{
            link.click();
            break;
        }}
    }}

    await new Promise(r => setTimeout(r, {_DOCID_CAPTURE_WAIT_MS}));
    window.open = originalOpen;

    return JSON.stringify({{capturedUrl, edinetCode}});
}})()"""


def _extract_doc_id_from_url(url: str | None) -> str | None:
    """docIDをWZEK0040.aspxのURLから抽出。例: ./WZEK0040.aspx?S100VWVY,, → S100VWVY"""
    if not url:
        return None
    match = re.match(r"^[^?]*\?([A-Za-z0-9]+)", url)
    return match.group(1) if match else None


def _run_search(
    client: BrowserServiceClient,
    ticker: str,
    *,
    proxy: str | None = None,
    search_ticker: str | None = None,
    edinet_code: str | None = None,
    company_name: str | None = None,
    before_request: Callable[[], None] | None = None,
) -> tuple[str | None, str | None, str | None]:
    """Execute search + extract in one evaluate call.

    Returns (doc_id, edinet_code, error_type).
    error_type: 'no_records' | 'blocked' | 'extraction_failed' | None
    """
    global _consecutive_no_records

    js = _build_search_and_extract_js(
        search_ticker=search_ticker, edinet_code=edinet_code, company_name=company_name,
    )
    try:
        if before_request is not None:
            before_request()
        result = client.evaluate(_SEARCH_FORM_URL, js, proxy=proxy, timeout=120000)
    except (ValueError, RuntimeError, OSError) as exc:
        logger.warning("Search evaluate failed for %s: %s", ticker, exc)
        return None, None, None

    try:
        data = json.loads(str(result))
    except (json.JSONDecodeError, TypeError):
        logger.warning("Invalid JSON from search for %s: %s", ticker, str(result)[:200])
        return None, None, None

    # ブロック検知
    if data.get("blocked"):
        raise EdinetBlockError(
            f"EDINET blocked the request for ticker {ticker}: '{data['blocked']}' detected"
        )

    # レコードなし
    if data.get("noRecords"):
        count = _increment_no_records()
        logger.info("No records (consecutive: %d) for %s", count, ticker)
        if count >= _MAX_CONSECUTIVE_NO_RECORDS:
            raise EdinetNoRecordsError(
                f"{count} consecutive 'no records' — search may be broken"
            )
        return None, None, "no_records"

    _reset_no_records()

    doc_id = _extract_doc_id_from_url(data.get("capturedUrl"))
    edinet = data.get("edinetCode")

    if not doc_id and data.get("capturedUrl"):
        raise DocIdExtractionError(
            f"Annual report link found for {ticker} but docID extraction failed "
            f"(capturedUrl={data['capturedUrl']})"
        )

    return doc_id, edinet, None


def search_annual_reports(
    client: BrowserServiceClient,
    ticker: str,
    *,
    proxy: str | None = None,
    edinet_code: str | None = None,
    company_name: str | None = None,
    company_name_candidates: list[str] | None = None,
    before_request: Callable[[], None] | None = None,
) -> tuple[str | None, str | None]:
    """Search EDINET for the latest annual report of a given ticker.

    Returns (doc_id, edinet_code_or_None).
    Raises EdinetBlockError / EdinetNoRecordsError / DocIdExtractionError.
    """
    # 1. EDINETコードで検索 (DBにあれば)
    if edinet_code:
        doc_id, found_edinet, err = _run_search(
            client, ticker, proxy=proxy, edinet_code=edinet_code,
            before_request=before_request,
        )
        if doc_id:
            logger.info("Found docID %s for ticker %s via EDINET code", doc_id, ticker)
            return doc_id, found_edinet or edinet_code

    if _supports_ticker_code_search(ticker):
        # 2. 証券コードで検索
        doc_id, found_edinet, err = _run_search(
            client, ticker, proxy=proxy, search_ticker=ticker,
            before_request=before_request,
        )
        if doc_id:
            logger.info("Found docID %s for ticker %s via ticker code", doc_id, ticker)
            return doc_id, found_edinet

    candidates = build_company_name_candidates(*(company_name_candidates or []))
    if company_name is not None:
        candidates = _dedupe_preserve_order(
            candidates + build_company_name_candidates(company_name),
        )
    for candidate in candidates:
        # 3. 提出者名称でフォールバック
        doc_id, found_edinet, err = _run_search(
            client, ticker, proxy=proxy, company_name=candidate,
            before_request=before_request,
        )
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
