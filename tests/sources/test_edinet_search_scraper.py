from __future__ import annotations

import types

import pytest

from stock_db.sources.edinet.search_scraper import (
    _build_search_and_extract_js,
    _extract_doc_id_from_url,
    build_company_name_candidates,
    search_annual_reports,
)


class TestBuildSearchAndExtractJs:
    def test_sets_search_period_to_all_time(self) -> None:
        js = _build_search_and_extract_js(search_ticker="1352")

        assert "const kikan = document.querySelector('#vD_KIKAN');" in js
        assert "kikan.value = '7';" in js

    def test_limits_results_to_annual_reports(self) -> None:
        js = _build_search_and_extract_js(search_ticker="8306")

        assert "const syoruiRadio = document.querySelector('#vD_SYORUI2');" in js
        assert "const cb = document.querySelector('#W0277vCHKSYORUI1');" in js

    def test_uses_requested_field(self) -> None:
        js = _build_search_and_extract_js(company_name="ピックルスコーポレーション")

        assert "document.querySelector('#vD_TEISYUTUSYA_MEISYOU')" in js
        assert "value = 'ピックルスコーポレーション'" in js

    def test_decodes_html_entities_in_company_name(self) -> None:
        js = _build_search_and_extract_js(company_name="ビッグツリーテクノロジー&amp;コンサルティング")

        assert "value = 'ビッグツリーテクノロジー&コンサルティング'" in js
        assert "&amp;" not in js


class TestExtractDocIdFromUrl:
    def test_extracts_doc_id(self) -> None:
        url = "./WZEK0040.aspx?S100VWVY,,"

        assert _extract_doc_id_from_url(url) == "S100VWVY"

    def test_returns_none_for_missing_url(self) -> None:
        assert _extract_doc_id_from_url(None) is None


class TestSearchAnnualReports:
    def test_falls_back_when_edinet_code_search_has_records_but_no_doc_id(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        calls: list[tuple[str | None, str | None, str | None, bool]] = []
        before_request_calls = 0
        responses = iter([
            (None, "E99999", None),
            ("S100TEST1", "E00017", None),
        ])

        def fake_run_search(
            client: object,
            ticker: str,
            *,
            proxy: str | None = None,
            search_ticker: str | None = None,
            edinet_code: str | None = None,
            company_name: str | None = None,
            before_request: object = None,
        ) -> tuple[str | None, str | None, str | None]:
            nonlocal before_request_calls
            calls.append((search_ticker, edinet_code, company_name, before_request is not None))
            if before_request is not None:
                before_request()
                before_request_calls += 1
            return next(responses)

        monkeypatch.setattr(
            "stock_db.sources.edinet.search_scraper._run_search", fake_run_search,
        )

        def before_request() -> None:
            return None

        doc_id, found_edinet = search_annual_reports(
            types.SimpleNamespace(),
            "1352",
            edinet_code="E06845",
            before_request=before_request,
        )

        assert doc_id == "S100TEST1"
        assert found_edinet == "E00017"
        assert calls == [
            (None, "E06845", None, True),
            ("1352", None, None, True),
        ]
        assert before_request_calls == 2

    def test_falls_back_to_company_name_even_on_non_no_records_error(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        calls: list[tuple[str | None, str | None, str | None]] = []
        responses = iter([
            (None, None, None),
            ("S100TEST2", "E00001", None),
        ])

        def fake_run_search(
            client: object,
            ticker: str,
            *,
            proxy: str | None = None,
            search_ticker: str | None = None,
            edinet_code: str | None = None,
            company_name: str | None = None,
            before_request: object = None,
        ) -> tuple[str | None, str | None, str | None]:
            calls.append((search_ticker, edinet_code, company_name))
            return next(responses)

        monkeypatch.setattr(
            "stock_db.sources.edinet.search_scraper._run_search", fake_run_search,
        )

        doc_id, found_edinet = search_annual_reports(
            types.SimpleNamespace(),
            "8306",
            company_name="三菱UFJフィナンシャル・グループ",
        )

        assert doc_id == "S100TEST2"
        assert found_edinet == "E00001"
        assert calls == [
            ("8306", None, None),
            (None, None, "三菱UFJフィナンシャル・グループ"),
        ]

    def test_skips_ticker_code_search_for_alphanumeric_ticker(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        calls: list[tuple[str | None, str | None, str | None]] = []

        def fake_run_search(
            client: object,
            ticker: str,
            *,
            proxy: str | None = None,
            search_ticker: str | None = None,
            edinet_code: str | None = None,
            company_name: str | None = None,
            before_request: object = None,
        ) -> tuple[str | None, str | None, str | None]:
            del client, proxy, before_request
            calls.append((search_ticker, edinet_code, company_name))
            return None, None, "no_records"

        monkeypatch.setattr(
            "stock_db.sources.edinet.search_scraper._run_search", fake_run_search,
        )

        search_annual_reports(
            types.SimpleNamespace(),
            "275A",
            company_name_candidates=["ハンワホームズ株式会社"],
        )

        assert calls == [
            (None, None, "ハンワホームズ株式会社"),
            (None, None, "ハンワホームズ"),
            (None, None, "株式会社ハンワホームズ"),
        ]

    def test_tries_company_name_candidates_in_supplied_order(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        calls: list[str] = []
        responses = iter([
            (None, None, "no_records"),
            (None, None, "no_records"),
            ("S100ALIAS", "E11111", None),
        ])

        def fake_run_search(
            client: object,
            ticker: str,
            *,
            proxy: str | None = None,
            search_ticker: str | None = None,
            edinet_code: str | None = None,
            company_name: str | None = None,
            before_request: object = None,
        ) -> tuple[str | None, str | None, str | None]:
            del client, ticker, proxy, search_ticker, edinet_code, before_request
            if company_name is not None:
                calls.append(company_name)
            return next(responses)

        monkeypatch.setattr(
            "stock_db.sources.edinet.search_scraper._run_search", fake_run_search,
        )

        doc_id, found_edinet = search_annual_reports(
            types.SimpleNamespace(),
            "8306",
            company_name_candidates=[
                "株式会社三菱ＵＦＪフィナンシャル・グループ",
                "三菱ＵＦＪフィナンシャル・グループ",
            ],
        )

        assert doc_id == "S100ALIAS"
        assert found_edinet == "E11111"
        assert calls == [
            "株式会社三菱UFJフィナンシャル・グループ",
            "三菱UFJフィナンシャル・グループ",
        ]


class TestBuildCompanyNameCandidates:
    def test_normalizes_html_entities_fullwidth_and_corporate_markers(self) -> None:
        assert build_company_name_candidates(
            "ビッグツリーテクノロジー&amp;コンサルティング",
            "  (株)ファイントゥデイホールディングス  ",
        ) == [
            "ビッグツリーテクノロジー&コンサルティング",
            "株式会社ビッグツリーテクノロジー&コンサルティング",
            "ビッグツリーテクノロジー&コンサルティング株式会社",
            "株式会社ファイントゥデイホールディングス",
            "ファイントゥデイホールディングス",
            "ファイントゥデイホールディングス株式会社",
        ]
