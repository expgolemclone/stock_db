from __future__ import annotations

from pathlib import Path

from unittest.mock import MagicMock

from stock_db.sources.edinet.api_client import build_pdf_url, build_xbrl_url, doc_id_from_url, download_xbrl


def _valid_ixbrl_html(*, inventory_value: str = "500") -> str:
    return (
        '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL"><head></head><body>'
        '<ix:nonnumeric contextref="FilingDateInstant" '
        'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
        '<ix:nonfraction contextref="CurrentYearInstant" '
        f'name="jppfs_cor:Inventories">{inventory_value}</ix:nonfraction>'
        "</body></html>"
    )


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
        html = _valid_ixbrl_html()
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

    def test_calls_before_request_hook(self, tmp_path: Path) -> None:
        client = MagicMock()
        client.evaluate.return_value = _valid_ixbrl_html()
        calls: list[str] = []

        dest = download_xbrl(
            client,
            "S100TEST",
            tmp_path,
            before_request=lambda: calls.append("called"),
        )

        assert dest is not None
        assert calls == ["called"]

    def test_rejects_header_only_html(self, tmp_path: Path) -> None:
        client = MagicMock()
        client.evaluate.return_value = "<html><head><title>0000000_header.htm</title></head><body>header</body></html>"

        dest = download_xbrl(client, "S100TEST", tmp_path)

        assert dest is None
