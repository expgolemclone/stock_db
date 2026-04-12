from __future__ import annotations

import sqlite3
from typing import TypedDict


class MarketCapRow(TypedDict):
    ticker: str
    source: str
    value_yen: int
    fetched_at: str


def upsert_market_cap(
    conn: sqlite3.Connection,
    ticker: str,
    source: str,
    value_yen: int,
    fetched_at: str,
) -> None:
    conn.execute(
        """
        INSERT INTO market_cap (ticker, source, value_yen, fetched_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(ticker, source) DO UPDATE SET
            value_yen  = excluded.value_yen,
            fetched_at = excluded.fetched_at
        """,
        (ticker, source, value_yen, fetched_at),
    )


def get_market_cap(
    conn: sqlite3.Connection,
    ticker: str,
) -> MarketCapRow | None:
    row = conn.execute(
        """
        SELECT ticker, source, value_yen, fetched_at
        FROM market_cap
        WHERE ticker = ?
        ORDER BY fetched_at DESC
        LIMIT 1
        """,
        (ticker,),
    ).fetchone()
    if row is None:
        return None
    return MarketCapRow(
        ticker=row["ticker"],
        source=row["source"],
        value_yen=row["value_yen"],
        fetched_at=row["fetched_at"],
    )
