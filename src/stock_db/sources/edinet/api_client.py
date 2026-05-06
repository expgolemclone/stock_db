"""Download EDINET securities report artifacts and scrape search results."""

from __future__ import annotations

import logging
import os
import re
import shutil
import tempfile
import zipfile
from collections.abc import Callable
from pathlib import Path
from typing import TYPE_CHECKING

import requests

if TYPE_CHECKING:
    from stock_db.browser_client.client import BrowserServiceClient

logger = logging.getLogger("stock_db.sources.edinet.api_client")

_PDF_URL_RE = re.compile(r"/searchdocument/pdf/([A-Za-z0-9]+)\.pdf")
_SEARCH_BASE = "https://disclosure2.edinet-fsa.go.jp/EKW01Z01/wk110000"
_XBRL_BASE_URL = "https://disclosure2.edinet-fsa.go.jp/WZEK0040.aspx"
_DOCUMENTS_API_BASE_URL = "https://api.edinet-fsa.go.jp/api/v2/documents"
_API_KEY_ENV = "EDINET_API_KEY"
_DEFAULT_XBRL_TIMEOUT_SECONDS = 300
_ZIP_CONTENT_TYPES: tuple[str, ...] = (
    "application/octet-stream",
    "application/zip",
    "application/x-zip-compressed",
)


class EdinetApiError(RuntimeError):
    """Raised when the EDINET API download cannot be completed."""


def doc_id_from_url(url: str) -> str | None:
    """Extract EDINET docID from a PDF URL."""
    m = _PDF_URL_RE.search(url)
    return m.group(1) if m else None


def build_pdf_url(doc_id: str) -> str:
    """Build a standardized EDINET PDF URL from a docID."""
    return f"https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/{doc_id}.pdf"


def build_xbrl_url(doc_id: str) -> str:
    """Build EDINET document viewer URL for XBRL retrieval."""
    return f"{_XBRL_BASE_URL}?{doc_id},,"


def build_documents_api_url(doc_id: str) -> str:
    """Build EDINET API v2 document download URL."""
    return f"{_DOCUMENTS_API_BASE_URL}/{doc_id}"


def get_edinet_api_key() -> str | None:
    """Return the configured EDINET API key, if any."""
    value = os.environ.get(_API_KEY_ENV)
    if value is None:
        return None
    stripped = value.strip()
    return stripped or None


def require_edinet_api_key() -> str:
    """Return the EDINET API key or raise a helpful error."""
    api_key = get_edinet_api_key()
    if api_key is None:
        raise EdinetApiError(f"{_API_KEY_ENV} is not set")
    return api_key


def _ensure_safe_zip_member(base_dir: Path, member_name: str) -> None:
    destination = (base_dir / member_name).resolve()
    if not str(destination).startswith(str(base_dir.resolve())):
        raise EdinetApiError(f"Unsafe ZIP member path: {member_name}")


def _looks_like_xbrl_package(extract_dir: Path) -> bool:
    for suffix in ("*.xhtml", "*.html", "*.htm", "*.xbrl"):
        if next(extract_dir.rglob(suffix), None) is not None:
            return True
    return False


def _replace_existing_path(path: Path) -> None:
    if path.is_dir():
        shutil.rmtree(path)
    elif path.exists():
        path.unlink()


def download_xbrl_package(
    doc_id: str,
    dest_dir: Path,
    *,
    api_key: str,
    timeout: int = _DEFAULT_XBRL_TIMEOUT_SECONDS,
    before_request: Callable[[], None] | None = None,
) -> Path:
    """Download the EDINET API ZIP, save it locally, and extract it atomically."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    zip_dest = dest_dir / f"{doc_id}.zip"
    extract_dest = dest_dir / doc_id
    temp_root = Path(tempfile.mkdtemp(prefix=f"{doc_id}.", dir=dest_dir))
    temp_zip = temp_root / f"{doc_id}.zip"
    temp_extract = temp_root / doc_id

    try:
        if before_request is not None:
            before_request()
        response = requests.get(
            build_documents_api_url(doc_id),
            params={"type": 1, "Subscription-Key": api_key},
            stream=True,
            timeout=timeout,
        )
        response.raise_for_status()
        content_type = response.headers.get("Content-Type", "").lower()
        if not any(expected in content_type for expected in _ZIP_CONTENT_TYPES):
            raise EdinetApiError(
                f"Unexpected EDINET API content type for {doc_id}: {content_type or 'missing'}"
            )

        with temp_zip.open("wb") as f:
            for chunk in response.iter_content(chunk_size=1024 * 1024):
                if chunk:
                    f.write(chunk)

        temp_extract.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(temp_zip) as zf:
            for member in zf.namelist():
                _ensure_safe_zip_member(temp_extract, member)
            zf.extractall(temp_extract)

        if not _looks_like_xbrl_package(temp_extract):
            raise EdinetApiError(f"EDINET API ZIP for {doc_id} did not contain XBRL artifacts")

        _replace_existing_path(zip_dest)
        _replace_existing_path(extract_dest)
        shutil.move(str(temp_zip), str(zip_dest))
        shutil.move(str(temp_extract), str(extract_dest))
        logger.info("Saved EDINET API XBRL package %s -> %s", doc_id, extract_dest)
        return extract_dest
    except requests.RequestException as exc:
        raise EdinetApiError(f"EDINET API request failed for {doc_id}: {exc}") from exc
    except zipfile.BadZipFile as exc:
        raise EdinetApiError(f"EDINET API ZIP was invalid for {doc_id}: {exc}") from exc
    finally:
        shutil.rmtree(temp_root, ignore_errors=True)


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
