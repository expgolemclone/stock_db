#!/usr/bin/env python3
"""screening.db → stocks.db マイグレーション.

既存の screening.db に market_cap テーブルと
stocks.{securities_report_url, address_source_urls} カラムを追加して
stocks.db にリネームする。

Usage:
    uv run python scripts/migrate_screening_db.py [--db PATH]
"""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from stock_db.config import DATA_DIR
from stock_db.db.connection import get_connection
from stock_db.db.schema import init_db


def migrate(source: Path, dest: Path) -> None:
    if not source.exists():
        print(f"source not found: {source}", file=sys.stderr)
        sys.exit(1)

    if source != dest:
        print(f"copying {source} -> {dest}")
        shutil.copy2(source, dest)

    conn = get_connection(dest)
    init_db(conn)
    conn.close()
    print(f"migration complete: {dest}")


def main() -> None:
    parser = argparse.ArgumentParser(description="screening.db → stocks.db マイグレーション")
    parser.add_argument("--db", type=Path, default=DATA_DIR / "screening.db",
                        help="元の screening.db パス")
    parser.add_argument("--dest", type=Path, default=DATA_DIR / "stocks.db",
                        help="出力先 stocks.db パス")
    args = parser.parse_args()
    migrate(args.db, args.dest)


if __name__ == "__main__":
    main()
