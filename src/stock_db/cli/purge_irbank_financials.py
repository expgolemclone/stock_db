from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import purge_financial_items_for_source_like

_IRBANK_SOURCE_PATTERN = "irbank%"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Purge irbank-derived financial_items rows from stocks.db",
    )
    parser.add_argument(
        "--db",
        type=Path,
        default=STOCKS_DB_PATH,
        help=f"Path to sqlite DB (default: {STOCKS_DB_PATH})",
    )
    return parser


def _count_irbank_rows(conn: sqlite3.Connection) -> int:
    row = conn.execute(
        "SELECT COUNT(*) FROM financial_items WHERE source LIKE ?",
        (_IRBANK_SOURCE_PATTERN,),
    ).fetchone()
    return int(row[0]) if row is not None else 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    db_path: Path = args.db
    if not db_path.exists():
        print(f"DB not found: {db_path}", file=sys.stderr)
        return 2

    conn = get_connection(db_path)
    try:
        before = _count_irbank_rows(conn)
        deleted = purge_financial_items_for_source_like(conn, _IRBANK_SOURCE_PATTERN)
        conn.commit()
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.execute("VACUUM")
        after = _count_irbank_rows(conn)
    except sqlite3.Error as exc:
        print(str(exc), file=sys.stderr)
        return 1
    finally:
        conn.close()

    print(f"irbank rows before purge: {before}", file=sys.stderr)
    print(f"irbank rows deleted: {deleted}", file=sys.stderr)
    print(f"irbank rows after purge: {after}", file=sys.stderr)
    return 0 if after == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
