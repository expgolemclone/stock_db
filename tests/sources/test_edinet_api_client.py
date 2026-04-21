from __future__ import annotations

from pathlib import Path

from unittest.mock import MagicMock

from stock_db.sources.edinet.api_client import build_pdf_url, build_xbrl_url, doc_id_from_url, download_xbrl


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


class TestBuildXbrlUrl:
    def test_builds_viewer_url(self) -> None:
        result = build_xbrl_url("S100VWVY")

        assert result == "https://disclosure2.edinet-fsa.go.jp/WZEK0040.aspx?S100VWVY,,"


class TestDownloadXbrl:
    def test_saves_html_to_file(self, tmp_path: Path) -> None:
        html = "<html><body>" + "x" * 200 + "</body></html>"
        client = MagicMock()
        client.evaluate.return_value = html

        dest = download_xbrl(client, "S100TEST", tmp_path)

        assert dest is not None
        assert dest.name == "S100TEST.xhtml"
        assert dest.read_text(encoding="utf-8") == html

    def test_returns_none_on_empty_result(self, tmp_path: Path) -> None:
        client = MagicMock()
        client.evaluate.return_value = "<p>short</p>"

        dest = download_xbrl(client, "S100TEST", tmp_path)

        assert dest is None

    def test_returns_none_on_exception(self, tmp_path: Path) -> None:
        client = MagicMock()
        client.evaluate.side_effect = RuntimeError("browser crashed")

        dest = download_xbrl(client, "S100TEST", tmp_path)

        assert dest is None
