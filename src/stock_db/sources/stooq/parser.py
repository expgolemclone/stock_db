from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path

from stock_db.sources.stooq.exceptions import StooqParseError
from stock_db.storage.prices import (
    STOOQ_PRICE_SOURCE,
    record_unresolved_price,
    upsert_price,
)

_EXPECTED_HEADER = (
    "<ticker>",
    "<per>",
    "<date>",
    "<time>",
    "<open>",
    "<high>",
    "<low>",
    "<close>",
    "<vol>",
    "<openint>",
)
_JPX_CODE_LETTERS = frozenset("ACDFGHJKLMNPRSTUWXY")


def _is_jpx_security_code(ticker: str) -> bool:
    if len(ticker) != 4:
        return False
    return (
        ticker[0].isdigit()
        and (ticker[1].isdigit() or ticker[1] in _JPX_CODE_LETTERS)
        and ticker[2].isdigit()
        and (ticker[3].isdigit() or ticker[3] in _JPX_CODE_LETTERS)
    )


def _is_supported_ticker(ticker: str) -> bool:
    return _is_jpx_security_code(ticker) or (len(ticker) == 5 and ticker.isdigit())


def parse_stooq_daily_file(file_path: str) -> list[tuple[str, str, float]]:
    text = Path(file_path).read_text(encoding="utf-8-sig")
    lines = text.splitlines()
    if not lines:
        raise StooqParseError("Stooq daily file is empty")
    header = [cell.strip().lower() for cell in lines[0].split(",")]
    if tuple(header) != _EXPECTED_HEADER:
        raise StooqParseError(f"Unexpected Stooq header: {lines[0].split(',')}")

    rows: list[tuple[str, str, float]] = []
    for index, line in enumerate(lines[1:], start=2):
        cells = line.split(",")
        if not cells or all(cell.strip() == "" for cell in cells):
            continue
        if len(cells) != len(_EXPECTED_HEADER):
            raise StooqParseError(
                f"Unexpected column count on line {index}: {len(cells)}"
            )
        symbol = cells[0].strip().upper()
        if not symbol.endswith(".JP"):
            continue
        ticker = symbol[:-3]
        if not _is_supported_ticker(ticker):
            continue
        price_date = cells[2].strip()
        if not price_date:
            continue
        close_raw = cells[7].strip()
        if not close_raw:
            continue
        try:
            normalized_date = datetime.strptime(price_date, "%Y%m%d").strftime("%Y-%m-%d")
        except ValueError as exc:
            raise StooqParseError(
                f"Invalid Date value on line {index}: {price_date!r}"
            ) from exc
        try:
            close = float(close_raw)
        except ValueError as exc:
            raise StooqParseError(
                f"Invalid Close value on line {index}: {close_raw!r}"
            ) from exc
        rows.append((ticker, normalized_date, close))

    if not rows:
        raise StooqParseError(f"No JP daily prices found in {file_path}")
    return rows


def ingest_daily_prices(conn: sqlite3.Connection, file_path: Path) -> int:
    rows = parse_stooq_daily_file(str(file_path))
    known_tickers = {
        str(row[0])
        for row in conn.execute("SELECT ticker FROM stock_entities").fetchall()
    }
    imported = 0
    for ticker, normalized_date, close in rows:
        if ticker not in known_tickers:
            record_unresolved_price(
                conn,
                ticker,
                normalized_date,
                close,
                None,
                source=STOOQ_PRICE_SOURCE,
                reason="ticker_not_in_stock_entities",
            )
            continue
        upsert_price(
            conn,
            ticker,
            normalized_date,
            close,
            None,
            source=STOOQ_PRICE_SOURCE,
        )
        imported += 1
    return imported
