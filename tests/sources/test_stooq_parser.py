from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.stooq import StooqParseError, ingest_daily_prices
from stock_db.storage.stocks import upsert_stock


def test_ingest_daily_prices_imports_only_supported_jp_rows(
    db_conn: object,
    tmp_path: Path,
) -> None:
    for ticker in ("7203", "6758", "211A", "9A76", "9A7A"):
        upsert_stock(db_conn, ticker, ticker, "", "")
    csv_path = tmp_path / "0429_d.csv"
    csv_path.write_text(
        "\n".join([
            "<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>",
            "7203.JP,D,20260429,000000,3065,3101,3057,3067,19390900,0",
            "6758.JP,D,20260429,000000,3500,3510,3480,3495,101.44927535999999,0",
            "211A.JP,D,20260429,000000,4070,4080,4050,4080,2100,0",
            "9A76.JP,D,20260429,000000,100,101,99,100,500,0",
            "9A7A.JP,D,20260429,000000,200,201,199,200,600,0",
            "12345.JP,D,20260429,000000,100,101,99,100,500,0",
            "130B.JP,D,20260429,000000,100,101,99,100,500,0",
            "5594.JP,D,,000000,,,,,0,0",
            "AAPL.US,D,20260429,000000,200,205,198,204,1000,0",
        ]),
        encoding="utf-8",
    )

    imported = ingest_daily_prices(db_conn, csv_path)
    rows = db_conn.execute(
        "SELECT ticker, date, close, volume FROM prices ORDER BY ticker"
    ).fetchall()
    unresolved = db_conn.execute(
        "SELECT ticker, reason FROM unresolved_price_observations ORDER BY ticker"
    ).fetchall()

    assert imported == 5
    assert [dict(row) for row in rows] == [
        {"ticker": "211A", "date": "2026-04-29", "close": 4080.0, "volume": None},
        {"ticker": "6758", "date": "2026-04-29", "close": 3495.0, "volume": None},
        {"ticker": "7203", "date": "2026-04-29", "close": 3067.0, "volume": None},
        {"ticker": "9A76", "date": "2026-04-29", "close": 100.0, "volume": None},
        {"ticker": "9A7A", "date": "2026-04-29", "close": 200.0, "volume": None},
    ]
    assert [tuple(row) for row in unresolved] == [
        ("12345", "ticker_not_in_stock_entities"),
    ]


def test_ingest_daily_prices_rejects_unexpected_header(
    db_conn: object,
    tmp_path: Path,
) -> None:
    csv_path = tmp_path / "bad.csv"
    csv_path.write_text(
        "\n".join([
            "Ticker,Per,Date,Time,Open,High,Low,Close,Vol,OpenInt",
            "7203.JP,D,20260429,000000,3065,3101,3057,3067,19390900,0",
        ]),
        encoding="utf-8",
    )

    with pytest.raises(StooqParseError, match="Unexpected Stooq header"):
        ingest_daily_prices(db_conn, csv_path)
