from __future__ import annotations

import io
import zipfile
from pathlib import Path

import pytest

from stock_db.sources.edinet.api_client import (
    EdinetApiError,
    build_documents_api_url,
    build_pdf_url,
    build_xbrl_url,
    doc_id_from_url,
    download_xbrl_package,
    get_edinet_api_key,
    require_edinet_api_key,
)


def _xbrl_zip_bytes() -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as zf:
        zf.writestr(
            "XBRL/PublicDoc/report.xhtml",
            (
                '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
                "<body><ix:nonfraction name=\"jppfs_cor:Inventories\">500</ix:nonfraction></body></html>"
            ),
        )
    return buf.getvalue()


class _FakeResponse:
    def __init__(self, body: bytes, *, content_type: str = "application/octet-stream") -> None:
        self._body = body
        self.headers = {"Content-Type": content_type}

    def raise_for_status(self) -> None:
        return None

    def iter_content(self, chunk_size: int = 1024 * 1024) -> list[bytes]:
        del chunk_size
        return [self._body]


class TestDocIdFromUrl:
    def test_extracts_doc_id_from_pdf_url(self) -> None:
        url = "https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100VWVY.pdf"

        result = doc_id_from_url(url)

        assert result == "S100VWVY"

    def test_returns_none_for_non_pdf_url(self) -> None:
        result = doc_id_from_url("https://example.com/page.html")

        assert result is None


class TestBuildUrls:
    def test_builds_standard_pdf_url(self) -> None:
        assert build_pdf_url("S100VWVY") == "https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100VWVY.pdf"

    def test_builds_viewer_url(self) -> None:
        assert build_xbrl_url("S100VWVY") == "https://disclosure2.edinet-fsa.go.jp/WZEK0040.aspx?S100VWVY,,"

    def test_builds_documents_api_url(self) -> None:
        assert build_documents_api_url("S100VWVY") == "https://api.edinet-fsa.go.jp/api/v2/documents/S100VWVY"


class TestApiKeyHelpers:
    def test_reads_api_key_from_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setenv("EDINET_API_KEY", "  secret  ")

        assert get_edinet_api_key() == "secret"

    def test_require_api_key_raises_when_missing(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv("EDINET_API_KEY", raising=False)

        with pytest.raises(EdinetApiError, match="EDINET_API_KEY"):
            require_edinet_api_key()


class TestDownloadXbrlPackage:
    def test_saves_zip_and_extracts_artifact(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr(
            "stock_db.sources.edinet.api_client.requests.get",
            lambda *args, **kwargs: _FakeResponse(_xbrl_zip_bytes()),
        )

        dest = download_xbrl_package("S100TEST", tmp_path, api_key="dummy")

        assert dest == tmp_path / "S100TEST"
        assert (tmp_path / "S100TEST.zip").is_file()
        assert (dest / "XBRL" / "PublicDoc" / "report.xhtml").is_file()

    def test_calls_before_request_hook(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(
            "stock_db.sources.edinet.api_client.requests.get",
            lambda *args, **kwargs: _FakeResponse(_xbrl_zip_bytes()),
        )
        calls: list[str] = []

        download_xbrl_package(
            "S100TEST",
            tmp_path,
            api_key="dummy",
            before_request=lambda: calls.append("called"),
        )

        assert calls == ["called"]

    def test_rejects_non_zip_content_type(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(
            "stock_db.sources.edinet.api_client.requests.get",
            lambda *args, **kwargs: _FakeResponse(b"{}", content_type="application/json; charset=utf-8"),
        )

        with pytest.raises(EdinetApiError, match="content type"):
            download_xbrl_package("S100TEST", tmp_path, api_key="dummy")

    def test_rejects_invalid_zip(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(
            "stock_db.sources.edinet.api_client.requests.get",
            lambda *args, **kwargs: _FakeResponse(b"not a zip"),
        )

        with pytest.raises(EdinetApiError, match="invalid"):
            download_xbrl_package("S100TEST", tmp_path, api_key="dummy")
