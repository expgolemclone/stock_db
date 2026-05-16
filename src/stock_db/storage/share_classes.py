from __future__ import annotations

import sqlite3

from stock_db.storage._util import utc_now_iso


def upsert_share_classes_bulk(conn: sqlite3.Connection, rows: list[dict]) -> None:
    now = utc_now_iso()
    conn.executemany(
        """
        INSERT INTO share_classes
            (
                ticker,
                period,
                source,
                class_key,
                class_name,
                shares,
                is_preferred,
                source_kind,
                updated_at
            )
        VALUES (
            :ticker,
            :period,
            :source,
            :class_key,
            :class_name,
            :shares,
            :is_preferred,
            :source_kind,
            :updated_at
        )
        ON CONFLICT(ticker, period, source, class_key) DO UPDATE SET
            class_name=excluded.class_name,
            shares=excluded.shares,
            is_preferred=excluded.is_preferred,
            source_kind=excluded.source_kind,
            updated_at=excluded.updated_at
        """,
        [{**row, "updated_at": now} for row in rows],
    )


def replace_share_classes_for_ticker_source(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    rows: list[dict],
) -> None:
    conn.execute(
        "DELETE FROM share_classes WHERE ticker = ? AND source = ?",
        (ticker, source),
    )
    if rows:
        upsert_share_classes_bulk(conn, rows)
