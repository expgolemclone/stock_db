from __future__ import annotations

import sqlite3

from stock_db.storage.sec_reports import (
    get_processed_doc_ids,
    get_sec_reports_for_ticker,
    upsert_sec_report,
)


class TestSecReports:
    def test_upsert_and_get_by_ticker(self, db_conn: sqlite3.Connection) -> None:
        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            file_path="var/raw/edinet/7203/2024.md",
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
            file_path="var/raw/edinet/7203/2024.md",
        )
        upsert_sec_report(
            db_conn,
            ticker="6501",
            fiscal_year="2024",
            doc_id="S100FGHIJ",
            file_path="var/raw/edinet/6501/2024.md",
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
            file_path="var/raw/edinet/7203/2024_v1.md",
            page_count=100,
        )
        db_conn.commit()

        upsert_sec_report(
            db_conn,
            ticker="7203",
            fiscal_year="2024",
            doc_id="S100ABCDE",
            file_path="var/raw/edinet/7203/2024_v2.md",
            page_count=150,
        )
        db_conn.commit()

        rows = get_sec_reports_for_ticker(db_conn, "7203")
        assert len(rows) == 1
        assert rows[0]["file_path"] == "var/raw/edinet/7203/2024_v2.md"
        assert rows[0]["page_count"] == 150

    def test_get_returns_empty_for_missing_ticker(self, db_conn: sqlite3.Connection) -> None:
        result = get_sec_reports_for_ticker(db_conn, "9999")

        assert result == []

    def test_get_processed_doc_ids_empty(self, db_conn: sqlite3.Connection) -> None:
        result = get_processed_doc_ids(db_conn)

        assert result == set()
