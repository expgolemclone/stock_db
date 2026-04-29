from __future__ import annotations

import csv
import sqlite3
from datetime import datetime
from pathlib import Path
import re

from stock_db.sources.stooq.exceptions import StooqParseError
from stock_db.storage.prices import upsert_price

_EXPECTED_HEADER = [
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
]


def _parse_close(raw_value: str, *, line_number: int) -> float:
    value = raw_value.strip()
    if value == "":
        raise StooqParseError(f"Missing Close value on line {line_number}")
    try:
        return float(value)
    except ValueError as exc:
        raise StooqParseError(f"Invalid Close value on line {line_number}: {raw_value!r}") from exc


def ingest_daily_prices(conn: sqlite3.Connection, file_path: Path) -> int:
    with file_path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.reader(handle)
        try:
            header = next(reader)
        except StopIteration as exc:
            raise StooqParseError("Stooq daily file is empty") from exc

        normalized_header = [column.strip().lower() for column in header]
        if normalized_header != _EXPECTED_HEADER:
            raise StooqParseError(
                f"Unexpected Stooq header: {header!r}"
            )

        imported = 0
        for line_number, row in enumerate(reader, start=2):
            if not row or all(cell.strip() == "" for cell in row):
                continue
            if len(row) != len(header):
                raise StooqParseError(
                    f"Unexpected column count on line {line_number}: {len(row)}"
                )

            symbol = row[0].strip().upper()
            if not symbol.endswith(".JP"):
                continue

            ticker = symbol.removesuffix(".JP")
            if re.fullmatch(r"\d{4}", ticker) is None:
                continue

            price_date = row[2].strip()
            if price_date == "":
                continue

            close_raw = row[7].strip()
            if close_raw == "":
                continue

            try:
                normalized_date = datetime.strptime(price_date, "%Y%m%d").date().isoformat()
            except ValueError as exc:
                raise StooqParseError(
                    f"Invalid Date value on line {line_number}: {price_date!r}"
                ) from exc

            close = _parse_close(close_raw, line_number=line_number)
            upsert_price(conn, ticker, normalized_date, close, None)
            imported += 1

    if imported == 0:
        raise StooqParseError(f"No JP daily prices found in {file_path}")
    return imported
