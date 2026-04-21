from __future__ import annotations

import sqlite3

from stock_db.storage._util import utc_now_iso


def upsert_sec_report(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    fiscal_year: str,
    doc_id: str,
    file_path: str,
    page_count: int | None = None,
    char_count: int | None = None,
    doc_type: str = "annual_report",
    source: str = "edinet",
) -> None:
    conn.execute(
        """
        INSERT INTO sec_reports
            (ticker, fiscal_year, doc_id, doc_type, file_path,
             page_count, char_count, source, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(doc_id) DO UPDATE SET
            ticker      = excluded.ticker,
            fiscal_year = excluded.fiscal_year,
            doc_type    = excluded.doc_type,
            file_path   = excluded.file_path,
            page_count  = excluded.page_count,
            char_count  = excluded.char_count,
            source      = excluded.source,
            updated_at  = excluded.updated_at
        """,
        (ticker, fiscal_year, doc_id, doc_type, file_path,
         page_count, char_count, source, utc_now_iso()),
    )


def get_processed_doc_ids(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute("SELECT doc_id FROM sec_reports").fetchall()
    return {r["doc_id"] for r in rows}


def get_sec_reports_for_ticker(
    conn: sqlite3.Connection,
    ticker: str,
) -> list[sqlite3.Row]:
    rows = conn.execute(
        """
        SELECT ticker, fiscal_year, doc_id, doc_type, file_path,
               page_count, char_count, source, updated_at
        FROM sec_reports
        WHERE ticker = ?
        ORDER BY fiscal_year DESC
        """,
        (ticker,),
    ).fetchall()
    return list(rows)
