from __future__ import annotations

import sqlite3
from importlib.resources import files

SCHEMA_VERSION = 4


def _read_schema_resource(name: str) -> str:
    return files("stock_db.storage.sql").joinpath(name).read_text(encoding="utf-8")


_BASE_SCHEMA_SQL = _read_schema_resource("0001_base.sql")
_VIEWS_SCHEMA_SQL = _read_schema_resource("0001_views.sql")
_CONTEXT_PERIOD_VIEWS_SQL = _read_schema_resource("0002_context_period_views.sql")


def _schema_version(conn: sqlite3.Connection) -> int:
    row = conn.execute("PRAGMA user_version").fetchone()
    if row is None:
        raise RuntimeError("SQLite did not return PRAGMA user_version")
    return int(row[0])


def _application_object_count(conn: sqlite3.Connection) -> int:
    row = conn.execute(
        "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'"
    ).fetchone()
    if row is None:
        raise RuntimeError("SQLite did not return an object count")
    return int(row[0])


def init_db(conn: sqlite3.Connection) -> None:
    conn.execute("PRAGMA foreign_keys = ON")
    version = _schema_version(conn)
    if version == SCHEMA_VERSION:
        return
    if version != 0:
        raise RuntimeError(
            f"unsupported stocks DB schema version {version}; "
            f"expected {SCHEMA_VERSION}; rebuild the DB"
        )
    if _application_object_count(conn) != 0:
        raise RuntimeError(
            "unversioned stocks DB detected; rebuild it explicitly before using stock_db"
        )
    if conn.in_transaction:
        raise RuntimeError("schema initialization requires an idle connection")

    try:
        conn.executescript(
            "BEGIN IMMEDIATE;\n"
            + _BASE_SCHEMA_SQL
            + "\n"
            + _VIEWS_SCHEMA_SQL
            + "\n"
            + _CONTEXT_PERIOD_VIEWS_SQL
            + f"\nPRAGMA user_version = {SCHEMA_VERSION};\n"
        )
        violation = conn.execute("PRAGMA foreign_key_check").fetchone()
        if violation is not None:
            raise RuntimeError(f"new schema failed foreign key validation: {tuple(violation)}")
        conn.commit()
    except Exception:
        conn.rollback()
        raise
