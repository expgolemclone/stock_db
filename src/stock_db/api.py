from __future__ import annotations

import sqlite3
from collections.abc import Sequence
from datetime import date
from typing import TypedDict

from stock_db.paths import STOCKS_DB_PATH
from stock_db.sources.price_refresh import (
    PriceRefreshCommandResult,
    PriceRefreshError,
    ensure_prices_fresh_for_api,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import get_financial_dict, get_historical_items
from stock_db.storage.prices import get_latest_price_date, get_previous_jpx_business_day
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import get_all_tickers as _get_all_tickers
from stock_db.storage.stocks import get_stock_names as _get_stock_names


class HistoricalItems(TypedDict):
    period: str
    items: dict[str, float | None]


class ScreeningStock(TypedDict):
    ticker: str
    name: str
    price: float | None
    price_date: str | None
    shares_outstanding: int | None
    financials: dict[str, dict[str, float | None]]
    cf_history: list[tuple[str, dict[str, float | None]]]
    pl_history: list[tuple[str, dict[str, float | None]]]


class StockPriceMetadata(TypedDict):
    price_date: str | None
    target_price_date: str


class ValidationTarget(TypedDict):
    ticker: str
    name: str
    securities_report_url: str
    price: float
    shares_outstanding: int


def ensure_prices_fresh() -> PriceRefreshCommandResult | None:
    """Refresh the owned stock database if downstream API reads need fresh prices."""

    return ensure_prices_fresh_for_api(db_path=STOCKS_DB_PATH)


def get_all_tickers() -> list[str]:
    with _open_stock_db() as conn:
        return _get_all_tickers(conn)


def get_stock_names() -> dict[str, str]:
    with _open_stock_db() as conn:
        return _get_stock_names(conn)


def get_screening_tickers(limit: int | None = None) -> list[str]:
    """Return tickers with enough owned data to exercise screening consumers."""

    ensure_prices_fresh()
    sql = """
        SELECT s.ticker
        FROM stocks AS s
        WHERE EXISTS (SELECT 1 FROM prices AS p WHERE p.ticker = s.ticker)
          AND EXISTS (SELECT 1 FROM financial_items AS f WHERE f.ticker = s.ticker)
        ORDER BY s.ticker
    """
    params: tuple[int, ...] = ()
    if limit is not None:
        sql += " LIMIT ?"
        params = (limit,)

    with _open_stock_db() as conn:
        rows = conn.execute(sql, params).fetchall()
    return [str(row["ticker"]) for row in rows]


def load_screening_stocks(
    tickers: Sequence[str] | None = None,
    *,
    fcf_periods: int = 10,
    pl_periods: int = 6,
) -> list[ScreeningStock]:
    """Load the public screening data contract without exposing SQLite handles."""

    ensure_prices_fresh()
    with _open_stock_db() as conn:
        selected_tickers = list(tickers) if tickers is not None else _get_all_tickers(conn)
        names = _get_stock_names(conn)
        return [
            _build_screening_stock(
                conn,
                ticker=str(ticker),
                name=names.get(str(ticker), ""),
                fcf_periods=fcf_periods,
                pl_periods=pl_periods,
            )
            for ticker in selected_tickers
        ]


def get_stock_price_metadata() -> StockPriceMetadata:
    ensure_prices_fresh()
    with _open_stock_db() as conn:
        price_date: date | None = get_latest_price_date(conn)
    target_price_date = get_previous_jpx_business_day()
    return {
        "price_date": price_date.isoformat() if price_date is not None else None,
        "target_price_date": target_price_date.isoformat(),
    }


def get_validation_targets(limit: int) -> list[ValidationTarget]:
    ensure_prices_fresh()
    with _open_stock_db() as conn:
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
    return [
        {
            "ticker": str(row["ticker"]),
            "name": str(row["name"]),
            "securities_report_url": str(row["securities_report_url"]),
            "price": float(row["close"]),
            "shares_outstanding": int(row["shares_outstanding"]),
        }
        for row in rows
    ]


def get_latest_balance_sheet(
    ticker: str,
    *,
    source: str = "edinet_xbrl",
) -> tuple[str | None, dict[str, float | None], str | None]:
    with _open_stock_db() as conn:
        rows = conn.execute(
            """
            SELECT period, statement, item_name, value
            FROM financial_items
            WHERE ticker = ? AND source = ?
            ORDER BY period DESC, statement, item_name
            """,
            (ticker, source),
        ).fetchall()

    if not rows:
        return None, {}, "scrape_missing"

    status_rows = [row for row in rows if row["statement"] == "_status"]
    data_rows = [row for row in rows if row["statement"] == "bs"]
    if data_rows:
        latest_period = max(str(row["period"]) for row in data_rows)
        bs = {
            str(row["item_name"]): row["value"]
            for row in data_rows
            if str(row["period"]) == latest_period
        }
        return latest_period, bs, None
    if status_rows:
        return None, {}, f"scrape_{status_rows[0]['item_name']}"
    return None, {}, "scrape_missing"


def _open_stock_db() -> sqlite3.Connection:
    conn = get_connection(STOCKS_DB_PATH)
    init_db(conn)
    return conn


def _build_screening_stock(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    name: str,
    fcf_periods: int,
    pl_periods: int,
) -> ScreeningStock:
    financials = get_financial_dict(conn, ticker)
    price, price_date, shares_outstanding = _get_latest_price_with_shares(conn, ticker)
    return {
        "ticker": ticker,
        "name": name,
        "price": price,
        "price_date": price_date,
        "shares_outstanding": shares_outstanding,
        "financials": financials,
        "cf_history": get_historical_items(conn, ticker, "cf", n_periods=fcf_periods),
        "pl_history": get_historical_items(conn, ticker, "pl", n_periods=pl_periods),
    }


def _get_latest_price_with_shares(
    conn: sqlite3.Connection,
    ticker: str,
) -> tuple[float | None, str | None, int | None]:
    price_row = conn.execute(
        """
        SELECT close, date
        FROM prices
        WHERE ticker = ?
        ORDER BY date DESC
        LIMIT 1
        """,
        (ticker,),
    ).fetchone()
    shares_row = conn.execute(
        "SELECT shares_outstanding FROM stocks WHERE ticker = ?",
        (ticker,),
    ).fetchone()
    return (
        price_row["close"] if price_row else None,
        price_row["date"] if price_row else None,
        shares_row["shares_outstanding"] if shares_row else None,
    )
