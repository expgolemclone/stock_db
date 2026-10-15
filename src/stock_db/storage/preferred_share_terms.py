from __future__ import annotations

import sqlite3
from collections import defaultdict
from collections.abc import Callable

from stock_db.storage._refs import (
    ensure_data_source,
    ensure_share_class,
    ensure_source_kind,
)
from stock_db.storage._util import utc_now_iso

_DILUTION_VALUE_KEYS = (
    "preferred_shares",
    "converted_common_shares",
    "conversion_ratio_to_common",
    "issue_price_yen",
    "capitalized_amount_yen",
)


def _source_kind_priority(source_kind: str) -> int:
    return {
        "issued_share_change_text": 0,
        "issued_shares_text": 1,
        "classes_of_shares_text": 2,
        "per_share_information_text": 3,
        "business_results_text": 4,
    }.get(source_kind, 9)


def _value_count(row: dict) -> int:
    return sum(row.get(key) is not None for key in _DILUTION_VALUE_KEYS)


def _term_sort_key(row: dict) -> tuple[int, int, str, str]:
    return (
        _source_kind_priority(str(row.get("source_kind", ""))),
        -_value_count(row),
        str(row.get("source_concept", "")),
        str(row.get("terms_text", "")),
    )


def _first_value(
    rows: list[dict],
    getter: Callable[[dict], float | None],
) -> float | None:
    for row in rows:
        value = getter(row)
        if value is not None:
            return value
    return None


def normalize_preferred_share_dilution_terms(rows: list[dict]) -> list[dict]:
    """Collapse raw text-source rows into one dilution row per share class."""

    grouped: dict[tuple[str, str, str, str], list[dict]] = defaultdict(list)
    for row in rows:
        grouped[
            (
                str(row["ticker"]),
                str(row["period"]),
                str(row["source"]),
                str(row["class_key"]),
            )
        ].append(row)

    normalized = []
    for (ticker, period, source, class_key), group_rows in grouped.items():
        sorted_rows = sorted(group_rows, key=_term_sort_key)
        best_row = sorted_rows[0]
        preferred_shares = _first_value(
            sorted_rows, lambda row: row["preferred_shares"]
        )
        converted_common_shares = _first_value(
            sorted_rows, lambda row: row["converted_common_shares"]
        )
        conversion_ratio_to_common = _first_value(
            sorted_rows, lambda row: row["conversion_ratio_to_common"]
        )
        if (
            conversion_ratio_to_common is None
            and preferred_shares not in (None, 0)
            and converted_common_shares is not None
        ):
            conversion_ratio_to_common = converted_common_shares / preferred_shares
        if (
            converted_common_shares is None
            and preferred_shares is not None
            and conversion_ratio_to_common is not None
        ):
            converted_common_shares = preferred_shares * conversion_ratio_to_common

        normalized.append(
            {
                "ticker": ticker,
                "period": period,
                "source": source,
                "class_key": class_key,
                "class_name": str(best_row["class_name"]),
                "preferred_shares": preferred_shares,
                "converted_common_shares": converted_common_shares,
                "conversion_ratio_to_common": conversion_ratio_to_common,
                "issue_price_yen": _first_value(
                    sorted_rows, lambda row: row["issue_price_yen"]
                ),
                "capitalized_amount_yen": _first_value(
                    sorted_rows, lambda row: row["capitalized_amount_yen"]
                ),
                "source_concept": str(best_row["source_concept"]),
                "source_kind": str(best_row["source_kind"]),
            }
        )

    normalized.sort(key=lambda row: (str(row["period"]), str(row["class_name"])))
    return normalized


def upsert_preferred_share_terms_bulk(conn: sqlite3.Connection, rows: list[dict]) -> None:
    now = utc_now_iso()
    for row in rows:
        source_id = ensure_data_source(conn, str(row["source"]))
        source_kind_id = ensure_source_kind(conn, str(row["source_kind"]))
        share_class_id = ensure_share_class(
            conn,
            ticker=str(row["ticker"]),
            class_key=str(row["class_key"]),
            class_name=str(row["class_name"]),
            is_preferred=1,
            updated_at=now,
        )
        conn.execute(
            """
            INSERT INTO preferred_share_text_blocks
                (period, source_id, share_class_id, terms_text,
                 source_concept, source_kind_id, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(period, source_id, share_class_id, source_concept)
            DO UPDATE SET
                terms_text=excluded.terms_text,
                source_kind_id=excluded.source_kind_id,
                updated_at=excluded.updated_at
            """,
            (
                row["period"],
                source_id,
                share_class_id,
                row["terms_text"],
                row["source_concept"],
                source_kind_id,
                now,
            ),
        )
        text_block_id = conn.execute(
            """
            SELECT text_block_id FROM preferred_share_text_blocks
            WHERE period = ?
              AND source_id = ?
              AND share_class_id = ?
              AND source_concept = ?
            """,
            (row["period"], source_id, share_class_id, row["source_concept"]),
        ).fetchone()["text_block_id"]
        conn.execute(
            "DELETE FROM preferred_share_term_facts WHERE text_block_id = ?",
            (text_block_id,),
        )
        conn.executemany(
            """
            INSERT INTO preferred_share_term_facts
                (text_block_id, fact_name, value)
            VALUES (?, ?, ?)
            """,
            [
                (text_block_id, key, row[key])
                for key in _DILUTION_VALUE_KEYS
                if row[key] is not None
            ],
        )


def upsert_preferred_share_dilution_terms_bulk(
    conn: sqlite3.Connection,
    rows: list[dict],
    *,
    updated_at: str | None = None,
) -> None:
    # Kept for callers that still import this helper. Dilution terms are now
    # derived by the preferred_share_dilution_terms compatibility view.
    _ = (conn, rows, updated_at)


def replace_preferred_share_terms_for_ticker_source(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    rows: list[dict],
) -> None:
    conn.execute(
        """
        DELETE FROM preferred_share_text_blocks
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
        upsert_preferred_share_terms_bulk(conn, rows)
