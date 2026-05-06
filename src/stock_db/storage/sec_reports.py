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
    xbrl_path: str | None = None
    artifact_mtime_ns: int = 0


def _max_mtime_ns(path: Path) -> int:
    max_mtime = path.stat().st_mtime_ns
    if path.is_dir():
        for child in path.rglob("*"):
            max_mtime = max(max_mtime, child.stat().st_mtime_ns)
    return max_mtime


def upsert_sec_report(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    fiscal_year: str,
    doc_id: str,
    xbrl_path: str | None = None,
    doc_type: str = "annual_report",
    source: str = "edinet",
) -> None:
    conn.execute(
        """
        INSERT INTO sec_reports
            (ticker, fiscal_year, doc_id, doc_type, xbrl_path,
             source, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, doc_id) DO UPDATE SET
            fiscal_year = excluded.fiscal_year,
            doc_type    = excluded.doc_type,
            xbrl_path   = excluded.xbrl_path,
            source      = excluded.source,
            updated_at  = excluded.updated_at
        """,
        (ticker, fiscal_year, doc_id, doc_type, xbrl_path,
         source, utc_now_iso()),
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
        SELECT ticker, fiscal_year, doc_id, doc_type, xbrl_path,
               source, updated_at
        FROM sec_reports
        WHERE ticker = ?
        ORDER BY fiscal_year DESC
        """,
        (ticker,),
    ).fetchall()
    return list(rows)


def _discover_raw_edinet_reports(raw_dir: Path) -> list[_RawEdinetReport]:
    reports: dict[tuple[str, str], _RawEdinetReport] = {}

    xbrl_root = raw_dir / "xbrl"
    if not xbrl_root.is_dir():
        return []

    for ticker_dir in sorted(path for path in xbrl_root.iterdir() if path.is_dir()):
        ticker = ticker_dir.name

        for extract_dir in sorted(path for path in ticker_dir.iterdir() if path.is_dir()):
            zip_path = ticker_dir / f"{extract_dir.name}.zip"
            if not zip_path.is_file():
                continue
            key = (ticker, extract_dir.name)
            report = reports.get(key)
            if report is None:
                report = _RawEdinetReport(ticker=ticker, doc_id=extract_dir.name)
                reports[key] = report
            report.xbrl_path = str(extract_dir.resolve())
            report.artifact_mtime_ns = max(
                report.artifact_mtime_ns,
                _max_mtime_ns(extract_dir),
                zip_path.stat().st_mtime_ns,
            )

        for legacy_file in sorted(ticker_dir.glob("*.xhtml")):
            if (ticker_dir / legacy_file.stem).is_dir() and (ticker_dir / f"{legacy_file.stem}.zip").is_file():
                continue
            key = (ticker, legacy_file.stem)
            report = reports.get(key)
            if report is None:
                report = _RawEdinetReport(ticker=ticker, doc_id=legacy_file.stem)
                reports[key] = report
            report.xbrl_path = str(legacy_file.resolve())
            report.artifact_mtime_ns = max(report.artifact_mtime_ns, legacy_file.stat().st_mtime_ns)

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
            SELECT ticker, doc_id, xbrl_path
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
            current["xbrl_path"] != report.xbrl_path
        ):
            upsert_sec_report(
                conn,
                ticker=report.ticker,
                fiscal_year="latest",
                doc_id=report.doc_id,
                xbrl_path=report.xbrl_path,
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
