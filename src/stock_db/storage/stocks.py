from __future__ import annotations

import sqlite3
from datetime import datetime, timezone


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def upsert_stock(
    conn: sqlite3.Connection,
    ticker: str,
    name: str,
    sector: str,
    market: str,
    *,
    edinet_code: str | None = None,
) -> None:
    conn.execute(
        """
        INSERT INTO stocks (ticker, edinet_code, name, sector, market, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker) DO UPDATE SET
            edinet_code = COALESCE(excluded.edinet_code, stocks.edinet_code),
            name        = CASE WHEN excluded.name   = '' THEN stocks.name   ELSE excluded.name   END,
            sector      = CASE WHEN excluded.sector = '' THEN stocks.sector ELSE excluded.sector END,
            market      = CASE WHEN excluded.market = '' THEN stocks.market ELSE excluded.market END,
            updated_at  = excluded.updated_at
        """,
        (ticker, edinet_code, name, sector, market, _now()),
    )


def get_all_tickers(conn: sqlite3.Connection) -> list[str]:
    rows = conn.execute("SELECT ticker FROM stocks ORDER BY ticker").fetchall()
    return [r["ticker"] for r in rows]


def get_existing_tickers(conn: sqlite3.Connection, source: str) -> set[str]:
    rows = conn.execute(
        "SELECT DISTINCT ticker FROM financial_items WHERE source = ?",
        (source,),
    ).fetchall()
    return {r[0] for r in rows}


def get_edinet_code(conn: sqlite3.Connection, ticker: str) -> str | None:
    row = conn.execute(
        "SELECT edinet_code FROM stocks WHERE ticker = ?", (ticker,)
    ).fetchone()
    return row["edinet_code"] if row else None


def get_ticker_edinet_map(conn: sqlite3.Connection) -> dict[str, str]:
    rows = conn.execute(
        "SELECT ticker, edinet_code FROM stocks WHERE edinet_code IS NOT NULL"
    ).fetchall()
    return {r["ticker"]: r["edinet_code"] for r in rows}


def get_stock_names(conn: sqlite3.Connection) -> dict[str, str]:
    rows = conn.execute(
        "SELECT ticker, name FROM stocks ORDER BY ticker"
    ).fetchall()
    return {r["ticker"]: r["name"] for r in rows}


def upsert_company_metadata(
    conn: sqlite3.Connection,
    ticker: str,
    *,
    securities_report_url: str | None = None,
    address_source_urls: str | None = None,
) -> None:
    now = _now()
    conn.execute(
        """
        UPDATE stocks SET
            securities_report_url = COALESCE(?, stocks.securities_report_url),
            address_source_urls   = COALESCE(?, stocks.address_source_urls),
            updated_at            = ?
        WHERE ticker = ?
        """,
        (securities_report_url, address_source_urls, now, ticker),
    )
