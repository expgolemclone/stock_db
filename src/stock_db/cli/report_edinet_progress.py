from __future__ import annotations

import argparse
import csv
import sqlite3
import sys
from collections.abc import Mapping
from datetime import datetime
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, VAR_DIR, edinet_phase1_config


def _connect_readonly(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(f"file:{db_path.resolve()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    return conn


def _default_label() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def _fetch_count(
    conn: sqlite3.Connection,
    query: str,
    params: Sequence[object] = (),
) -> int:
    row = conn.execute(query, params).fetchone()
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


def _write_tsv(path: Path, headers: list[str], rows: Sequence[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n")
        writer.writerow(headers)
        for row in rows:
            if isinstance(row, Mapping):
                writer.writerow([row[h] for h in headers])
                continue
            writer.writerow([row[h] for h in headers])


def _excluded_ticker_reasons() -> dict[str, str]:
    raw = edinet_phase1_config()
    excluded = raw.get("excluded_tickers", {})
    return {
        str(ticker): str(reason).strip()
        for ticker, reason in excluded.items()
        if str(reason).strip()
    }


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
        excluded_tickers = _excluded_ticker_reasons()
        excluded_keys = sorted(excluded_tickers)
        excluded_placeholders = ",".join("?" for _ in excluded_keys)
        total_stocks = _fetch_count(conn, "SELECT COUNT(*) FROM stocks")
        phase1_pending = _fetch_count(
            conn,
            "SELECT COUNT(*) FROM stocks WHERE securities_report_url IS NULL",
        )
        phase1_excluded = (
            _fetch_count(
                conn,
                (
                    "SELECT COUNT(*) FROM stocks "
                    f"WHERE securities_report_url IS NULL AND ticker IN ({excluded_placeholders})"
                ),
                excluded_keys,
            ) if excluded_keys else 0
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
        phase1_pending_actionable = phase1_pending - phase1_excluded

        if excluded_keys:
            phase1_headers = ["ticker", "name", "edinet_code", "updated_at"]
            phase1_rows = conn.execute(
                f"""
                SELECT ticker, name, COALESCE(edinet_code, '') AS edinet_code, updated_at
                FROM stocks
                WHERE securities_report_url IS NULL
                  AND ticker NOT IN ({excluded_placeholders})
                ORDER BY ticker
                """,
                excluded_keys,
            ).fetchall()
            phase1_excluded_headers = ["ticker", "name", "edinet_code", "reason", "updated_at"]
            excluded_rows = conn.execute(
                f"""
                SELECT ticker, name, COALESCE(edinet_code, '') AS edinet_code, updated_at
                FROM stocks
                WHERE securities_report_url IS NULL
                  AND ticker IN ({excluded_placeholders})
                ORDER BY ticker
                """,
                excluded_keys,
            ).fetchall()
            phase1_excluded_rows = [
                {
                    "ticker": row["ticker"],
                    "name": row["name"],
                    "edinet_code": row["edinet_code"],
                    "reason": excluded_tickers[row["ticker"]],
                    "updated_at": row["updated_at"],
                }
                for row in excluded_rows
            ]
        else:
            phase1_headers, phase1_rows = _fetch_rows(
                conn,
                """
                SELECT ticker, name, COALESCE(edinet_code, '') AS edinet_code, updated_at
                FROM stocks
                WHERE securities_report_url IS NULL
                ORDER BY ticker
                """,
            )
            phase1_excluded_headers = ["ticker", "name", "edinet_code", "reason", "updated_at"]
            phase1_excluded_rows: list[dict[str, str]] = []
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
    phase1_excluded_path = output_dir / f"edinet_phase1_excluded_{label}.tsv"
    no_report_path = output_dir / f"edinet_phase2_no_report_{label}.tsv"
    no_xbrl_path = output_dir / f"edinet_phase2_no_xbrl_{label}.tsv"

    _write_tsv(phase1_path, phase1_headers, phase1_rows)
    _write_tsv(phase1_excluded_path, phase1_excluded_headers, phase1_excluded_rows)
    _write_tsv(no_report_path, no_report_headers, no_report_rows)
    _write_tsv(no_xbrl_path, no_xbrl_headers, no_xbrl_rows)

    print(f"db_path: {db_path.resolve()}")
    print(f"label: {label}")
    print(f"total_stocks: {total_stocks}")
    print(f"phase1_pending: {phase1_pending}")
    print(f"phase1_excluded: {phase1_excluded}")
    print(f"phase1_pending_actionable: {phase1_pending_actionable}")
    print(f"phase2_pending: {phase2_pending}")
    print(f"with_url_no_report: {with_url_no_report}")
    print(f"with_url_report_no_xbrl: {with_url_report_no_xbrl}")
    print(f"wrote: {phase1_path}")
    print(f"wrote: {phase1_excluded_path}")
    print(f"wrote: {no_report_path}")
    print(f"wrote: {no_xbrl_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
