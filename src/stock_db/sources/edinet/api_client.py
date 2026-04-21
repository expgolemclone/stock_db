"""EDINET API v2 client for listing and downloading securities reports."""

from __future__ import annotations

import logging
from pathlib import Path

import requests

logger = logging.getLogger("stock_db.sources.edinet.api_client")

_BASE_URL = "https://disclosure2.edinet-fsa.go.jp/api/v2"
_ANNUAL_REPORT_ORDINANCE = "010"
_ANNUAL_REPORT_FORM = "030000"


class EdinetAPIError(RuntimeError):
    pass


def _http_get_json(url: str, *, params: dict | None = None, timeout: float = 30) -> dict:
    resp = requests.get(url, params=params, timeout=timeout)
    resp.raise_for_status()
    return resp.json()


def _download_file(url: str, dest: Path, *, params: dict | None = None, timeout: float = 120) -> None:
    resp = requests.get(url, params=params, timeout=timeout, stream=True)
    resp.raise_for_status()
    dest.write_bytes(resp.content)


def list_documents(date: str) -> list[dict]:
    """List all documents submitted on `date` (YYYY-MM-DD)."""
    url = f"{_BASE_URL}/documents.json"
    params = {"date": date, "type": 2}
    data = _http_get_json(url, params=params)

    metadata = data.get("metadata", {})
    status = metadata.get("status", "")
    if str(status) != "200":
        msg = metadata.get("message", "Unknown error")
        raise EdinetAPIError(f"EDINET API error (status={status}): {msg}")

    return data.get("results", [])


def filter_annual_reports(docs: list[dict]) -> list[dict]:
    """Filter documents to only annual securities reports (有価証券報告書)."""
    return [
        d for d in docs
        if d.get("ordinanceCode") == _ANNUAL_REPORT_ORDINANCE
        and d.get("formCode") == _ANNUAL_REPORT_FORM
        and d.get("secCode")
    ]


def download_pdf(doc_id: str, dest_dir: Path) -> Path:
    """Download a PDF for the given docID. Returns the saved file path."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{doc_id}.pdf"
    url = f"{_BASE_URL}/documents/{doc_id}"
    params = {"type": 1}
    _download_file(url, dest, params=params)
    logger.info("Downloaded %s -> %s", doc_id, dest)
    return dest
