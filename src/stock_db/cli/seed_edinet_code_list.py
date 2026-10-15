"""CLI entry point for ingesting the EDINET Code List CSV.

Downloads (or reads a local copy of) the EDINET Code List CSV and upserts
edinet_code + company name mappings into the stock_entities table.

Usage:
    seed-edinet-code-list /path/to/EdinetcodeDlInfo.csv
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from stock_db.paths import STOCKS_DB_PATH
from stock_db.sources.edinet.resolver import EdinetResolver, parse_edinet_code_csv
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Ingest the EDINET Code List CSV into the stock database",
    )
    parser.add_argument(
        "csv_path",
        type=Path,
        help="Path to the EdinetcodeDlInfo.csv file (Shift-JIS encoded)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Parse the CSV but do not write to the database",
    )
    args = parser.parse_args(argv)

    csv_path: Path = args.csv_path
    if not csv_path.is_file():
        print(f"File not found: {csv_path}", file=sys.stderr)
        return 1

    rows = parse_edinet_code_csv(csv_path)
    if not rows:
        print(f"No valid rows found in {csv_path}", file=sys.stderr)
        return 1

    print(f"Parsed {len(rows)} entries from {csv_path.name}")

    if args.dry_run:
        for row in rows[:10]:
            print(f"  {row['ticker']}  {row['edinet_code']}  {row['name']}")
        if len(rows) > 10:
            print(f"  ... and {len(rows) - 10} more")
        return 0

    conn = get_connection(STOCKS_DB_PATH)
    try:
        init_db(conn)
        resolver = EdinetResolver.from_edinet_code_csv(conn, csv_path)
        print(
            f"Ingested {len(rows)} entries; "
            f"resolver now has {len(resolver.edinet_code_reverse_map)} edinet_code mappings"
        )
    finally:
        conn.close()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
