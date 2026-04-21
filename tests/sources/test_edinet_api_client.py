from __future__ import annotations

import json
from pathlib import Path

import pytest

from stock_db.sources.edinet.api_client import (
    EdinetAPIError,
    download_pdf,
    filter_annual_reports,
    list_documents,
)


def _make_list_response(results: list[dict]) -> dict:
    return {
        "metadata": {
            "title": "API metadata",
            "parameter": {"date": "2025-04-01", "type": "2"},
            "status": "200",
        },
        "results": results,
    }


def _annual_report_doc(sec_code: str = "72030", doc_id: str = "S100ABCDE") -> dict:
    return {
        "docID": doc_id,
        "secCode": sec_code,
        "ordinanceCode": "010",
        "formCode": "030000",
        "docTypeCode": "120",
        "filerName": "Test Corp.",
        "periodStart": "2024-04-01",
        "periodEnd": "2025-03-31",
    }


def _quarterly_report_doc(sec_code: str = "72030", doc_id: str = "S100ZZZZZ") -> dict:
    return {
        "docID": doc_id,
        "secCode": sec_code,
        "ordinanceCode": "010",
        "formCode": "030043",
        "docTypeCode": "120",
        "filerName": "Test Corp.",
        "periodStart": "2024-04-01",
        "periodEnd": "2024-06-30",
    }


class TestFilterAnnualReports:
    def test_filters_annual_reports_only(self) -> None:
        docs = [
            _annual_report_doc(),
            _quarterly_report_doc(),
            {"docID": "S100OTHER", "ordinanceCode": "030", "formCode": "030000"},
        ]

        result = filter_annual_reports(docs)

        assert len(result) == 1
        assert result[0]["docID"] == "S100ABCDE"

    def test_returns_empty_when_no_annual_reports(self) -> None:
        docs = [_quarterly_report_doc()]

        result = filter_annual_reports(docs)

        assert result == []

    def test_filters_by_ordinance_and_form_code(self) -> None:
        docs = [
            _annual_report_doc(doc_id="CORRECT"),
            {**_annual_report_doc(doc_id="WRONG_ORD"), "ordinanceCode": "030"},
            {**_annual_report_doc(doc_id="WRONG_FORM"), "formCode": "043000"},
        ]

        result = filter_annual_reports(docs)

        assert len(result) == 1
        assert result[0]["docID"] == "CORRECT"

    def test_handles_missing_sec_code(self) -> None:
        doc = _annual_report_doc()
        del doc["secCode"]

        result = filter_annual_reports([doc])

        assert result == []


class TestListDocuments:
    def test_returns_results_on_success(self) -> None:
        import stock_db.sources.edinet.api_client as mod

        response_data = _make_list_response([_annual_report_doc()])

        original = mod._http_get_json

        def mock_get_json(url: str, **kwargs: object) -> dict:
            return response_data

        mod._http_get_json = mock_get_json  # type: ignore[attr-defined]
        try:
            results = list_documents("2025-04-01")
        finally:
            mod._http_get_json = original  # type: ignore[attr-defined]

        assert len(results) == 1
        assert results[0]["docID"] == "S100ABCDE"

    def test_raises_on_api_error(self) -> None:
        import stock_db.sources.edinet.api_client as mod

        original = mod._http_get_json

        def mock_get_json(url: str, **kwargs: object) -> dict:
            return {"metadata": {"status": "404", "message": "Not found"}}

        mod._http_get_json = mock_get_json  # type: ignore[attr-defined]
        try:
            with pytest.raises(EdinetAPIError, match="Not found"):
                list_documents("2025-04-01")
        finally:
            mod._http_get_json = original  # type: ignore[attr-defined]


class TestDownloadPdf:
    def test_downloads_pdf_to_dest(self, tmp_path: Path) -> None:
        import stock_db.sources.edinet.api_client as mod

        pdf_content = b"%PDF-1.4 fake content"
        original = mod._download_file

        def mock_download(url: str, dest: Path, **kwargs: object) -> None:
            dest.write_bytes(pdf_content)

        mod._download_file = mock_download  # type: ignore[attr-defined]
        try:
            result = download_pdf("S100ABCDE", tmp_path)
        finally:
            mod._download_file = original  # type: ignore[attr-defined]

        assert result == tmp_path / "S100ABCDE.pdf"
        assert result.read_bytes() == pdf_content
