from __future__ import annotations

import sqlite3
from dataclasses import dataclass
from pathlib import Path

from stock_db.sources.edinet.api_client import build_pdf_url
from stock_db.storage._util import utc_now_iso
from stock_db.storage.stocks import upsert_company_metadata


@dataclass
class _RawEdinetReport:
    ticker: str
    doc_id: str
    file_path: str = ""
    xbrl_path: str | None = None
    page_count: int | None = None
    char_count: int | None = None
    artifact_mtime_ns: int = 0


def upsert_sec_report(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    fiscal_year: str,
    doc_id: str,
    file_path: str,
    xbrl_path: str | None = None,
    page_count: int | None = None,
    char_count: int | None = None,
    doc_type: str = "annual_report",
    source: str = "edinet",
) -> None:
    conn.execute(
        """
        INSERT INTO sec_reports
            (ticker, fiscal_year, doc_id, doc_type, file_path, xbrl_path,
             page_count, char_count, source, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, doc_id) DO UPDATE SET
            fiscal_year = excluded.fiscal_year,
            doc_type    = excluded.doc_type,
            file_path   = excluded.file_path,
            xbrl_path   = excluded.xbrl_path,
            page_count  = excluded.page_count,
            char_count  = excluded.char_count,
            source      = excluded.source,
            updated_at  = excluded.updated_at
        """,
        (ticker, fiscal_year, doc_id, doc_type, file_path, xbrl_path,
         page_count, char_count, source, utc_now_iso()),
    )


def get_processed_doc_ids(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute("SELECT doc_id FROM sec_reports").fetchall()
    return {r["doc_id"] for r in rows}


def get_processed_report_keys(conn: sqlite3.Connection) -> set[tuple[str, str]]:
    rows = conn.execute("SELECT ticker, doc_id FROM sec_reports").fetchall()
    return {(r["ticker"], r["doc_id"]) for r in rows}


def get_processed_xbrl_report_keys(conn: sqlite3.Connection) -> set[tuple[str, str]]:
    rows = conn.execute(
        "SELECT ticker, doc_id FROM sec_reports WHERE xbrl_path IS NOT NULL"
    ).fetchall()
    return {(r["ticker"], r["doc_id"]) for r in rows}


def get_sec_reports_for_ticker(
    conn: sqlite3.Connection,
    ticker: str,
) -> list[sqlite3.Row]:
    rows = conn.execute(
        """
        SELECT ticker, fiscal_year, doc_id, doc_type, file_path, xbrl_path,
               page_count, char_count, source, updated_at
        FROM sec_reports
        WHERE ticker = ?
        ORDER BY fiscal_year DESC
        """,
        (ticker,),
    ).fetchall()
    return list(rows)


def _discover_raw_edinet_reports(raw_dir: Path) -> list[_RawEdinetReport]:
    reports: dict[tuple[str, str], _RawEdinetReport] = {}
    by_ticker: dict[str, list[_RawEdinetReport]] = {}

    for path in sorted((raw_dir / "pdf").glob("*/*.pdf")):
        ticker = path.parent.name
        doc_id = path.stem
        key = (ticker, doc_id)
        report = reports.get(key)
        if report is None:
            report = _RawEdinetReport(ticker=ticker, doc_id=doc_id)
            reports[key] = report
            by_ticker.setdefault(ticker, []).append(report)
        report.artifact_mtime_ns = max(report.artifact_mtime_ns, path.stat().st_mtime_ns)

    for path in sorted((raw_dir / "xbrl").glob("*/*.xhtml")):
        ticker = path.parent.name
        doc_id = path.stem
        key = (ticker, doc_id)
        report = reports.get(key)
        if report is None:
            report = _RawEdinetReport(ticker=ticker, doc_id=doc_id)
            reports[key] = report
            by_ticker.setdefault(ticker, []).append(report)
        report.xbrl_path = str(path.resolve())
        report.artifact_mtime_ns = max(report.artifact_mtime_ns, path.stat().st_mtime_ns)

    for ticker_dir in sorted(raw_dir.iterdir()):
        if not ticker_dir.is_dir() or ticker_dir.name in {"pdf", "xbrl"}:
            continue
        md_path = ticker_dir / "latest.md"
        if not md_path.is_file():
            continue
        candidates = by_ticker.get(ticker_dir.name)
        if not candidates:
            continue
        target = max(candidates, key=lambda report: (report.artifact_mtime_ns, report.doc_id))
        markdown = md_path.read_text(encoding="utf-8")
        target.file_path = str(md_path.resolve())
        target.page_count = len(markdown.split("\n\n"))
        target.char_count = len(markdown)
        target.artifact_mtime_ns = max(target.artifact_mtime_ns, md_path.stat().st_mtime_ns)

    return sorted(reports.values(), key=lambda report: (report.ticker, report.doc_id))


def sync_edinet_raw_to_db(
    conn: sqlite3.Connection,
    raw_dir: Path,
) -> tuple[int, int]:
    reports = _discover_raw_edinet_reports(raw_dir)
    existing_reports = {
        (row["ticker"], row["doc_id"]): row
        for row in conn.execute(
            """
            SELECT ticker, doc_id, file_path, xbrl_path, page_count, char_count
            FROM sec_reports
            """
        ).fetchall()
    }
    existing_urls = {
        row["ticker"]: row["securities_report_url"]
        for row in conn.execute(
            """
            SELECT ticker, securities_report_url
            FROM stocks
            WHERE securities_report_url IS NOT NULL
            """
        ).fetchall()
    }
    stock_tickers = {
        row["ticker"]
        for row in conn.execute("SELECT ticker FROM stocks").fetchall()
    }

    synced_reports = 0
    synced_urls = 0

    for report in reports:
        current = existing_reports.get((report.ticker, report.doc_id))
        if current is None or (
            current["ticker"] != report.ticker
            or current["file_path"] != report.file_path
            or current["xbrl_path"] != report.xbrl_path
            or current["page_count"] != report.page_count
            or current["char_count"] != report.char_count
        ):
            upsert_sec_report(
                conn,
                ticker=report.ticker,
                fiscal_year="latest",
                doc_id=report.doc_id,
                file_path=report.file_path,
                xbrl_path=report.xbrl_path,
                page_count=report.page_count,
                char_count=report.char_count,
            )
            synced_reports += 1

        if report.ticker not in stock_tickers:
            continue
        url = build_pdf_url(report.doc_id)
        if existing_urls.get(report.ticker) != url:
            upsert_company_metadata(conn, report.ticker, securities_report_url=url)
            existing_urls[report.ticker] = url
            synced_urls += 1

    return synced_reports, synced_urls
