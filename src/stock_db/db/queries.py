from __future__ import annotations

import sqlite3
from datetime import datetime, timedelta, timezone
from typing import TypedDict


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# ---------------------------------------------------------------------------
# stocks
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# financial_items (EAV)
# ---------------------------------------------------------------------------


def upsert_financial_item(
    conn: sqlite3.Connection,
    ticker: str,
    period: str,
    statement: str,
    item_name: str,
    value: float | None,
    source: str,
) -> None:
    conn.execute(
        """
        INSERT INTO financial_items
            (ticker, period, statement, item_name, value, source, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, period, statement, item_name) DO UPDATE SET
            value=excluded.value,
            source=excluded.source,
            updated_at=excluded.updated_at
        """,
        (ticker, period, statement, item_name, value, source, _now()),
    )


def upsert_financial_items_bulk(
    conn: sqlite3.Connection,
    rows: list[dict],
) -> None:
    now = _now()
    conn.executemany(
        """
        INSERT INTO financial_items
            (ticker, period, statement, item_name, value, source, updated_at)
        VALUES (:ticker, :period, :statement, :item_name, :value, :source, :updated_at)
        ON CONFLICT(ticker, period, statement, item_name) DO UPDATE SET
            value=excluded.value,
            source=excluded.source,
            updated_at=excluded.updated_at
        """,
        [{**r, "updated_at": now} for r in rows],
    )


def get_financial_dict(
    conn: sqlite3.Connection,
    ticker: str,
    period: str | None = None,
) -> dict[str, dict[str, float | None]]:
    if period is None:
        row = conn.execute(
            """
            SELECT period FROM financial_items
            WHERE ticker = ? AND statement = 'pl'
            ORDER BY period DESC LIMIT 1
            """,
            (ticker,),
        ).fetchone()
        if row is None:
            return {}
        period = row["period"]

    rows = conn.execute(
        """
        SELECT statement, item_name, value
        FROM financial_items
        WHERE ticker = ? AND period = ?
        """,
        (ticker, period),
    ).fetchall()

    result: dict[str, dict[str, float | None]] = {
        "pl": {}, "bs": {}, "cf": {}, "dividend": {}, "ss": {}, "forecast": {},
    }
    for r in rows:
        stmt = r["statement"]
        result.setdefault(stmt, {})[r["item_name"]] = r["value"]

    forecast_rows = conn.execute(
        """
        SELECT item_name, value FROM financial_items
        WHERE ticker = ? AND statement = 'forecast'
          AND period = (
              SELECT MAX(period) FROM financial_items
              WHERE ticker = ? AND statement = 'forecast'
          )
        """,
        (ticker, ticker),
    ).fetchall()
    for r in forecast_rows:
        result["forecast"][r["item_name"]] = r["value"]

    return result


def get_historical_items(
    conn: sqlite3.Connection,
    ticker: str,
    statement: str,
    n_periods: int = 5,
) -> list[tuple[str, dict[str, float | None]]]:
    rows = conn.execute(
        """
        SELECT period, item_name, value FROM financial_items
        WHERE ticker = ? AND statement = ?
          AND period IN (
              SELECT DISTINCT period FROM financial_items
              WHERE ticker = ? AND statement = ?
              ORDER BY period DESC LIMIT ?
          )
        ORDER BY period DESC
        """,
        (ticker, statement, ticker, statement, n_periods),
    ).fetchall()

    grouped: dict[str, dict[str, float | None]] = {}
    for r in rows:
        grouped.setdefault(r["period"], {})[r["item_name"]] = r["value"]

    return [(period, items) for period, items in sorted(grouped.items(), reverse=True)]


def get_cached_periods(
    conn: sqlite3.Connection,
    ticker: str,
    statement: str,
) -> set[str]:
    rows = conn.execute(
        """
        SELECT DISTINCT period FROM financial_items
        WHERE ticker = ? AND statement = ?
        """,
        (ticker, statement),
    ).fetchall()
    return {r["period"] for r in rows}


# ---------------------------------------------------------------------------
# prices
# ---------------------------------------------------------------------------


def upsert_price(
    conn: sqlite3.Connection,
    ticker: str,
    date: str,
    close: float | None,
    volume: int | None,
) -> None:
    conn.execute(
        """
        INSERT INTO prices (ticker, date, close, volume, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(ticker, date) DO UPDATE SET
            close=excluded.close,
            volume=excluded.volume,
            updated_at=excluded.updated_at
        """,
        (ticker, date, close, volume, _now()),
    )


def upsert_shares_outstanding(
    conn: sqlite3.Connection,
    ticker: str,
    shares: int,
) -> None:
    now = _now()
    conn.execute(
        """
        INSERT INTO stocks (ticker, name, sector, market, shares_outstanding, shares_updated_at, updated_at)
        VALUES (?, '', '', '', ?, ?, ?)
        ON CONFLICT(ticker) DO UPDATE SET
            shares_outstanding = excluded.shares_outstanding,
            shares_updated_at  = excluded.shares_updated_at,
            updated_at         = excluded.updated_at
        """,
        (ticker, shares, now, now),
    )


def get_tickers_with_shares(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute(
        "SELECT ticker FROM stocks WHERE shares_outstanding IS NOT NULL"
    ).fetchall()
    return {r["ticker"] for r in rows}


def get_latest_price(
    conn: sqlite3.Connection,
    ticker: str,
) -> float | None:
    row = conn.execute(
        "SELECT close FROM prices WHERE ticker = ? ORDER BY date DESC LIMIT 1",
        (ticker,),
    ).fetchone()
    return row["close"] if row else None


class PriceWithShares(TypedDict):
    price: float | None
    shares_outstanding: int | None
    updated_at: str | None


def get_latest_price_with_shares(
    conn: sqlite3.Connection,
    ticker: str,
) -> PriceWithShares:
    price_row = conn.execute(
        "SELECT close, updated_at FROM prices WHERE ticker = ? ORDER BY date DESC LIMIT 1",
        (ticker,),
    ).fetchone()
    shares_row = conn.execute(
        "SELECT shares_outstanding FROM stocks WHERE ticker = ?",
        (ticker,),
    ).fetchone()
    return PriceWithShares(
        price=price_row["close"] if price_row else None,
        shares_outstanding=shares_row["shares_outstanding"] if shares_row else None,
        updated_at=price_row["updated_at"] if price_row else None,
    )


def is_price_stale(updated_at: str | None, stale_days: int) -> bool:
    if updated_at is None:
        return True
    ts = datetime.fromisoformat(updated_at)
    return datetime.now(timezone.utc) - ts > timedelta(days=stale_days)


def get_fresh_price_tickers(conn: sqlite3.Connection, stale_days: int) -> set[str]:
    threshold = (
        datetime.now(timezone.utc) - timedelta(days=stale_days)
    ).isoformat()
    rows = conn.execute(
        "SELECT DISTINCT ticker FROM prices WHERE updated_at > ?",
        (threshold,),
    ).fetchall()
    return {r[0] for r in rows}


# ---------------------------------------------------------------------------
# market_cap
# ---------------------------------------------------------------------------


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
