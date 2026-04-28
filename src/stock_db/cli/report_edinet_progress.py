from __future__ import annotations

import argparse
import csv
import sqlite3
import sys
from datetime import datetime
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, VAR_DIR


def _connect_readonly(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(f"file:{db_path.resolve()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    return conn


def _default_label() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def _fetch_count(conn: sqlite3.Connection, query: str) -> int:
    row = conn.execute(query).fetchone()
    if row is None:
        return 0
    return int(row[0])


def _fetch_rows(
    conn: sqlite3.Connection, query: str,
) -> tuple[list[str], list[sqlite3.Row]]:
    cursor = conn.execute(query)
    headers = [desc[0] for desc in cursor.description or []]
    rows = cursor.fetchall()
    return headers, rows


def _write_tsv(path: Path, headers: list[str], rows: list[sqlite3.Row]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n")
        writer.writerow(headers)
        for row in rows:
            writer.writerow([row[h] for h in headers])


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Summarize EDINET Phase 1/2 progress and export unresolved ticker lists",
    )
    parser.add_argument(
        "--db",
        type=Path,
        default=STOCKS_DB_PATH,
        help=f"Path to sqlite DB (default: {STOCKS_DB_PATH})",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=VAR_DIR / "reports",
        help=f"Directory for TSV exports (default: {VAR_DIR / 'reports'})",
    )
    parser.add_argument(
        "--label",
        default=None,
        help="Label suffix for exported files (default: current timestamp)",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    db_path: Path = args.db
    if not db_path.exists():
        print(f"DB not found: {db_path}", file=sys.stderr)
        return 2

    label = args.label or _default_label()
    output_dir: Path = args.output_dir

    conn = _connect_readonly(db_path)
    try:
        total_stocks = _fetch_count(conn, "SELECT COUNT(*) FROM stocks")
        phase1_pending = _fetch_count(
            conn,
            "SELECT COUNT(*) FROM stocks WHERE securities_report_url IS NULL",
        )
        with_url_no_report = _fetch_count(
            conn,
            """
            SELECT COUNT(*)
            FROM stocks s
            WHERE s.securities_report_url IS NOT NULL
              AND NOT EXISTS (
                SELECT 1 FROM sec_reports r WHERE r.ticker = s.ticker
              )
            """,
        )
        with_url_report_no_xbrl = _fetch_count(
            conn,
            """
            SELECT COUNT(*)
            FROM stocks s
            WHERE s.securities_report_url IS NOT NULL
              AND EXISTS (
                SELECT 1 FROM sec_reports r WHERE r.ticker = s.ticker
              )
              AND NOT EXISTS (
                SELECT 1 FROM sec_reports r2
                WHERE r2.ticker = s.ticker AND r2.xbrl_path IS NOT NULL
              )
            """,
        )
        phase2_pending = with_url_no_report + with_url_report_no_xbrl

        phase1_headers, phase1_rows = _fetch_rows(
            conn,
            """
            SELECT ticker, name, COALESCE(edinet_code, '') AS edinet_code, updated_at
            FROM stocks
            WHERE securities_report_url IS NULL
            ORDER BY ticker
            """,
        )
        no_report_headers, no_report_rows = _fetch_rows(
            conn,
            """
            SELECT s.ticker, s.name, s.securities_report_url, s.updated_at
            FROM stocks s
            WHERE s.securities_report_url IS NOT NULL
              AND NOT EXISTS (
                SELECT 1 FROM sec_reports r WHERE r.ticker = s.ticker
              )
            ORDER BY s.ticker
            """,
        )
        no_xbrl_headers, no_xbrl_rows = _fetch_rows(
            conn,
            """
            SELECT
                s.ticker,
                s.name,
                s.securities_report_url,
                COALESCE((
                    SELECT group_concat(r.doc_id, ',')
                    FROM sec_reports r
                    WHERE r.ticker = s.ticker
                ), '') AS doc_ids,
                s.updated_at
            FROM stocks s
            WHERE s.securities_report_url IS NOT NULL
              AND EXISTS (
                SELECT 1 FROM sec_reports r WHERE r.ticker = s.ticker
              )
              AND NOT EXISTS (
                SELECT 1 FROM sec_reports r2
                WHERE r2.ticker = s.ticker AND r2.xbrl_path IS NOT NULL
              )
            ORDER BY s.ticker
            """,
        )
    finally:
        conn.close()

    phase1_path = output_dir / f"edinet_phase1_unresolved_{label}.tsv"
    no_report_path = output_dir / f"edinet_phase2_no_report_{label}.tsv"
    no_xbrl_path = output_dir / f"edinet_phase2_no_xbrl_{label}.tsv"

    _write_tsv(phase1_path, phase1_headers, phase1_rows)
    _write_tsv(no_report_path, no_report_headers, no_report_rows)
    _write_tsv(no_xbrl_path, no_xbrl_headers, no_xbrl_rows)

    print(f"db_path: {db_path.resolve()}")
    print(f"label: {label}")
    print(f"total_stocks: {total_stocks}")
    print(f"phase1_pending: {phase1_pending}")
    print(f"phase2_pending: {phase2_pending}")
    print(f"with_url_no_report: {with_url_no_report}")
    print(f"with_url_report_no_xbrl: {with_url_report_no_xbrl}")
    print(f"wrote: {phase1_path}")
    print(f"wrote: {no_report_path}")
    print(f"wrote: {no_xbrl_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
