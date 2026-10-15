from __future__ import annotations

import sqlite3


def ensure_data_source(conn: sqlite3.Connection, source: str) -> int:
    conn.execute(
        """
        INSERT INTO data_sources (source_code)
        VALUES (?)
        ON CONFLICT(source_code) DO NOTHING
        """,
        (source,),
    )
    row = conn.execute(
        "SELECT source_id FROM data_sources WHERE source_code = ?",
        (source,),
    ).fetchone()
    return int(row["source_id"])


def ensure_source_kind(conn: sqlite3.Connection, source_kind: str) -> int:
    conn.execute(
        """
        INSERT INTO source_kinds (source_kind)
        VALUES (?)
        ON CONFLICT(source_kind) DO NOTHING
        """,
        (source_kind,),
    )
    row = conn.execute(
        "SELECT source_kind_id FROM source_kinds WHERE source_kind = ?",
        (source_kind,),
    ).fetchone()
    return int(row["source_kind_id"])


def ensure_doc_type(conn: sqlite3.Connection, doc_type: str) -> int:
    conn.execute(
        """
        INSERT INTO doc_types (doc_type)
        VALUES (?)
        ON CONFLICT(doc_type) DO NOTHING
        """,
        (doc_type,),
    )
    row = conn.execute(
        "SELECT doc_type_id FROM doc_types WHERE doc_type = ?",
        (doc_type,),
    ).fetchone()
    return int(row["doc_type_id"])


def ensure_financial_item(
    conn: sqlite3.Connection,
    statement: str,
    item_name: str,
) -> int:
    conn.execute(
        """
        INSERT INTO financial_item_defs (statement, item_name)
        VALUES (?, ?)
        ON CONFLICT(statement, item_name) DO NOTHING
        """,
        (statement, item_name),
    )
    row = conn.execute(
        """
        SELECT item_id FROM financial_item_defs
        WHERE statement = ? AND item_name = ?
        """,
        (statement, item_name),
    ).fetchone()
    return int(row["item_id"])


def ensure_share_class(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    class_key: str,
    class_name: str,
    is_preferred: bool | int,
    updated_at: str,
) -> int:
    conn.execute(
        """
        INSERT INTO share_class_defs
            (ticker, class_key, class_name, is_preferred, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(ticker, class_key) DO UPDATE SET
            class_name = excluded.class_name,
            is_preferred = excluded.is_preferred,
            updated_at = excluded.updated_at
        """,
        (ticker, class_key, class_name, int(is_preferred), updated_at),
    )
    row = conn.execute(
        """
        SELECT share_class_id FROM share_class_defs
        WHERE ticker = ? AND class_key = ?
        """,
        (ticker, class_key),
    ).fetchone()
    return int(row["share_class_id"])
