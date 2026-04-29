from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.stooq import StooqParseError, ingest_daily_prices


def test_ingest_daily_prices_imports_only_jp_rows(db_conn: object, tmp_path: Path) -> None:
    csv_path = tmp_path / "0429_d.csv"
    csv_path.write_text(
        "\n".join([
            "<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>",
            "7203.JP,D,20260429,000000,3065,3101,3057,3067,19390900,0",
            "6758.JP,D,20260429,000000,3500,3510,3480,3495,101.44927535999999,0",
            "AAPL.US,D,20260429,000000,200,205,198,204,1000,0",
            "12345.JP,D,20260429,000000,1,1,1,1,1,0",
        ]),
        encoding="utf-8",
    )

    imported = ingest_daily_prices(db_conn, csv_path)
    rows = db_conn.execute(
        "SELECT ticker, date, close, volume FROM prices ORDER BY ticker"
    ).fetchall()

    assert imported == 2
    assert [dict(row) for row in rows] == [
        {"ticker": "6758", "date": "2026-04-29", "close": 3495.0, "volume": None},
        {"ticker": "7203", "date": "2026-04-29", "close": 3067.0, "volume": None},
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
