from __future__ import annotations

import sqlite3

from stock_db.storage._refs import (
    ensure_data_source,
    ensure_share_class,
    ensure_source_kind,
)
from stock_db.storage._util import utc_now_iso


def upsert_share_classes_bulk(conn: sqlite3.Connection, rows: list[dict]) -> None:
    now = utc_now_iso()
    for row in rows:
        source_id = ensure_data_source(conn, str(row["source"]))
        source_kind_id = ensure_source_kind(conn, str(row["source_kind"]))
        share_class_id = ensure_share_class(
            conn,
            ticker=str(row["ticker"]),
            class_key=str(row["class_key"]),
            class_name=str(row["class_name"]),
            is_preferred=int(row["is_preferred"]),
            updated_at=now,
        )
        conn.execute(
            """
            INSERT INTO share_class_issuances
                (period, source_id, share_class_id, shares, source_kind_id, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(period, source_id, share_class_id) DO UPDATE SET
                shares=excluded.shares,
                source_kind_id=excluded.source_kind_id,
                updated_at=excluded.updated_at
            """,
            (
                row["period"],
                source_id,
                share_class_id,
                row["shares"],
                source_kind_id,
                now,
            ),
        )


def replace_share_classes_for_ticker_source(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    rows: list[dict],
) -> None:
    conn.execute(
        """
        DELETE FROM share_class_issuances
        WHERE source_id = (
            SELECT source_id FROM data_sources WHERE source_code = ?
        )
          AND share_class_id IN (
              SELECT share_class_id FROM share_class_defs WHERE ticker = ?
          )
        """,
        (source, ticker),
    )
    if rows:
        upsert_share_classes_bulk(conn, rows)
