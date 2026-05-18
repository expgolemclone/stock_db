from __future__ import annotations

import sqlite3
from typing import ClassVar

import pytest

from stock_db.browser_client.client import BrowserResponse
from stock_db.sources.yahoo_finance_jp.scraper import (
    YFStaleQuoteError,
    discover_company_name,
    discover_suffix,
    scrape_and_store,
)
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import upsert_stock


class FakeBrowserClient:
    _NOT_FOUND_HTML: ClassVar[str] = (
        "<html><head><title>Yahoo!ファイナンス</title></head>"
        "<body><main>指定されたページは表示できません。</main></body></html>"
    )

    def __init__(self, pages: dict[str, str]) -> None:
        self.pages = pages
        self.urls: list[str] = []

    def fetch(self, url: str) -> BrowserResponse:
        self.urls.append(url)
        return BrowserResponse(
            html=self.pages.get(url, self._NOT_FOUND_HTML),
            status=200,
            error=None,
        )


def _not_found_page() -> str:
    return (
        "<html><head><title>Yahoo!ファイナンス</title></head>"
        "<body><main>指定されたページは表示できません。</main></body></html>"
    )


def _quote_page_without_quote() -> str:
    return (
        "<html><head><title>銘柄名【289A】：株価・株式情報 - Yahoo!ファイナンス</title></head>"
        "<body><main>"
        '<dl><dt><span>前日終値</span></dt><dd><span class="value">---</span>'
        '<span class="date">(--/--)</span></dd></dl>'
        "</main></body></html>"
    )


def _quote_page(close: str, date: str) -> str:
    return (
        "<html><head><title>銘柄名【289A】：株価・株式情報 - Yahoo!ファイナンス</title></head>"
        "<body><main>"
        f'<dl><dt><span>前日終値</span></dt><dd><span class="value">{close}</span>'
        f'<span class="date">({date})</span></dd></dl>'
        "</main></body></html>"
    )


def test_discover_suffix_returns_page_suffix_even_without_quote_data() -> None:
    client = FakeBrowserClient(
        {
            "https://finance.yahoo.co.jp/quote/289A.T": _quote_page_without_quote(),
        }
    )

    suffix = discover_suffix(client, "289A", interval=0)

    assert suffix == "T"
    assert client.urls == ["https://finance.yahoo.co.jp/quote/289A.T"]


def test_discover_company_name_returns_normalized_quote_title() -> None:
    client = FakeBrowserClient(
        {
            "https://finance.yahoo.co.jp/quote/8306.T": (
                "<html><head>"
                "<title>(株)三菱ＵＦＪフィナンシャル・グループ【8306】：株価・株式情報 - Yahoo!ファイナンス</title>"
                "</head><body><main></main></body></html>"
            ),
        }
    )

    name, suffix = discover_company_name(client, "8306", interval=0)

    assert (name, suffix) == ("株式会社三菱ＵＦＪフィナンシャル・グループ", "T")
    assert client.urls == ["https://finance.yahoo.co.jp/quote/8306.T"]


def test_scrape_and_store_persists_suffix_for_page_without_quote_data() -> None:
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    init_db(conn)
    upsert_stock(conn, "289A", "銘柄名", "", "")
    conn.commit()

    client = FakeBrowserClient(
        {
            "https://finance.yahoo.co.jp/quote/289A.T": _quote_page_without_quote(),
            "https://finance.yahoo.co.jp/quote/289A.N": _not_found_page(),
            "https://finance.yahoo.co.jp/quote/289A.S": _not_found_page(),
            "https://finance.yahoo.co.jp/quote/289A.F": _not_found_page(),
        }
    )

    ok, errors = scrape_and_store(client, conn, ["289A"], skip_existing=False)

    row = conn.execute(
        "SELECT yf_suffix FROM stocks WHERE ticker = ?",
        ("289A",),
    ).fetchone()

    assert (ok, errors) == (0, 1)
    assert row is not None
    assert row["yf_suffix"] == "T"
    assert client.urls == [
        "https://finance.yahoo.co.jp/quote/289A.T",
        "https://finance.yahoo.co.jp/quote/289A.T",
    ]


def test_scrape_and_store_fails_fast_when_quote_date_is_stale() -> None:
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    init_db(conn)
    upsert_stock(conn, "289A", "銘柄名", "", "")
    conn.commit()

    client = FakeBrowserClient(
        {
            "https://finance.yahoo.co.jp/quote/289A.T": _quote_page("1,000", "05/19"),
        }
    )

    with pytest.raises(YFStaleQuoteError, match="289A.*older than 2026-05-20"):
        scrape_and_store(
            client,
            conn,
            ["289A"],
            skip_existing=False,
            min_date="2026-05-20",
            fail_fast=True,
        )
