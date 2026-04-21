"""Download EDINET securities report PDFs and scrape search results."""

from __future__ import annotations

import logging
import re
from pathlib import Path
from typing import TYPE_CHECKING

import requests

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.edinet.api_client")

_PDF_URL_RE = re.compile(r"/searchdocument/pdf/([A-Za-z0-9]+)\.pdf")
_SEARCH_BASE = "https://disclosure2.edinet-fsa.go.jp/EKW01Z01/wk110000"


def doc_id_from_url(url: str) -> str | None:
    """Extract EDINET docID from a PDF URL."""
    m = _PDF_URL_RE.search(url)
    return m.group(1) if m else None


def download_pdf(url: str, dest_dir: Path, *, timeout: float = 120) -> Path:
    """Download a PDF directly via requests. Returns the saved file path."""
    doc_id = doc_id_from_url(url)
    if doc_id is None:
        raise ValueError(f"Cannot extract docID from URL: {url}")

    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{doc_id}.pdf"
    resp = requests.get(url, timeout=timeout, stream=True)
    resp.raise_for_status()
    with open(dest, "wb") as f:
        for chunk in resp.iter_content(8192):
            f.write(chunk)
    logger.info("Downloaded %s -> %s (%d bytes)", url, dest, dest.stat().st_size)
    return dest


def build_pdf_url(doc_id: str) -> str:
    """Build a standardized EDINET PDF URL from a docID."""
    return f"https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/{doc_id}.pdf"


def search_documents_html(
    client: BrowserServiceClient,
    *,
    date: str,
    doc_type: str = "030000",
    proxy: str | None = None,
) -> str:
    """Fetch EDINET search page HTML via browser service."""
    params = {
        "pKbn": "01",
        "pSsn": "99",
        "pLst": date,
        "pTky": "",
    }
    query = "&".join(f"{k}={v}" for k, v in params.items())
    url = f"{_SEARCH_BASE}?{query}"
    resp = client.fetch(url, proxy=proxy)
    if resp.error:
        raise RuntimeError(f"Failed to fetch EDINET search page: {resp.error}")
    if resp.html is None:
        raise RuntimeError("Empty HTML response from EDINET search page")
    return resp.html
