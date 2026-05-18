from __future__ import annotations

import sqlite3

from stock_db.storage._util import utc_now_iso


def _ensure_prices_fresh_for_api(conn: sqlite3.Connection) -> None:
    from stock_db.sources.price_refresh import ensure_prices_fresh_for_api

    ensure_prices_fresh_for_api(conn)


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
        (ticker, edinet_code, name, sector, market, utc_now_iso()),
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
) -> None:
    now = utc_now_iso()
    conn.execute(
        """
        UPDATE stocks SET
            securities_report_url = COALESCE(?, stocks.securities_report_url),
            updated_at            = ?
        WHERE ticker = ?
        """,
        (securities_report_url, now, ticker),
    )


def upsert_yf_suffix(conn: sqlite3.Connection, ticker: str, suffix: str) -> None:
    conn.execute(
        "UPDATE stocks SET yf_suffix = ?, updated_at = ? WHERE ticker = ?",
        (suffix, utc_now_iso(), ticker),
    )


def get_ticker_suffix_map(conn: sqlite3.Connection) -> dict[str, str]:
    rows = conn.execute(
        "SELECT ticker, yf_suffix FROM stocks WHERE yf_suffix IS NOT NULL"
    ).fetchall()
    return {r["ticker"]: r["yf_suffix"] for r in rows}


def get_validation_targets(
    conn: sqlite3.Connection,
    limit: int,
) -> list[sqlite3.Row]:
    """Return stocks with price data ordered by market cap (descending).

    Only includes stocks that have a securities_report_url, shares_outstanding,
    and a latest closing price.
    """
    _ensure_prices_fresh_for_api(conn)
    rows = conn.execute(
        """
        WITH latest_price AS (
            SELECT ticker, MAX(date) AS latest_date
            FROM prices
            GROUP BY ticker
        )
        SELECT
            s.ticker,
            s.name,
            s.securities_report_url,
            p.close,
            s.shares_outstanding
        FROM stocks s
        JOIN latest_price lp
          ON lp.ticker = s.ticker
        JOIN prices p
          ON p.ticker = lp.ticker
         AND p.date = lp.latest_date
        WHERE s.securities_report_url IS NOT NULL
          AND s.shares_outstanding IS NOT NULL
          AND p.close IS NOT NULL
        ORDER BY CAST(p.close * s.shares_outstanding AS REAL) DESC, s.ticker
        LIMIT ?
        """,
        (limit,),
    ).fetchall()
    return list(rows)
