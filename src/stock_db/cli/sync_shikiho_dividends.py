"""Sync dividend data from japan_company_handbook into financial_items."""

from __future__ import annotations

import argparse
import sqlite3
import sys
import tomllib
from pathlib import Path
from typing import Sequence

from stock_db.paths import PROJECT_ROOT, STOCKS_DB_PATH
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_items_bulk
from stock_db.storage.schema import init_db

_SHIKIHO_DB_PATH = (
    STOCKS_DB_PATH.parent.parent.parent.parent
    / "japan_company_handbook"
    / "data"
    / "stock_performance.db"
)
_DIVIDEND_OVERRIDES_PATH = PROJECT_ROOT / "config" / "shikiho_dividend_overrides.toml"
_SOURCE = "shikiho"
_STATEMENT = "dividend"
DividendOverrides = dict[str, dict[str, float]]


def _load_dividend_overrides(path: Path) -> DividendOverrides:
    if not path.exists():
        return {}

    with path.open("rb") as handle:
        data = tomllib.load(handle)

    raw_overrides = data.get("dividend_overrides", {})
    if not isinstance(raw_overrides, dict):
        raise ValueError("dividend_overrides must be a table")

    overrides: DividendOverrides = {}
    for ticker, period_values in raw_overrides.items():
        if not isinstance(ticker, str) or not ticker:
            raise ValueError(f"invalid dividend override ticker: {ticker!r}")
        if not isinstance(period_values, dict):
            raise ValueError(f"dividend override for {ticker} must be a table")

        overrides[ticker] = {}
        for period, value in period_values.items():
            if not isinstance(period, str) or not period:
                raise ValueError(f"invalid dividend override period for {ticker}: {period!r}")
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                raise ValueError(
                    f"dividend override for {ticker} {period} must be numeric"
                )
            overrides[ticker][period] = float(value)

    return overrides


def _dividend_value(
    *,
    ticker: str,
    period: str,
    value: float,
    overrides: DividendOverrides,
) -> float:
    return overrides.get(ticker, {}).get(period, value)


def _sync(
    conn: sqlite3.Connection,
    shikiho_db_path: str,
    dividend_overrides_path: Path,
) -> tuple[int, int]:
    if not Path(shikiho_db_path).exists():
        print(f"Shikiho DB not found: {shikiho_db_path}", file=sys.stderr)
        return 0, 0

    dividend_overrides = _load_dividend_overrides(dividend_overrides_path)

    shikiho_conn = sqlite3.connect(shikiho_db_path)
    shikiho_conn.row_factory = sqlite3.Row
    try:
        rows = shikiho_conn.execute(
            """
            SELECT stock_code, period, dividend
            FROM stock_dividends
            WHERE dividend IS NOT NULL
            ORDER BY stock_code, period ASC
            """
        ).fetchall()
    finally:
        shikiho_conn.close()

    grouped: dict[str, list[sqlite3.Row]] = {}
    for row in rows:
        grouped.setdefault(row["stock_code"], []).append(row)

    conn.execute(
        "DELETE FROM financial_items WHERE source = ? AND statement = ?",
        (_SOURCE, _STATEMENT),
    )

    ok = 0
    skipped = 0
    for ticker, dividends in sorted(grouped.items()):
        if not dividends:
            skipped += 1
            continue

        db_rows = [
            {
                "ticker": ticker,
                "period": div["period"],
                "statement": _STATEMENT,
                "item_name": "dps",
                "value": _dividend_value(
                    ticker=ticker,
                    period=div["period"],
                    value=div["dividend"],
                    overrides=dividend_overrides,
                ),
                "source": _SOURCE,
            }
            for div in dividends
        ]

        upsert_financial_items_bulk(conn, db_rows)
        ok += 1

    conn.commit()
    return ok, skipped


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Sync dividend data from japan_company_handbook"
    )
    parser.add_argument(
        "--shikiho-db",
        default=str(_SHIKIHO_DB_PATH),
        help="Path to stock_performance.db",
    )
    parser.add_argument(
        "--db",
        default=str(STOCKS_DB_PATH),
        help="Path to stocks.db",
    )
    parser.add_argument(
        "--dividend-overrides",
        default=str(_DIVIDEND_OVERRIDES_PATH),
        help="Path to dividend override TOML",
    )
    args = parser.parse_args(argv)

    conn: sqlite3.Connection = get_connection(Path(args.db))
    try:
        init_db(conn)
        ok, skipped = _sync(conn, args.shikiho_db, Path(args.dividend_overrides))
        print(f"Synced {ok} tickers ({skipped} skipped)", file=sys.stderr)
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    raise SystemExit(main())
