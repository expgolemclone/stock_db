from __future__ import annotations

import sqlite3
import threading
import time
from pathlib import Path

import pytest

from stock_db.cli import scrape_edinet_reports
from stock_db.cli import scrape_edinet_reports_step1, scrape_edinet_reports_step2
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_company_metadata, upsert_stock


def _valid_ixbrl_html(*, inventory_value: str = "500") -> str:
    return (
        '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL"><head></head><body>'
        '<ix:nonnumeric contextref="FilingDateInstant" '
        'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
        '<ix:nonfraction contextref="CurrentYearInstant" '
        f'name="jppfs_cor:Inventories">{inventory_value}</ix:nonfraction>'
        "</body></html>"
    )


class _EvaluateClient:
    def __init__(
        self,
        request_times: list[float],
        lock: threading.Lock,
        *,
        html: str | None = None,
    ) -> None:
        self._request_times = request_times
        self._lock = lock
        self._html = html or _valid_ixbrl_html()

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
        return self._html


def _build_db(db_path: Path) -> sqlite3.Connection:
    conn = get_connection(db_path)
    init_db(conn)
    return conn


def _fake_download_xbrl_package_factory(
    request_times: list[float],
    lock: threading.Lock,
    *,
    html: str | None = None,
    raise_error: bool = False,
):
    payload = html or _valid_ixbrl_html()

    def _fake_download_xbrl_package(
        doc_id: str,
        dest_dir: Path,
        *,
        api_key: str,
        before_request: object = None,
        timeout: int = 300,
    ) -> Path:
        del api_key, timeout
        if before_request is not None:
            before_request()
        with lock:
            request_times.append(time.monotonic())
        if raise_error:
            raise scrape_edinet_reports.EdinetApiError("invalid package")
        artifact_dir = dest_dir / doc_id
        public_doc = artifact_dir / "XBRL" / "PublicDoc"
        public_doc.mkdir(parents=True, exist_ok=True)
        (dest_dir / f"{doc_id}.zip").write_bytes(b"zip")
        (public_doc / "report.xhtml").write_text(payload, encoding="utf-8")
        return artifact_dir

    return _fake_download_xbrl_package


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
    def test_phase1_only_discovers_urls(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        upsert_stock(conn, "1111", "Alpha", "Sector", "Prime")
        conn.commit()

        monkeypatch.setattr(
            scrape_edinet_reports,
            "search_annual_reports",
            lambda client, ticker, **kwargs: ("S100STEP1", "E12345"),
        )

        result = scrape_edinet_reports.scrape_edinet_phase1(
            conn,
            _EvaluateClient([], threading.Lock()),
            ["1111"],
            interval=0.0,
        )

        stock = conn.execute(
            """
            SELECT edinet_code, securities_report_url
            FROM stocks
            WHERE ticker = '1111'
            """
        ).fetchone()
        report_count = conn.execute("SELECT COUNT(*) FROM sec_reports").fetchone()[0]

        assert result == scrape_edinet_reports._Phase1Result(
            searched=1,
            found=1,
            not_found=0,
            errors=0,
        )
        assert stock["edinet_code"] == "E12345"
        assert stock["securities_report_url"].endswith("/S100STEP1.pdf")
        assert report_count == 0
        conn.close()

    def test_phase1_retries_with_yahoo_company_name_when_initial_name_misses(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        upsert_stock(conn, "8306", "三菱UFJ FG", "Sector", "Prime")
        conn.commit()

        search_calls: list[str | None] = []

        def fake_search_annual_reports(
            client: object,
            ticker: str,
            *,
            proxy: str | None = None,
            edinet_code: str | None = None,
            company_name: str | None = None,
            before_request: object = None,
        ) -> tuple[str | None, str | None]:
            del client, proxy, edinet_code, before_request
            assert ticker == "8306"
            search_calls.append(company_name)
            if company_name == "株式会社三菱ＵＦＪフィナンシャル・グループ":
                return "S100W4FB", "E03606"
            return None, None

        monkeypatch.setattr(
            scrape_edinet_reports,
            "search_annual_reports",
            fake_search_annual_reports,
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "discover_company_name",
            lambda client, ticker, **kwargs: ("株式会社三菱ＵＦＪフィナンシャル・グループ", "T"),
        )

        result = scrape_edinet_reports.scrape_edinet_phase1(
            conn,
            _EvaluateClient([], threading.Lock()),
            ["8306"],
            interval=0.0,
        )

        stock = conn.execute(
            """
            SELECT name, edinet_code, yf_suffix, securities_report_url
            FROM stocks
            WHERE ticker = '8306'
            """
        ).fetchone()

        assert result == scrape_edinet_reports._Phase1Result(
            searched=1,
            found=1,
            not_found=0,
            errors=0,
        )
        assert search_calls == [
            "三菱UFJ FG",
            "株式会社三菱ＵＦＪフィナンシャル・グループ",
        ]
        assert stock["name"] == "株式会社三菱ＵＦＪフィナンシャル・グループ"
        assert stock["edinet_code"] == "E03606"
        assert stock["yf_suffix"] == "T"
        assert stock["securities_report_url"].endswith("/S100W4FB.pdf")
        conn.close()

    def test_combined_run_reloads_urls_discovered_in_phase1(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()

        upsert_stock(conn, "1111", "Alpha", "Sector", "Prime")
        conn.commit()

        monkeypatch.setattr(
            scrape_edinet_reports,
            "search_annual_reports",
            lambda client, ticker, **kwargs: ("S100STEP12", "E54321"),
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(request_times, request_lock),
        )

        ok, errors = scrape_edinet_reports.scrape_all_edinet_reports(
            conn,
            _EvaluateClient(request_times, request_lock),
            ["1111"],
            skip_existing=False,
            interval=0.0,
        )

        row = conn.execute(
            """
            SELECT ticker, doc_id, xbrl_path
            FROM sec_reports
            WHERE ticker = '1111'
            """
        ).fetchone()

        assert ok == 1
        assert errors == 0
        assert row["doc_id"] == "S100STEP12"
        assert Path(row["xbrl_path"]).is_dir()
        assert Path(row["xbrl_path"]).joinpath("XBRL", "PublicDoc", "report.xhtml").is_file()
        conn.close()

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

        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(request_times, request_lock),
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
            SELECT ticker, doc_id, xbrl_path
            FROM sec_reports
            ORDER BY ticker
            """
        ).fetchall()
        assert [row["ticker"] for row in rows] == ["1111", "2222"]
        assert all(Path(str(row["xbrl_path"])).is_dir() for row in rows)
        assert all((Path(str(row["xbrl_path"])).parent / f"{row['doc_id']}.zip").is_file() for row in rows)

        diffs = [
            current - previous
            for previous, current in zip(sorted(request_times), sorted(request_times)[1:])
        ]
        assert len(request_times) == 2
        assert all(diff >= 0.02 for diff in diffs)
        conn.close()

    def test_skip_existing_skips_fully_processed_report(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        raw_dir = tmp_path / "var" / "raw" / "edinet"
        xbrl_path = raw_dir / "xbrl" / "3333" / "S100CCC3.xhtml"
        xbrl_path.parent.mkdir(parents=True, exist_ok=True)
        xbrl_path.write_text(_valid_ixbrl_html(inventory_value="600"), encoding="utf-8")

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
            xbrl_path=str(xbrl_path.resolve()),
        )
        conn.commit()

        monkeypatch.setattr(scrape_edinet_reports, "_EDINET_RAW_DIR", raw_dir)

        ok, errors = scrape_edinet_reports.scrape_all_edinet_reports(
            conn,
            _EvaluateClient([], threading.Lock()),
            ["3333"],
            skip_existing=True,
            interval=0.0,
        )

        assert ok == 0
        assert errors == 0

        row = conn.execute(
            """
            SELECT xbrl_path
            FROM sec_reports
            WHERE doc_id = 'S100CCC3'
            """
        ).fetchone()
        assert row["xbrl_path"] is not None
        assert Path(row["xbrl_path"]).is_file()
        conn.close()

    def test_phase2_skips_tickers_without_url(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()

        upsert_stock(conn, "1111", "No URL", "Sector", "Prime")
        upsert_stock(conn, "2222", "Has URL", "Sector", "Prime")
        upsert_company_metadata(
            conn,
            "2222",
            securities_report_url="https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100STEP2.pdf",
        )
        conn.commit()

        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(request_times, request_lock),
        )

        result = scrape_edinet_reports.scrape_edinet_phase2(
            conn,
            _EvaluateClient(request_times, request_lock),
            ["1111", "2222"],
            skip_existing=False,
            interval=0.0,
        )

        rows = conn.execute(
            """
            SELECT ticker, doc_id
            FROM sec_reports
            ORDER BY ticker
            """
        ).fetchall()

        assert result == scrape_edinet_reports._Phase2Result(
            ok=1,
            errors=0,
            xbrl_failures=0,
            skipped_missing_url=1,
        )
        assert [(row["ticker"], row["doc_id"]) for row in rows] == [("2222", "S100STEP2")]
        conn.close()

    def test_skip_existing_refreshes_invalid_saved_xbrl(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()
        raw_dir = tmp_path / "var" / "raw" / "edinet"
        xbrl_path = raw_dir / "xbrl" / "4444" / "S100BADX.xhtml"
        xbrl_path.parent.mkdir(parents=True, exist_ok=True)
        xbrl_path.write_text("<html><body>header only</body></html>", encoding="utf-8")

        upsert_stock(conn, "4444", "Name 4444", "Sector", "Prime")
        upsert_company_metadata(
            conn,
            "4444",
            securities_report_url="https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100BADX.pdf",
        )
        upsert_sec_report(
            conn,
            ticker="4444",
            fiscal_year="latest",
            doc_id="S100BADX",
            xbrl_path=str(xbrl_path.resolve()),
        )
        conn.commit()

        monkeypatch.setattr(scrape_edinet_reports, "_EDINET_RAW_DIR", raw_dir)
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(
                request_times,
                request_lock,
                html=_valid_ixbrl_html(inventory_value="700"),
            ),
        )

        result = scrape_edinet_reports.scrape_edinet_phase2(
            conn,
            _EvaluateClient(request_times, request_lock, html=_valid_ixbrl_html(inventory_value="700")),
            ["4444"],
            skip_existing=True,
            interval=0.0,
        )

        row = conn.execute(
            """
            SELECT xbrl_path
            FROM sec_reports
            WHERE ticker = '4444'
            """
        ).fetchone()

        assert result == scrape_edinet_reports._Phase2Result(
            ok=1,
            errors=0,
            xbrl_failures=0,
            skipped_missing_url=0,
        )
        assert Path(row["xbrl_path"]).is_dir()
        saved = Path(row["xbrl_path"]) / "XBRL" / "PublicDoc" / "report.xhtml"
        assert _valid_ixbrl_html(inventory_value="700") in saved.read_text(encoding="utf-8")
        conn.close()

    def test_phase2_counts_invalid_xbrl_payload_as_failure(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        conn = _build_db(db_path)
        request_times: list[float] = []
        request_lock = threading.Lock()

        upsert_stock(conn, "5555", "Name 5555", "Sector", "Prime")
        upsert_company_metadata(
            conn,
            "5555",
            securities_report_url="https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100FAILX.pdf",
        )
        conn.commit()

        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(
                request_times,
                request_lock,
                raise_error=True,
            ),
        )

        result = scrape_edinet_reports.scrape_edinet_phase2(
            conn,
            _EvaluateClient(
                request_times,
                request_lock,
                html="<html><head><title>0000000_header.htm</title></head><body>header</body></html>",
            ),
            ["5555"],
            skip_existing=False,
            interval=0.0,
        )

        row = conn.execute(
            """
            SELECT xbrl_path
            FROM sec_reports
            WHERE ticker = '5555'
            """
        ).fetchone()

        assert result == scrape_edinet_reports._Phase2Result(
            ok=1,
            errors=0,
            xbrl_failures=1,
            skipped_missing_url=0,
        )
        assert row["xbrl_path"] is None
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
            xbrl_path=str((tmp_path / "legacy.xhtml").resolve()),
        )
        conn.commit()

        monkeypatch.setattr(
            scrape_edinet_reports,
            "_EDINET_RAW_DIR",
            tmp_path / "var" / "raw" / "edinet",
        )
        monkeypatch.setattr(
            scrape_edinet_reports,
            "download_xbrl_package",
            _fake_download_xbrl_package_factory(request_times, request_lock),
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
            SELECT ticker, doc_id, xbrl_path
            FROM sec_reports
            WHERE doc_id = 'S100DUP1'
            ORDER BY ticker
            """
        ).fetchall()
        assert [(row["ticker"], row["doc_id"]) for row in rows] == [
            ("1111", "S100DUP1"),
            ("9999", "S100DUP1"),
        ]
        assert Path(rows[0]["xbrl_path"]).is_dir()
        conn.close()


class TestStepCliWrappers:
    def test_step1_wrapper_delegates_to_main_step1(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        called: dict[str, object] = {}

        def fake_main_step1(argv: object = None) -> int:
            called["argv"] = argv
            return 7

        monkeypatch.setattr(scrape_edinet_reports_step1, "main_step1", fake_main_step1)

        rc = scrape_edinet_reports_step1.main(["--ticker", "7203"])

        assert rc == 7
        assert called["argv"] == ["--ticker", "7203"]

    def test_step2_wrapper_delegates_to_main_step2(
        self, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        called: dict[str, object] = {}

        def fake_main_step2(argv: object = None) -> int:
            called["argv"] = argv
            return 9

        monkeypatch.setattr(scrape_edinet_reports_step2, "main_step2", fake_main_step2)

        rc = scrape_edinet_reports_step2.main(["--ticker", "7203", "--force"])

        assert rc == 9
        assert called["argv"] == ["--ticker", "7203", "--force"]
