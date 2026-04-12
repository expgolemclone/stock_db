from __future__ import annotations

import sqlite3

_SCHEMA_SQL = """\
CREATE TABLE IF NOT EXISTS stocks (
    ticker               TEXT PRIMARY KEY,
    edinet_code          TEXT,
    name                 TEXT,
    sector               TEXT,
    market               TEXT,
    shares_outstanding   INTEGER,
    shares_updated_at    TEXT,
    securities_report_url TEXT,
    address_source_urls  TEXT,
    updated_at           TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_stocks_edinet_code
    ON stocks (edinet_code) WHERE edinet_code IS NOT NULL;

CREATE TABLE IF NOT EXISTS financial_items (
    ticker     TEXT    NOT NULL,
    period     TEXT    NOT NULL,
    statement  TEXT    NOT NULL,
    item_name  TEXT    NOT NULL,
    value      REAL,
    source     TEXT    NOT NULL,
    updated_at TEXT    NOT NULL,
    PRIMARY KEY (ticker, period, statement, item_name)
);

CREATE INDEX IF NOT EXISTS idx_fi_statement_item
    ON financial_items (statement, item_name);

CREATE INDEX IF NOT EXISTS idx_fi_ticker
    ON financial_items (ticker);

CREATE TABLE IF NOT EXISTS prices (
    ticker     TEXT    NOT NULL,
    date       TEXT    NOT NULL,
    close      REAL,
    volume     INTEGER,
    updated_at TEXT,
    PRIMARY KEY (ticker, date)
);

CREATE TABLE IF NOT EXISTS market_cap (
    ticker      TEXT    NOT NULL,
    source      TEXT    NOT NULL,
    value_yen   INTEGER,
    fetched_at  TEXT    NOT NULL,
    PRIMARY KEY (ticker, source)
);
"""


def _table_columns(conn: sqlite3.Connection, table: str) -> set[str]:
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()  # noqa: S608
    return {row[1] for row in rows}


def _migrate(conn: sqlite3.Connection) -> None:
    stock_cols = _table_columns(conn, "stocks")
    if stock_cols:
        new_cols = {
            "edinet_code": "TEXT",
            "shares_outstanding": "INTEGER",
            "shares_updated_at": "TEXT",
            "securities_report_url": "TEXT",
            "address_source_urls": "TEXT",
        }
        for col_name, col_type in new_cols.items():
            if col_name not in stock_cols:
                conn.execute(f"ALTER TABLE stocks ADD COLUMN {col_name} {col_type}")
        if "edinet_code" not in stock_cols:
            conn.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_stocks_edinet_code "
                "ON stocks (edinet_code) WHERE edinet_code IS NOT NULL"
            )
        conn.commit()

    price_cols = _table_columns(conn, "prices")
    if price_cols and "updated_at" not in price_cols:
        conn.execute("ALTER TABLE prices ADD COLUMN updated_at TEXT")
        conn.commit()


def init_db(conn: sqlite3.Connection) -> None:
    _migrate(conn)
    conn.executescript(_SCHEMA_SQL)
    conn.commit()
