from __future__ import annotations

import sqlite3

from stock_db.sources.irbank.bs_scraper import scrape_and_store
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.stocks import get_existing_tickers, upsert_stock


class _FakeClient:
    def __init__(self, responses: dict[str, object]) -> None:
        self._responses = responses

    def fetch(self, url: str, *, proxy: str | None = None) -> object:
        ticker = url.rstrip("/").split("/")[-2]
        return self._responses[ticker]


class _SequenceClient:
    def __init__(self, responses: dict[str, list[object]]) -> None:
        self._responses = {ticker: list(items) for ticker, items in responses.items()}

    def fetch(self, url: str, *, proxy: str | None = None) -> object:
        ticker = url.rstrip("/").split("/")[-2]
        items = self._responses[ticker]
        if len(items) == 1:
            return items[0]
        return items.pop(0)


class _FakeResponse:
    def __init__(self, html: str | None, status: int = 200, error: str | None = None) -> None:
        self.html = html
        self.status = status
        self.error = error


def _overview_html() -> str:
    return """
    <html><body>
      <table>
        <tr><th>年</th><th>固定資産</th><th>流動資産</th><th>純資産</th><th>固定負債</th><th>流動負債</th></tr>
        <tr><td>2024年3月 借方</td><td>40% 40億</td><td>60% 60億</td><td>0%</td><td>0%</td><td>0%</td></tr>
        <tr><td>2024年3月 貸方</td><td>0%</td><td>0%</td><td>50% 50億</td><td>20% 20億</td><td>30% 30億</td></tr>
      </table>
      <table>
        <tr><th>年度</th><th>現金等</th><th>売上債権</th><th>たな卸資産</th></tr>
        <tr><td>2024年3月</td><td>10億</td><td>20億</td><td>30億</td></tr>
      </table>
    </body></html>
    """


class TestIrbankBsScraper:
    def test_empty_parse_is_marked_as_no_data(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7699", "No Data Inc.", "", "")
        db_conn.commit()

        client = _FakeClient({"7699": _FakeResponse("<html><body><h1>財務状況</h1></body></html>")})

        ok, errors = scrape_and_store(client, db_conn, ["7699"], skip_existing=False)

        assert (ok, errors) == (1, 0)
        rows = db_conn.execute(
            """
            SELECT period, statement, item_name, value, source
            FROM financial_items
            WHERE ticker = '7699'
            """
        ).fetchall()
        assert [tuple(row) for row in rows] == [
            ("0000-00", "_status", "no_data", None, "irbank_bs"),
        ]
        assert get_existing_tickers(db_conn, "irbank_bs") == {"7699"}

    def test_transient_fetch_error_retries_and_succeeds(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7203", "Retry Corp.", "", "")
        db_conn.commit()

        client = _SequenceClient({
            "7203": [
                _FakeResponse(
                    None,
                    error="net::ERR_INTERNET_DISCONNECTED at https://irbank.net/7203/bs",
                ),
                _FakeResponse(_overview_html()),
            ],
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

    def test_force_refresh_replaces_existing_source_rows(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "7203", "Test Corp.", "", "")
        upsert_financial_item(db_conn, "7203", "2024-03", "bs", "legacy_metric", 1.0, "irbank_bs")
        db_conn.commit()

        client = _FakeClient({"7203": _FakeResponse(_overview_html())})

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
        assert ("bs", "inventories", 3_000_000_000.0) in [tuple(row) for row in rows]

    def test_fetch_error_is_recorded_and_retried_on_future_default_runs(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_stock(db_conn, "9914", "Retry Later Corp.", "", "")
        db_conn.commit()

        failing_client = _FakeClient({
            "9914": _FakeResponse(
                None,
                error="net::ERR_INTERNET_DISCONNECTED at https://irbank.net/9914/bs",
            ),
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
            ("0000-00", "_status", "fetch_error.net_err_internet_disconnected"),
        ]

        succeeding_client = _FakeClient({"9914": _FakeResponse(_overview_html())})
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
        assert ("_status", "fetch_error.net_err_internet_disconnected") not in [tuple(row) for row in rows]
        assert ("bs", "inventories") in [tuple(row) for row in rows]

    def test_skip_existing_treats_status_row_as_processed(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "9257", "Status Only Ltd.", "", "")
        upsert_financial_item(db_conn, "9257", "0000-00", "_status", "no_data", None, "irbank_bs")
        db_conn.commit()

        client = _FakeClient({"9257": _FakeResponse(_overview_html())})

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
