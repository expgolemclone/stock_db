from __future__ import annotations

import argparse
import json
import sqlite3
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH


def _connect_readonly(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(f"file:{db_path.resolve()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    return conn


def _print_section(name: str, payload: object) -> None:
    print(f"[{name}]")
    if payload in (None, []):
        print("(none)")
    else:
        print(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
    print()


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Inspect stocks.db rows for one ticker")
    parser.add_argument("ticker", help="Ticker code to inspect")
    parser.add_argument(
        "--db",
        type=Path,
        default=STOCKS_DB_PATH,
        help=f"Path to sqlite DB (default: {STOCKS_DB_PATH})",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=5,
        help="Max rows to print for multi-row tables",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.limit < 1:
        print("--limit must be >= 1", file=sys.stderr)
        return 2

    db_path: Path = args.db
    if not db_path.exists():
        print(f"DB not found: {db_path}", file=sys.stderr)
        return 2

    conn = _connect_readonly(db_path)
    try:
        stock_row = conn.execute(
            "SELECT * FROM stocks WHERE ticker = ?",
            (args.ticker,),
        ).fetchone()
        price_rows = conn.execute(
            """
            SELECT ticker, date, close, volume, updated_at
            FROM prices
            WHERE ticker = ?
            ORDER BY date DESC
            LIMIT ?
            """,
            (args.ticker, args.limit),
        ).fetchall()
        market_cap_rows = conn.execute(
            """
            SELECT ticker, source, value_yen, fetched_at
            FROM market_cap
            WHERE ticker = ?
            ORDER BY fetched_at DESC, source
            LIMIT ?
            """,
            (args.ticker, args.limit),
        ).fetchall()
        sec_report_rows = conn.execute(
            """
            SELECT ticker, fiscal_year, doc_id, doc_type, file_path, xbrl_path,
                   page_count, char_count, source, updated_at
            FROM sec_reports
            WHERE ticker = ?
            ORDER BY updated_at DESC, fiscal_year DESC
            LIMIT ?
            """,
            (args.ticker, args.limit),
        ).fetchall()
        financial_rows = conn.execute(
            """
            SELECT ticker, period, statement, item_name, value, source, updated_at
            FROM financial_items
            WHERE ticker = ?
            ORDER BY period DESC, statement, item_name
            LIMIT ?
            """,
            (args.ticker, args.limit),
        ).fetchall()
    finally:
        conn.close()

    found_any = any((
        stock_row is not None,
        price_rows,
        market_cap_rows,
        sec_report_rows,
        financial_rows,
    ))
    if not found_any:
        print(f"No rows found for ticker {args.ticker}", file=sys.stderr)
        return 1

    print(f"db_path: {db_path.resolve()}")
    print(f"ticker: {args.ticker}")
    print()

    _print_section("stocks", dict(stock_row) if stock_row is not None else None)
    _print_section("prices", [dict(row) for row in price_rows])
    _print_section("market_cap", [dict(row) for row in market_cap_rows])
    _print_section("sec_reports", [dict(row) for row in sec_report_rows])
    _print_section("financial_items", [dict(row) for row in financial_rows])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
