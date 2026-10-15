"""EDINET API v2 document list client for discovering historical annual reports."""

from __future__ import annotations

import logging
import re
import time
from collections.abc import Callable
from datetime import date, timedelta
from typing import TYPE_CHECKING

import requests

if TYPE_CHECKING:
    from stock_db.sources.edinet.resolver import EdinetResolver

logger = logging.getLogger("stock_db.sources.edinet.document_list")

_DOCUMENTS_API_BASE_URL = "https://api.edinet-fsa.go.jp/api/v2/documents.json"

# 有価証券報告書: 金融商品取引法(010), 有価証券報告書(030000)
_ORDINANCE_CODE_ANNUAL = "010"
_FORM_CODE_ANNUAL = "030000"
_SUBSCRIPTION_KEY_RE = re.compile(r"(Subscription-Key=)[^&\s)]+")


def _redact_subscription_key(value: object) -> str:
    return _SUBSCRIPTION_KEY_RE.sub(r"\1<redacted>", str(value))


def fetch_document_list(
    target_date: str,
    api_key: str,
    timeout: int = 30,
) -> list[dict]:
    """Fetch the EDINET API v2 document list for a single date.

    Returns the list of result dicts (may be empty).
    """
    response = requests.get(
        _DOCUMENTS_API_BASE_URL,
        params={"date": target_date, "type": 2, "Subscription-Key": api_key},
        timeout=timeout,
    )
    response.raise_for_status()
    data = response.json()
    status = data.get("metadata", {}).get("status")
    if status == "404":
        return []
    if status != "200":
        logger.warning("EDINET API returned status %s for %s", status, target_date)
        return []
    return data.get("results") or []


def filter_annual_reports(results: list[dict]) -> list[dict]:
    """Filter for 有価証券報告書 (ordinanceCode=010, formCode=030000)."""
    return [
        r
        for r in results
        if r.get("ordinanceCode") == _ORDINANCE_CODE_ANNUAL
        and r.get("formCode") == _FORM_CODE_ANNUAL
    ]


# Re-export secCode helper for existing callers.
from stock_db.sources.edinet.resolver import sec_code_to_ticker  # noqa: F401


def _fiscal_year_from_period_end(period_end: str | None) -> str:
    """Extract fiscal year from periodEnd date string (e.g. '2024-03-31' -> 'FY2024')."""
    if not period_end:
        return "unknown"
    year = period_end[:4]
    return f"FY{year}" if year.isdigit() else "unknown"


def _resolve_ticker_from_api_result(
    r: dict,
    target_tickers: set[str],
    edinet_code_to_ticker: dict[str, str] | None,
) -> str | None:
    """Resolve a ticker from an EDINET API result row."""
    from stock_db.sources.edinet.resolver import sec_code_to_ticker as _sec_code_to_ticker

    edinet_code = str(r.get("edinetCode") or "").strip()
    mapped = (
        edinet_code_to_ticker.get(edinet_code)
        if edinet_code and edinet_code_to_ticker
        else None
    )
    ticker = _sec_code_to_ticker(r.get("secCode"))
    if ticker is not None:
        if mapped is not None and mapped != ticker:
            logger.warning(
                "EDINET identifier mismatch for doc_id=%s: secCode maps to "
                "ticker=%s but edinetCode=%s maps to ticker=%s; rejecting",
                r.get("docID"),
                ticker,
                edinet_code,
                mapped,
            )
            return None
        if ticker not in target_tickers:
            return None
        return ticker
    if mapped and mapped in target_tickers:
        return mapped
    return None


def discover_historical_reports(
    from_date: str,
    to_date: str,
    api_key: str,
    target_tickers: set[str],
    interval: float = 0.5,
    on_progress: Callable[[int, int], None] | None = None,
    initial_reports: dict[str, list[dict]] | None = None,
    skip_dates: set[str] | None = None,
    on_day_scanned: Callable[[str, list[tuple[str, dict]], int], None] | None = None,
    on_day_error: Callable[[str, str], None] | None = None,
    edinet_code_to_ticker: dict[str, str] | None = None,
    resolver: EdinetResolver | None = None,
) -> dict[str, list[dict]]:
    """Iterate over date range and collect annual report docIDs for target tickers.

    Returns {ticker: [{"doc_id", "fiscal_year", "period_end", "submit_date"}, ...]}.

    Records that lack a valid ``secCode`` can still be matched by
    reverse-looking up ``edinetCode`` when a mapping is available.
    """
    start = date.fromisoformat(from_date)
    end = date.fromisoformat(to_date)
    total_days = (end - start).days + 1
    if edinet_code_to_ticker is None and resolver is not None:
        edinet_code_to_ticker = resolver.edinet_code_reverse_map

    reports: dict[str, list[dict]] = {
        t: list((initial_reports or {}).get(t, [])) for t in target_tickers
    }
    seen_doc_ids = {
        (ticker, str(report["doc_id"]))
        for ticker, ticker_reports in reports.items()
        for report in ticker_reports
        if report.get("doc_id")
    }
    skip_dates = skip_dates or set()
    total_annual = 0

    for i in range(total_days):
        current = start + timedelta(days=i)
        date_str = current.isoformat()

        if on_progress:
            on_progress(i + 1, total_days)

        if date_str in skip_dates:
            continue

        try:
            results = fetch_document_list(date_str, api_key)
        except requests.RequestException as exc:
            error = _redact_subscription_key(exc)
            logger.warning("API error for %s: %s", date_str, error)
            if on_day_error:
                on_day_error(date_str, error)
            time.sleep(interval)
            continue

        annual = filter_annual_reports(results)
        if not annual:
            if on_day_scanned:
                on_day_scanned(date_str, [], 0)
            time.sleep(interval)
            continue

        total_annual += len(annual)
        matched_for_day: list[tuple[str, dict]] = []
        for r in annual:
            ticker = _resolve_ticker_from_api_result(r, target_tickers, edinet_code_to_ticker)
            if ticker is None:
                continue
            if resolver is not None:
                resolver.check_filer_name(ticker, r.get("filerName"))
            doc_id = r["docID"]
            if (ticker, doc_id) in seen_doc_ids:
                continue
            doc_info = {
                "doc_id": doc_id,
                "fiscal_year": _fiscal_year_from_period_end(r.get("periodEnd")),
                "period_end": r.get("periodEnd"),
                "submit_date": r.get("submitDateTime"),
                "filer_name": r.get("filerName"),
            }
            reports[ticker].append(doc_info)
            seen_doc_ids.add((ticker, doc_id))
            matched_for_day.append((ticker, doc_info))

        if on_day_scanned:
            on_day_scanned(date_str, matched_for_day, len(annual))

        time.sleep(interval)

    matched = {t: docs for t, docs in reports.items() if docs}
    logger.info(
        "Discovered %d annual reports for %d tickers over %d days",
        sum(len(v) for v in matched.values()),
        len(matched),
        total_days,
    )
    return matched
