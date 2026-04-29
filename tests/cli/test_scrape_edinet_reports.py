from __future__ import annotations

import sqlite3
import threading
import time
from pathlib import Path

import pytest

from stock_db.cli import scrape_edinet_reports
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_company_metadata, upsert_stock


class _FakePdfResponse:
    def __init__(self, body: bytes = b"%PDF-1.4 fake") -> None:
        self._body = body

    def raise_for_status(self) -> None:
        return None

    def iter_content(self, chunk_size: int) -> list[bytes]:
        del chunk_size
        return [self._body]


class _EvaluateClient:
    def __init__(self, request_times: list[float], lock: threading.Lock) -> None:
        self._request_times = request_times
        self._lock = lock

    def evaluate(
        self,
        url: str,
        script: str,
        *,
        proxy: str | None = None,
        timeout: int | None = None,
    ) -> str:
        del url, script, proxy, timeout
        with self._lock:
            self._request_times.append(time.monotonic())
        return "<html><body>" + ("x" * 200) + "</body></html>"


def _build_db(db_path: Path) -> sqlite3.Connection:
    conn = get_connection(db_path)
    init_db(conn)
    return conn


class TestRequestThrottle:
    def test_wait_enforces_shared_min_interval(self, monkeypatch: pytest.MonkeyPatch) -> None:
        now = {"value": 100.0}
        sleeps: list[float] = []

        def fake_monotonic() -> float:
            return now["value"]

        def fake_sleep(delay: float) -> None:
            sleeps.append(delay)
            now["value"] += delay

        monkeypatch.setattr(scrape_edinet_reports.time, "monotonic", fake_monotonic)
        monkeypatch.setattr(scrape_edinet_reports.time, "sleep", fake_sleep)

        throttle = scrape_edinet_reports._RequestThrottle(2.0)
        throttle.wait()
        now["value"] += 0.5
        throttle.wait()
        now["value"] += 0.5
        throttle.wait()

        assert sleeps == [1.5, 1.5]


class TestScrapeAllEdinetReports:
    def test_phase2_persists_reports_without_cross_thread_db_access(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()

        for ticker, doc_id in [("1111", "S100AAA1"), ("2222", "S100BBB2")]:
            upsert_stock(conn, ticker, f"Name {ticker}", "Sector", "Prime")
            upsert_company_metadata(
                conn, ticker,
                securities_report_url=f"https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/{doc_id}.pdf",
            )
        conn.commit()

        def fake_get(url: str, *, timeout: float, stream: bool) -> _FakePdfResponse:
            del url, timeout, stream
            with request_lock:
                request_times.append(time.monotonic())
            return _FakePdfResponse()

        monkeypatch.setattr("stock_db.sources.edinet.api_client.requests.get", fake_get)
        monkeypatch.setattr(
            scrape_edinet_reports,
            "extract_markdown",
            lambda pdf_path: f"markdown for {Path(pdf_path).stem}",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )

        ok, errors = scrape_edinet_reports.scrape_all_edinet_reports(
            conn,
            _EvaluateClient(request_times, request_lock),
            ["1111", "2222"],
            skip_existing=False,
            interval=0.03,
        )

        assert ok == 2
        assert errors == 0

        rows = conn.execute(
            """
            SELECT ticker, doc_id, file_path, xbrl_path, page_count, char_count
            FROM sec_reports
            ORDER BY ticker
            """
        ).fetchall()
        assert [row["ticker"] for row in rows] == ["1111", "2222"]
        assert all(Path(row["file_path"]).is_file() for row in rows)
        assert all(Path(str(row["xbrl_path"])).is_file() for row in rows)
        assert all(row["page_count"] == 1 for row in rows)
        assert all(row["char_count"] > 0 for row in rows)

        diffs = [
            current - previous
            for previous, current in zip(sorted(request_times), sorted(request_times)[1:])
        ]
        assert len(request_times) == 4
        assert all(diff >= 0.02 for diff in diffs)
        conn.close()

    def test_skip_existing_reuses_saved_markdown_metadata_for_xbrl_only(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        raw_dir = tmp_path / "var" / "raw" / "edinet"
        md_path = raw_dir / "3333" / "latest.md"
        md_path.parent.mkdir(parents=True, exist_ok=True)
        md_path.write_text("existing markdown", encoding="utf-8")

        upsert_stock(conn, "3333", "Name 3333", "Sector", "Prime")
        upsert_company_metadata(
            conn,
            "3333",
            securities_report_url="https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100CCC3.pdf",
        )
        upsert_sec_report(
            conn,
            ticker="3333",
            fiscal_year="latest",
            doc_id="S100CCC3",
            file_path=str(md_path.resolve()),
            xbrl_path=None,
            page_count=7,
            char_count=77,
        )
        conn.commit()

        def fail_get(*args: object, **kwargs: object) -> _FakePdfResponse:
            raise AssertionError("PDF should not be downloaded when skip_pdf=True")

        monkeypatch.setattr("stock_db.sources.edinet.api_client.requests.get", fail_get)
        monkeypatch.setattr(scrape_edinet_reports, "_EDINET_RAW_DIR", raw_dir)

        ok, errors = scrape_edinet_reports.scrape_all_edinet_reports(
            conn,
            _EvaluateClient([], threading.Lock()),
            ["3333"],
            skip_existing=True,
            interval=0.0,
        )

        assert ok == 1
        assert errors == 0

        row = conn.execute(
            """
            SELECT file_path, xbrl_path, page_count, char_count
            FROM sec_reports
            WHERE doc_id = 'S100CCC3'
            """
        ).fetchone()
        assert row["file_path"] == str(md_path.resolve())
        assert row["xbrl_path"] is not None
        assert Path(row["xbrl_path"]).is_file()
        assert row["page_count"] == 7
        assert row["char_count"] == 77
        conn.close()

    def test_skip_existing_is_scoped_per_ticker_not_global_doc_id(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()

        upsert_stock(conn, "1111", "Current", "Sector", "Prime")
        upsert_company_metadata(
            conn,
            "1111",
            securities_report_url="https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100DUP1.pdf",
        )
        upsert_stock(conn, "9999", "Legacy", "Sector", "Prime")
        upsert_sec_report(
            conn,
            ticker="9999",
            fiscal_year="latest",
            doc_id="S100DUP1",
            file_path=str((tmp_path / "legacy.md").resolve()),
            xbrl_path=str((tmp_path / "legacy.xhtml").resolve()),
            page_count=3,
            char_count=30,
        )
        conn.commit()

        def fake_get(url: str, *, timeout: float, stream: bool) -> _FakePdfResponse:
            del url, timeout, stream
            with request_lock:
                request_times.append(time.monotonic())
            return _FakePdfResponse()

        monkeypatch.setattr("stock_db.sources.edinet.api_client.requests.get", fake_get)
        monkeypatch.setattr(
            scrape_edinet_reports,
            "extract_markdown",
            lambda pdf_path: f"markdown for {Path(pdf_path).stem}",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )

        ok, errors = scrape_edinet_reports.scrape_all_edinet_reports(
            conn,
            _EvaluateClient(request_times, request_lock),
            ["1111"],
            skip_existing=True,
            interval=0.0,
        )

        assert ok == 1
        assert errors == 0

        rows = conn.execute(
            """
            SELECT ticker, doc_id, file_path, xbrl_path
            FROM sec_reports
            WHERE doc_id = 'S100DUP1'
            ORDER BY ticker
            """
        ).fetchall()
        assert [(row["ticker"], row["doc_id"]) for row in rows] == [
            ("1111", "S100DUP1"),
            ("9999", "S100DUP1"),
        ]
        assert Path(rows[0]["file_path"]).is_file()
        assert Path(rows[0]["xbrl_path"]).is_file()
        conn.close()
