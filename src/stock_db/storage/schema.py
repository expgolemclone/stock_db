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
    yf_suffix            TEXT,
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

CREATE TABLE IF NOT EXISTS share_classes (
    ticker       TEXT    NOT NULL,
    period       TEXT    NOT NULL,
    source       TEXT    NOT NULL,
    class_key    TEXT    NOT NULL,
    class_name   TEXT    NOT NULL,
    shares       REAL,
    is_preferred INTEGER NOT NULL,
    source_kind  TEXT    NOT NULL,
    updated_at   TEXT    NOT NULL,
    PRIMARY KEY (ticker, period, source, class_key)
);

CREATE INDEX IF NOT EXISTS idx_share_classes_ticker_period
    ON share_classes (ticker, period);

CREATE INDEX IF NOT EXISTS idx_share_classes_preferred
    ON share_classes (is_preferred);

CREATE TABLE IF NOT EXISTS prices (
    ticker     TEXT    NOT NULL,
    date       TEXT    NOT NULL,
    close      REAL,
    volume     INTEGER,
    updated_at TEXT,
    PRIMARY KEY (ticker, date)
);

CREATE TABLE IF NOT EXISTS sec_reports (
    ticker       TEXT    NOT NULL,
    fiscal_year  TEXT    NOT NULL,
    doc_id       TEXT    NOT NULL,
    doc_type     TEXT    NOT NULL DEFAULT 'annual_report',
    xbrl_path    TEXT,
    source       TEXT    NOT NULL DEFAULT 'edinet',
    updated_at   TEXT    NOT NULL,
    PRIMARY KEY (ticker, doc_id)
);

CREATE INDEX IF NOT EXISTS idx_sr_ticker ON sec_reports (ticker);
CREATE INDEX IF NOT EXISTS idx_sr_doc_id ON sec_reports (doc_id);
"""


def _table_columns(conn: sqlite3.Connection, table: str) -> set[str]:
    # table はハードコード値のみ。外部入力は挿入されない。
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()  # noqa: S608
    return {row[1] for row in rows}


def _rebuild_stocks_without_address_source_urls(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE TABLE stocks__new (
            ticker                TEXT PRIMARY KEY,
            edinet_code           TEXT,
            name                  TEXT,
            sector                TEXT,
            market                TEXT,
            shares_outstanding    INTEGER,
            shares_updated_at     TEXT,
            securities_report_url TEXT,
            updated_at            TEXT
        );

        INSERT INTO stocks__new (
            ticker,
            edinet_code,
            name,
            sector,
            market,
            shares_outstanding,
            shares_updated_at,
            securities_report_url,
            updated_at
        )
        SELECT
            ticker,
            edinet_code,
            name,
            sector,
            market,
            shares_outstanding,
            shares_updated_at,
            securities_report_url,
            updated_at
        FROM stocks;

        DROP TABLE stocks;
        ALTER TABLE stocks__new RENAME TO stocks;
        """
    )
    conn.commit()


def _rebuild_prices_without_legacy_shares(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE TABLE prices__new (
            ticker     TEXT    NOT NULL,
            date       TEXT    NOT NULL,
            close      REAL,
            volume     INTEGER,
            updated_at TEXT,
            PRIMARY KEY (ticker, date)
        );

        INSERT INTO prices__new (ticker, date, close, volume, updated_at)
        SELECT ticker, date, close, volume, updated_at
        FROM prices;

        DROP TABLE prices;
        ALTER TABLE prices__new RENAME TO prices;
        """
    )
    conn.commit()


def _rebuild_sec_reports_without_markdown_columns(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE TABLE sec_reports__new (
            ticker       TEXT    NOT NULL,
            fiscal_year  TEXT    NOT NULL,
            doc_id       TEXT    NOT NULL,
            doc_type     TEXT    NOT NULL DEFAULT 'annual_report',
            xbrl_path    TEXT,
            source       TEXT    NOT NULL DEFAULT 'edinet',
            updated_at   TEXT    NOT NULL,
            PRIMARY KEY (ticker, doc_id)
        );

        INSERT INTO sec_reports__new (
            ticker, fiscal_year, doc_id, doc_type, xbrl_path, source, updated_at
        )
        SELECT
            ticker, fiscal_year, doc_id, doc_type, xbrl_path, source, updated_at
        FROM sec_reports;

        DROP TABLE sec_reports;
        ALTER TABLE sec_reports__new RENAME TO sec_reports;
        """
    )
    conn.commit()


def _rebuild_sec_reports_with_composite_pk(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE TABLE sec_reports__new (
            ticker       TEXT    NOT NULL,
            fiscal_year  TEXT    NOT NULL,
            doc_id       TEXT    NOT NULL,
            doc_type     TEXT    NOT NULL DEFAULT 'annual_report',
            file_path    TEXT    NOT NULL,
            xbrl_path    TEXT,
            page_count   INTEGER,
            char_count   INTEGER,
            source       TEXT    NOT NULL DEFAULT 'edinet',
            updated_at   TEXT    NOT NULL,
            PRIMARY KEY (ticker, doc_id)
        );

        INSERT INTO sec_reports__new (
            ticker,
            fiscal_year,
            doc_id,
            doc_type,
            file_path,
            xbrl_path,
            page_count,
            char_count,
            source,
            updated_at
        )
        SELECT
            ticker,
            fiscal_year,
            doc_id,
            doc_type,
            file_path,
            xbrl_path,
            page_count,
            char_count,
            source,
            updated_at
        FROM sec_reports;

        DROP TABLE sec_reports;
        ALTER TABLE sec_reports__new RENAME TO sec_reports;
        """
    )
    conn.commit()


def _table_info(conn: sqlite3.Connection, table: str) -> list[sqlite3.Row]:
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()  # noqa: S608
    return list(rows)


def _migrate(conn: sqlite3.Connection) -> None:
    stock_cols = _table_columns(conn, "stocks")
    if stock_cols:
        new_cols = {
            "edinet_code": "TEXT",
            "shares_outstanding": "INTEGER",
            "shares_updated_at": "TEXT",
            "securities_report_url": "TEXT",
            "yf_suffix": "TEXT",
        }
        for col_name, col_type in new_cols.items():
            if col_name not in stock_cols:
                # col_name, col_type はハードコード値のみ。外部入力は挿入されない。
                conn.execute(f"ALTER TABLE stocks ADD COLUMN {col_name} {col_type}")  # noqa: S608
        conn.commit()
        if "address_source_urls" in stock_cols:
            _rebuild_stocks_without_address_source_urls(conn)

    price_cols = _table_columns(conn, "prices")
    if price_cols and "updated_at" not in price_cols:
        conn.execute("ALTER TABLE prices ADD COLUMN updated_at TEXT")
        conn.commit()
        price_cols = _table_columns(conn, "prices")
    if price_cols and "shares_outstanding" in price_cols:
        _rebuild_prices_without_legacy_shares(conn)

    if _table_columns(conn, "market_cap"):
        conn.execute("DROP TABLE market_cap")
        conn.commit()

    sr_cols = _table_columns(conn, "sec_reports")
    if sr_cols and "xbrl_path" not in sr_cols:
        conn.execute("ALTER TABLE sec_reports ADD COLUMN xbrl_path TEXT")
        conn.commit()
        sr_cols = _table_columns(conn, "sec_reports")
    if sr_cols:
        sr_pk = {row[1]: row[5] for row in _table_info(conn, "sec_reports")}
        if sr_pk.get("doc_id") == 1 and sr_pk.get("ticker", 0) == 0:
            _rebuild_sec_reports_with_composite_pk(conn)
    if sr_cols and "file_path" in sr_cols:
        _rebuild_sec_reports_without_markdown_columns(conn)


def init_db(conn: sqlite3.Connection) -> None:
    _migrate(conn)
    conn.executescript(_SCHEMA_SQL)
    conn.commit()
