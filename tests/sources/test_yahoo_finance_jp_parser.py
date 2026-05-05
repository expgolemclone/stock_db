"""Tests for Yahoo Finance Japan quote page parser."""

from __future__ import annotations

import pytest

from stock_db.sources.yahoo_finance_jp.parser import (
    QuoteData,
    extract_quote_page_name,
    is_quote_page,
    parse_quote_page,
)


def _make_dl(name: str, value: str, date: str | None = None) -> str:
    """Build a minimal dl element matching the Yahoo Finance Japan structure."""
    date_html = ""
    if date is not None:
        date_html = f'<span class="_DataListItem__date_xxx">(<!-- -->{date}<!---->)</span>'
    return (
        f'<dl class="_DataListItem_xxx">'
        f"<dt><span>{name}</span></dt>"
        f"<dd>"
        f'<span class="_StyledNumber_xxx">'
        f'<span><span class="_StyledNumber__value_xxx _DataListItem__value_xxx">{value}</span></span>'
        f"</span>"
        f"{date_html}"
        f"</dd></dl>"
    )


def _make_page(*dls: str) -> str:
    body = "".join(dls)
    return (
        "<html><head><title>銘柄名【3442】：株価 - Yahoo!ファイナンス</title></head>"
        f"<body><main>{body}</main></body></html>"
    )


class TestParseQuotePageSuccess:
    """Tests for successful quote page parsing."""

    def test_extracts_close_and_date(self) -> None:
        html = _make_page(
            _make_dl("前日終値", "1,555", "04/28"),
            _make_dl("始値", "1,562", "04/30"),
        )
        result = parse_quote_page(html)
        assert result is not None
        assert result.close == 1555.0
        assert result.date is not None

    def test_extracts_volume(self) -> None:
        html = _make_page(
            _make_dl("前日終値", "1,555", "04/28"),
            _make_dl("出来高", "100", "04/30"),
        )
        result = parse_quote_page(html)
        assert result is not None
        assert result.volume == 100

    def test_volume_without_unit(self) -> None:
        html = _make_page(
            _make_dl("前日終値", "1,555", "04/28"),
            _make_dl("出来高", "1,000", "04/30"),
        )
        result = parse_quote_page(html)
        assert result is not None
        assert result.volume == 1000

    def test_close_with_commas(self) -> None:
        html = _make_page(
            _make_dl("前日終値", "12,345", "04/28"),
        )
        result = parse_quote_page(html)
        assert result is not None
        assert result.close == 12345.0


class TestParseQuotePageNotFound:
    """Tests for pages where the ticker/suffix is invalid."""

    def test_returns_none_for_not_found_page(self) -> None:
        html = (
            "<html><head><title>Yahoo!ファイナンス</title></head>"
            "<body><main>指定されたページは表示できません。</main></body></html>"
        )
        result = parse_quote_page(html)
        assert result is None

    def test_returns_none_for_empty_html(self) -> None:
        result = parse_quote_page("")
        assert result is None

    def test_is_quote_page_returns_false_for_not_found_page(self) -> None:
        html = (
            "<html><head><title>Yahoo!ファイナンス</title></head>"
            "<body><main>指定されたページは表示できません。</main></body></html>"
        )
        assert is_quote_page(html) is False


class TestParseQuotePageNoDate:
    """Tests for quote data without a parseable date."""

    def test_close_without_date(self) -> None:
        html = _make_page(_make_dl("前日終値", "1,555"))
        result = parse_quote_page(html)
        assert result is not None
        assert result.close == 1555.0
        assert result.date is None


class TestIsQuotePage:
    def test_true_for_valid_quote_page_without_quote_data(self) -> None:
        html = (
            "<html><head><title>銘柄名【289A】：株価・株式情報 - Yahoo!ファイナンス</title></head>"
            "<body><main>"
            '<dl><dt><span>前日終値</span></dt><dd><span class="value">---</span>'
            '<span class="date">(--/--)</span></dd></dl>'
            "</main></body></html>"
        )

        assert is_quote_page(html) is True
        assert parse_quote_page(html) is None


class TestExtractQuotePageName:
    def test_converts_suffix_corporate_marker(self) -> None:
        html = (
            "<html><head>"
            "<title>ハンワホームズ(株)【275A】：株価・株式情報 - Yahoo!ファイナンス</title>"
            "</head><body></body></html>"
        )

        assert extract_quote_page_name(html) == "ハンワホームズ株式会社"

    def test_converts_prefix_corporate_marker(self) -> None:
        html = (
            "<html><head>"
            "<title>(株)三菱ＵＦＪフィナンシャル・グループ【8306】：株価・株式情報 - Yahoo!ファイナンス</title>"
            "</head><body></body></html>"
        )

        assert extract_quote_page_name(html) == "株式会社三菱ＵＦＪフィナンシャル・グループ"

    def test_returns_none_for_non_quote_page(self) -> None:
        assert extract_quote_page_name("<html><head><title>Yahoo!ファイナンス</title></head></html>") is None
