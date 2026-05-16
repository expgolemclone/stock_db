"""EDINET API v2 document list client for discovering historical annual reports."""

from __future__ import annotations

import logging
import re
import time
from collections.abc import Callable
from datetime import date, timedelta

import requests

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


def sec_code_to_ticker(sec_code: str | None) -> str | None:
    """Convert 5-digit EDINET secCode to 4-digit ticker (first 4 chars)."""
    if not sec_code or len(sec_code) < 4:
        return None
    return sec_code[:4]


def _fiscal_year_from_period_end(period_end: str | None) -> str:
    """Extract fiscal year from periodEnd date string (e.g. '2024-03-31' -> 'FY2024')."""
    if not period_end:
        return "unknown"
    year = period_end[:4]
    return f"FY{year}" if year.isdigit() else "unknown"


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
) -> dict[str, list[dict]]:
    """Iterate over date range and collect annual report docIDs for target tickers.

    Returns {ticker: [{"doc_id", "fiscal_year", "period_end", "submit_date"}, ...]}.
    """
    start = date.fromisoformat(from_date)
    end = date.fromisoformat(to_date)
    total_days = (end - start).days + 1

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
            ticker = sec_code_to_ticker(r.get("secCode"))
            if ticker is None or ticker not in target_tickers:
                continue
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
