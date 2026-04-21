from __future__ import annotations

from pathlib import Path

from stock_db.sources.edinet.api_client import build_pdf_url, doc_id_from_url


class TestDocIdFromUrl:
    def test_extracts_doc_id_from_pdf_url(self) -> None:
        url = "https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100VWVY.pdf"

        result = doc_id_from_url(url)

        assert result == "S100VWVY"

    def test_returns_none_for_non_pdf_url(self) -> None:
        result = doc_id_from_url("https://example.com/page.html")

        assert result is None

    def test_returns_none_for_empty_string(self) -> None:
        result = doc_id_from_url("")

        assert result is None


class TestBuildPdfUrl:
    def test_builds_standard_url(self) -> None:
        result = build_pdf_url("S100VWVY")

        assert result == "https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100VWVY.pdf"

    def test_roundtrip_with_doc_id_from_url(self) -> None:
        doc_id = "S100ABCDE"
        url = build_pdf_url(doc_id)

        assert doc_id_from_url(url) == doc_id
