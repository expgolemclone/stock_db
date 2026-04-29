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
            file_path="var/raw/edinet/markdown/7203/2024.md",
            page_count=150,
            char_count=50000,
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")

        assert len(rows) == 1
        assert rows[0]["ticker"] == "7203"
        assert rows[0]["fiscal_year"] == "2024"
        assert rows[0]["doc_id"] == "S100ABCDE"
        assert rows[0]["page_count"] == 150
        assert rows[0]["char_count"] == 50000

    def test_get_processed_doc_ids(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            file_path="var/raw/edinet/markdown/7203/2024.md",
        )
        upsert_sec_report(
            db_conn,
            ticker="6501",
            fiscal_year="2024",
            doc_id="S100FGHIJ",
            file_path="var/raw/edinet/markdown/6501/2024.md",
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
            file_path="var/raw/edinet/markdown/7203/2024_v1.md",
            page_count=100,
        )
        db_conn.commit()

        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            file_path="var/raw/edinet/markdown/7203/2024_v2.md",
            page_count=150,
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")
        assert len(rows) == 1
        assert rows[0]["file_path"] == "var/raw/edinet/markdown/7203/2024_v2.md"
        assert rows[0]["page_count"] == 150

    def test_allows_same_doc_id_for_different_tickers(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="1111",
            fiscal_year="latest",
            doc_id="S100DUP01",
            file_path="var/raw/edinet/markdown/1111/latest.md",
        )
        upsert_sec_report(
            db_conn,
            ticker="2222",
            fiscal_year="latest",
            doc_id="S100DUP01",
            file_path="var/raw/edinet/markdown/2222/latest.md",
        )
        db_conn.commit()

        rows = db_conn.execute(
            """
            SELECT ticker, doc_id, file_path
            FROM sec_reports
            WHERE doc_id = 'S100DUP01'
            ORDER BY ticker
            """
        ).fetchall()

        assert [(row["ticker"], row["doc_id"], row["file_path"]) for row in rows] == [
            ("1111", "S100DUP01", "var/raw/edinet/markdown/1111/latest.md"),
            ("2222", "S100DUP01", "var/raw/edinet/markdown/2222/latest.md"),
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
            file_path="var/raw/edinet/markdown/1111/latest.md",
        )
        upsert_sec_report(
            db_conn,
            ticker="2222",
            fiscal_year="latest",
            doc_id="S100DUP01",
            file_path="var/raw/edinet/markdown/2222/latest.md",
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
            file_path="var/raw/edinet/markdown/7203/latest.md",
            xbrl_path="var/raw/edinet/markdown/xbrl/7203/S100XBR1.xhtml",
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")

        assert len(rows) == 1
        assert rows[0]["xbrl_path"] == "var/raw/edinet/markdown/xbrl/7203/S100XBR1.xhtml"

    def test_sync_edinet_raw_to_db_recovers_reports_and_urls(
        self, db_conn: sqlite3.Connection, tmp_path: Path,
    ) -> None:
        raw_dir = tmp_path / "var" / "raw" / "edinet"
        (raw_dir / "pdf" / "7203").mkdir(parents=True)
        (raw_dir / "xbrl" / "7203").mkdir(parents=True)
        (raw_dir / "markdown" / "7203").mkdir(parents=True)
        (raw_dir / "xbrl" / "6758").mkdir(parents=True)

        (raw_dir / "pdf" / "7203" / "S100ABCDE.pdf").write_bytes(b"%PDF-1.4")
        (raw_dir / "xbrl" / "7203" / "S100ABCDE.xhtml").write_text(
            "<html>7203</html>", encoding="utf-8",
        )
        markdown = "page1\n\npage2"
        (raw_dir / "markdown" / "7203" / "latest.md").write_text(markdown, encoding="utf-8")
        (raw_dir / "xbrl" / "6758" / "S100FGHIJ.xhtml").write_text(
            "<html>6758</html>", encoding="utf-8",
        )

        upsert_stock(db_conn, "7203", "Toyota", "Auto", "Prime")
        upsert_stock(db_conn, "6758", "Sony", "Electric", "Prime")
        db_conn.commit()

        synced_reports, synced_urls = sync_edinet_raw_to_db(db_conn, raw_dir)
        db_conn.commit()

        assert synced_reports == 2
        assert synced_urls == 2

        toyota_rows = get_sec_reports_for_ticker(db_conn, "7203")
        assert len(toyota_rows) == 1
        assert toyota_rows[0]["doc_id"] == "S100ABCDE"
        assert toyota_rows[0]["file_path"] == str((raw_dir / "markdown" / "7203" / "latest.md").resolve())
        assert toyota_rows[0]["xbrl_path"] == str((raw_dir / "xbrl" / "7203" / "S100ABCDE.xhtml").resolve())
        assert toyota_rows[0]["page_count"] == 2
        assert toyota_rows[0]["char_count"] == len(markdown)

        sony_rows = get_sec_reports_for_ticker(db_conn, "6758")
        assert len(sony_rows) == 1
        assert sony_rows[0]["doc_id"] == "S100FGHIJ"
        assert sony_rows[0]["file_path"] == ""
        assert sony_rows[0]["xbrl_path"] == str((raw_dir / "xbrl" / "6758" / "S100FGHIJ.xhtml").resolve())

        rows = db_conn.execute(
            "SELECT ticker, securities_report_url FROM stocks ORDER BY ticker",
        ).fetchall()
        assert [(row["ticker"], row["securities_report_url"]) for row in rows] == [
            ("6758", build_pdf_url("S100FGHIJ")),
            ("7203", build_pdf_url("S100ABCDE")),
        ]
