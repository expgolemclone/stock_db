from __future__ import annotations

import sqlite3

from stock_db.sources.irbank.bs_parser import parse_latest_annual_bs_page
from stock_db.sources.irbank.bs_scraper import scrape_and_store
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.stocks import get_existing_tickers, upsert_stock

_SUMMARY_URL = "https://irbank.net/{ticker}/bs"
_DETAIL_URL_03 = "https://irbank.net/E00001/S100W78E/bs"
_DETAIL_URL_08 = "https://irbank.net/E00001/S100X6X6/bs"


class _FakeClient:
    def __init__(self, responses: dict[str, object]) -> None:
        self._responses = responses

    def fetch(self, url: str, *, proxy: str | None = None) -> object:
        return self._responses[url]


class _SequenceClient:
    def __init__(self, responses: dict[str, list[object]]) -> None:
        self._responses = {url: list(items) for url, items in responses.items()}

    def fetch(self, url: str, *, proxy: str | None = None) -> object:
        items = self._responses[url]
        if len(items) == 1:
            return items[0]
        return items.pop(0)


class _FakeResponse:
    def __init__(self, html: str | None, status: int = 200, error: str | None = None) -> None:
        self.html = html
        self.status = status
        self.error = error


def _overview_html(
    *,
    include_annual_link: bool = True,
    periods: list[tuple[str, str]] | None = None,
) -> str:
    if periods is None:
        periods = [("2025/03", "S100W78E/bs")] if include_annual_link else []
    links = "\n".join(
        f'<tr><td><a href="{href}">{text}</a></td><td>44,312</td></tr>'
        for text, href in periods
    )
    return f"""
    <html>
      <head>
        <link rel="canonical" href="https://irbank.net/E00001/bs">
      </head>
      <body>
        <table class="cs">
          <tbody>
            {links}
          </tbody>
        </table>
      </body>
    </html>
    """


def _detailed_html() -> str:
    return """
    <html><body>
      <table id="c_bs1">
        <caption>貸借対照表（千円）</caption>
        <tr><th>勘定科目</th><th class="weaken head">2024年3月31日</th><th class="weaken head">2025年3月31日</th></tr>
        <tr><td>現金及び預金</td><td class="value">1,735,615</td><td class="value">3,514,675</td></tr>
        <tr><td>商品</td><td class="value">8,284</td><td class="value">0</td></tr>
        <tr><td>販売用不動産</td><td class="value">28,439,999</td><td class="value">28,526,855</td></tr>
        <tr><td>信託販売用不動産</td><td class="value">0</td><td class="value">4,447,612</td></tr>
        <tr><td>未成工事支出金</td><td class="value">57,464</td><td class="value">8,737</td></tr>
        <tr><td>流動資産計</td><td class="value">32,496,956</td><td class="value">38,675,872</td></tr>
        <tr><td>投資有価証券</td><td class="value">3,045,373</td><td class="value">2,985,654</td></tr>
        <tr><td>支払手形及び買掛金</td><td class="value">1,122,900</td><td class="value">4,678,449</td></tr>
        <tr><td>流動負債計</td><td class="value">11,835,413</td><td class="value">15,158,894</td></tr>
        <tr><td>固定負債計</td><td class="value">815,522</td><td class="value">1,468,637</td></tr>
        <tr><td>株主資本合計</td><td class="value">24,787,980</td><td class="value">27,314,974</td></tr>
        <tr><td>純資産の部合計</td><td class="value">25,450,939</td><td class="value">27,684,817</td></tr>
      </table>
    </body></html>
    """


def _detailed_html_august() -> str:
    return """
    <html><body>
      <table id="c_bs1">
        <caption>貸借対照表（百万円）</caption>
        <tr><th>勘定科目</th><th class="weaken head">2024年8月31日</th><th class="weaken head">2025年8月31日</th></tr>
        <tr><td>現金及び現金同等物（IFRS）</td><td class="value">1,100</td><td class="value">1,250</td></tr>
        <tr><td>売掛金及びその他の短期債権</td><td class="value">220</td><td class="value">330</td></tr>
        <tr><td>棚卸資産</td><td class="value">0</td><td class="value">15</td></tr>
        <tr><td>流動資産合計</td><td class="value">7,000</td><td class="value">8,000</td></tr>
        <tr><td>買掛金及びその他の短期債務</td><td class="value">2,000</td><td class="value">3,000</td></tr>
        <tr><td>流動負債合計</td><td class="value">3,000</td><td class="value">4,000</td></tr>
        <tr><td>非流動負債合計</td><td class="value">500</td><td class="value">600</td></tr>
        <tr><td>親会社の所有者に帰属する持分（IFRS）</td><td class="value">2,800</td><td class="value">3,200</td></tr>
        <tr><td>資本合計</td><td class="value">3,500</td><td class="value">4,000</td></tr>
      </table>
    </body></html>
    """


class TestIrbankBsScraper:
    def test_parse_latest_annual_bs_page_uses_latest_column_only(self) -> None:
        parsed = parse_latest_annual_bs_page(_detailed_html())

        assert set(parsed) == {"2025-03"}
        assert parsed["2025-03"]["cash_and_deposits"] == 3_514_675_000.0
        assert parsed["2025-03"]["current_assets"] == 38_675_872_000.0
        assert parsed["2025-03"]["investment_securities"] == 2_985_654_000.0
        assert parsed["2025-03"]["trade_payables"] == 4_678_449_000.0
        assert parsed["2025-03"]["inventories"] == 32_983_204_000.0

    def test_latest_annual_detail_page_is_stored_for_ticker(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7203", "Test Corp.", "", "")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="7203"): _FakeResponse(_overview_html()),
            _DETAIL_URL_03: _FakeResponse(_detailed_html()),
        })

        ok, errors = scrape_and_store(client, db_conn, ["7203"], skip_existing=False)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT period, statement, item_name, value
            FROM financial_items
            WHERE ticker = '7203' AND source = 'irbank_bs'
            ORDER BY period, statement, item_name
            """
        ).fetchall()
        assert {row["period"] for row in rows} == {"2025-03"}
        assert ("2025-03", "bs", "inventories", 32_983_204_000.0) in [tuple(row) for row in rows]
        assert ("2025-03", "bs", "investment_securities", 2_985_654_000.0) in [tuple(row) for row in rows]

    def test_force_refresh_replaces_existing_source_rows(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7203", "Test Corp.", "", "")
        upsert_financial_item(db_conn, "7203", "2024-03", "bs", "legacy_metric", 1.0, "irbank_bs")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="7203"): _FakeResponse(_overview_html()),
            _DETAIL_URL_03: _FakeResponse(_detailed_html()),
        })

        ok, errors = scrape_and_store(client, db_conn, ["7203"], skip_existing=False)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT statement, item_name, value
            FROM financial_items
            WHERE ticker = '7203' AND source = 'irbank_bs'
            ORDER BY statement, item_name
            """
        ).fetchall()
        assert ("bs", "legacy_metric", 1.0) not in [tuple(row) for row in rows]
        assert ("bs", "inventories", 32_983_204_000.0) in [tuple(row) for row in rows]

    def test_transient_fetch_error_retries_and_succeeds(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7203", "Retry Corp.", "", "")
        db_conn.commit()

        client = _SequenceClient({
            _SUMMARY_URL.format(ticker="7203"): [
                _FakeResponse(
                    None,
                    error="net::ERR_INTERNET_DISCONNECTED at https://irbank.net/7203/bs",
                ),
                _FakeResponse(_overview_html()),
            ],
            _DETAIL_URL_03: [_FakeResponse(_detailed_html())],
        })

        ok, errors = scrape_and_store(client, db_conn, ["7203"], skip_existing=False)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT statement, item_name
            FROM financial_items
            WHERE ticker = '7203' AND source = 'irbank_bs'
            ORDER BY statement, item_name
            """
        ).fetchall()
        assert ("_status", "fetch_error.net_err_internet_disconnected") not in [tuple(row) for row in rows]
        assert ("bs", "inventories") in [tuple(row) for row in rows]

    def test_missing_annual_detail_link_is_recorded_as_status_row(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_stock(db_conn, "7699", "No Detail Link Inc.", "", "")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="7699"): _FakeResponse(_overview_html(include_annual_link=False)),
        })

        ok, errors = scrape_and_store(client, db_conn, ["7699"], skip_existing=False)

        assert (ok, errors) == (0, 1)
        rows = db_conn.execute(
            """
            SELECT period, statement, item_name, value, source
            FROM financial_items
            WHERE ticker = '7699'
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [
            ("0000-00", "_status", "no_annual_detail_link", None, "irbank_bs"),
        ]
        assert get_existing_tickers(db_conn, "irbank_bs") == {"7699"}

    def test_fetch_error_is_recorded_and_retried_on_future_default_runs(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_stock(db_conn, "9914", "Retry Later Corp.", "", "")
        db_conn.commit()

        failing_client = _FakeClient({
            _SUMMARY_URL.format(ticker="9914"): _FakeResponse(_overview_html(include_annual_link=False)),
        })
        ok, errors = scrape_and_store(failing_client, db_conn, ["9914"], skip_existing=False)
        assert (ok, errors) == (0, 1)

        rows = db_conn.execute(
            """
            SELECT period, statement, item_name
            FROM financial_items
            WHERE ticker = '9914' AND source = 'irbank_bs'
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [
            ("0000-00", "_status", "no_annual_detail_link"),
        ]

        succeeding_client = _FakeClient({
            _SUMMARY_URL.format(ticker="9914"): _FakeResponse(_overview_html()),
            _DETAIL_URL_03: _FakeResponse(_detailed_html()),
        })
        ok, errors = scrape_and_store(succeeding_client, db_conn, ["9914"], skip_existing=True)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT statement, item_name
            FROM financial_items
            WHERE ticker = '9914' AND source = 'irbank_bs'
            ORDER BY statement, item_name
            """
        ).fetchall()
        assert ("_status", "no_annual_detail_link") not in [tuple(row) for row in rows]
        assert ("bs", "inventories") in [tuple(row) for row in rows]

    def test_skip_existing_treats_no_data_status_row_as_processed(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "9257", "Status Only Ltd.", "", "")
        upsert_financial_item(db_conn, "9257", "0000-00", "_status", "no_data", None, "irbank_bs")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="9257"): _FakeResponse(_overview_html()),
            _DETAIL_URL_03: _FakeResponse(_detailed_html()),
        })

        ok, errors = scrape_and_store(client, db_conn, ["9257"], skip_existing=True)

        assert (ok, errors) == (0, 0)
        rows = db_conn.execute(
            """
            SELECT period, statement, item_name
            FROM financial_items
            WHERE ticker = '9257' AND source = 'irbank_bs'
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [("0000-00", "_status", "no_data")]

    def test_non_march_fiscal_year_uses_most_common_annual_month(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "9983", "August Corp.", "", "")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="9983"): _FakeResponse(_overview_html(periods=[
                ("2024/08", "S100UV0T/bs"),
                ("2025/05", "081220250707509560/bs"),
                ("2025/08", "S100X6X6/bs"),
                ("2025/11", "081220260107530230/bs"),
                ("2026/02", "S100XXP2/bs"),
            ])),
            _DETAIL_URL_08: _FakeResponse(_detailed_html_august()),
        })

        ok, errors = scrape_and_store(client, db_conn, ["9983"], skip_existing=False)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT period, item_name, value
            FROM financial_items
            WHERE ticker = '9983' AND source = 'irbank_bs'
            ORDER BY item_name
            """
        ).fetchall()
        assert {row["period"] for row in rows} == {"2025-08"}
        assert ("2025-08", "current_assets", 8_000_000_000.0) in [tuple(row) for row in rows]
        assert ("2025-08", "inventories", 15_000_000.0) in [tuple(row) for row in rows]
        assert ("2025-08", "non_current_liabilities", 600_000_000.0) in [tuple(row) for row in rows]

    def test_ambiguous_annual_month_is_recorded_as_status_row(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_stock(db_conn, "4452", "Ambiguous Corp.", "", "")
        db_conn.commit()

        client = _FakeClient({
            _SUMMARY_URL.format(ticker="4452"): _FakeResponse(_overview_html(periods=[
                ("2024/03", "S100AAAA/bs"),
                ("2025/03", "S100BBBB/bs"),
                ("2024/12", "S100CCCC/bs"),
                ("2025/12", "S100DDDD/bs"),
            ])),
        })

        ok, errors = scrape_and_store(client, db_conn, ["4452"], skip_existing=False)

        assert (ok, errors) == (0, 1)
        rows = db_conn.execute(
            """
            SELECT period, statement, item_name, value, source
            FROM financial_items
            WHERE ticker = '4452'
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [
            ("0000-00", "_status", "ambiguous_annual_detail_link", None, "irbank_bs"),
        ]
