"""Download EDINET securities report PDFs, XBRL, and scrape search results."""

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
_XBRL_BASE_URL = "https://disclosure2.edinet-fsa.go.jp/WZEK0040.aspx"
_DEFAULT_PDF_TIMEOUT = 120
_DEFAULT_XBRL_TIMEOUT_MS = 120_000

_EXTRACT_HONBUN_JS = """
(async () => {
  const frame = document.getElementById('frame_honbun');
  if (!frame) return null;
  for (let i = 0; i < 20; i++) {
    await new Promise(r => setTimeout(r, 1000));
    const content = frame.srcdoc || '';
    if (content.length > 100) return content;
  }
  return null;
})()
"""


def doc_id_from_url(url: str) -> str | None:
    """Extract EDINET docID from a PDF URL."""
    m = _PDF_URL_RE.search(url)
    return m.group(1) if m else None


def download_pdf(url: str, dest_dir: Path, *, timeout: float = _DEFAULT_PDF_TIMEOUT) -> Path:
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


def build_xbrl_url(doc_id: str) -> str:
    """Build EDINET document viewer URL for XBRL retrieval."""
    return f"{_XBRL_BASE_URL}?{doc_id},,"


def download_xbrl(
    client: BrowserServiceClient,
    doc_id: str,
    dest_dir: Path,
    *,
    proxy: str | None = None,
    timeout: int = _DEFAULT_XBRL_TIMEOUT_MS,
) -> Path | None:
    """Download iXBRL HTML from EDINET document viewer via browser service."""
    url = build_xbrl_url(doc_id)
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{doc_id}.xhtml"

    try:
        result = client.evaluate(url, _EXTRACT_HONBUN_JS, proxy=proxy, timeout=timeout)
    except (ValueError, RuntimeError, OSError) as exc:
        logger.warning("XBRL evaluate failed for %s: %s", doc_id, exc)
        return None

    if not result or not isinstance(result, str) or len(result) < 100:
        logger.warning("XBRL content too short for %s (%d chars)", doc_id, len(result) if result else 0)
        return None

    dest.write_text(result, encoding="utf-8")
    logger.info("Saved XBRL %s -> %s (%d chars)", doc_id, dest, len(result))
    return dest


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
