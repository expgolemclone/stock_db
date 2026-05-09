from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db.sources.edinet.api_client import build_pdf_url
from stock_db.storage.sec_reports import (
    get_processed_doc_ids,
    get_processed_report_keys,
    get_sec_reports_for_ticker,
    sync_edinet_raw_to_db,
    upsert_sec_report,
)
from stock_db.storage.stocks import upsert_stock


class TestSecReports:
    def test_upsert_and_get_by_ticker(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")

        assert len(rows) == 1
        assert rows[0]["ticker"] == "7203"
        assert rows[0]["fiscal_year"] == "2024"
        assert rows[0]["doc_id"] == "S100ABCDE"

    def test_get_processed_doc_ids(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
        )
        upsert_sec_report(
            db_conn,
            ticker="6501",
            fiscal_year="2024",
            doc_id="S100FGHIJ",
        )
        db_conn.commit()

        result = get_processed_doc_ids(db_conn)

        assert result == {"S100ABCDE", "S100FGHIJ"}

    def test_upsert_replaces_existing_doc(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            xbrl_path="/old/path.xhtml",
        )
        db_conn.commit()

        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            xbrl_path="/new/path.xhtml",
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")
        assert len(rows) == 1
        assert rows[0]["xbrl_path"] == "/new/path.xhtml"

    def test_allows_same_doc_id_for_different_tickers(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="1111",
            fiscal_year="latest",
            doc_id="S100DUP01",
        )
        upsert_sec_report(
            db_conn,
            ticker="2222",
            fiscal_year="latest",
            doc_id="S100DUP01",
        )
        db_conn.commit()

        rows = db_conn.execute(
            """
            SELECT ticker, doc_id
            FROM sec_reports
            WHERE doc_id = 'S100DUP01'
            ORDER BY ticker
            """
        ).fetchall()

        assert [(row["ticker"], row["doc_id"]) for row in rows] == [
            ("1111", "S100DUP01"),
            ("2222", "S100DUP01"),
        ]

    def test_get_returns_empty_for_missing_ticker(self, db_conn: sqlite3.Connection) -> None:
        result = get_sec_reports_for_ticker(db_conn, "9999")

        assert result == []

    def test_get_processed_doc_ids_empty(self, db_conn: sqlite3.Connection) -> None:
        result = get_processed_doc_ids(db_conn)

        assert result == set()

    def test_get_processed_report_keys(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="1111",
            fiscal_year="latest",
            doc_id="S100DUP01",
        )
        upsert_sec_report(
            db_conn,
            ticker="2222",
            fiscal_year="latest",
            doc_id="S100DUP01",
        )
        db_conn.commit()

        assert get_processed_report_keys(db_conn) >= {
            ("1111", "S100DUP01"),
            ("2222", "S100DUP01"),
        }

    def test_upsert_with_xbrl_path(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="latest",
            doc_id="S100XBR1",
            xbrl_path="var/raw/edinet/xbrl/7203/S100XBR1",
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")

        assert len(rows) == 1
        assert rows[0]["xbrl_path"] == "var/raw/edinet/xbrl/7203/S100XBR1"

    def test_sync_edinet_raw_to_db_recovers_reports_and_urls(
        self, db_conn: sqlite3.Connection, tmp_path: Path,
    ) -> None:
        raw_dir = tmp_path / "var" / "raw" / "edinet"
        (raw_dir / "xbrl" / "7203").mkdir(parents=True)
        (raw_dir / "xbrl" / "6758").mkdir(parents=True)

        (raw_dir / "xbrl" / "7203" / "S100ABCDE").mkdir()
        (raw_dir / "xbrl" / "7203" / "S100ABCDE" / "XBRL").mkdir()
        (raw_dir / "xbrl" / "7203" / "S100ABCDE" / "XBRL" / "report.xhtml").write_text(
            "<html>7203</html>", encoding="utf-8",
        )
        (raw_dir / "xbrl" / "7203" / "S100ABCDE.zip").write_bytes(b"zip")
        (raw_dir / "xbrl" / "6758" / "S100FGHIJ.xhtml").write_text(
            "<html>6758</html>", encoding="utf-8",
        )

        upsert_stock(db_conn, "7203", "Toyota", "Auto", "Prime")
        upsert_stock(db_conn, "6758", "Sony", "Electric", "Prime")
        db_conn.commit()

        synced_reports, synced_urls = sync_edinet_raw_to_db(db_conn, raw_dir)
        db_conn.commit()

        assert synced_reports == 1
        assert synced_urls == 1

        toyota_rows = get_sec_reports_for_ticker(db_conn, "7203")
        assert len(toyota_rows) == 1
        assert toyota_rows[0]["doc_id"] == "S100ABCDE"
        assert toyota_rows[0]["xbrl_path"] == str((raw_dir / "xbrl" / "7203" / "S100ABCDE").resolve())

        sony_rows = get_sec_reports_for_ticker(db_conn, "6758")
        assert sony_rows == []

        rows = db_conn.execute(
            "SELECT ticker, securities_report_url FROM stocks ORDER BY ticker",
        ).fetchall()
        assert [(row["ticker"], row["securities_report_url"]) for row in rows] == [
            ("6758", None),
            ("7203", build_pdf_url("S100ABCDE")),
        ]
