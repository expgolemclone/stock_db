from __future__ import annotations

import sqlite3
from collections.abc import Iterable
from datetime import date, datetime, timedelta, timezone
from typing import TypedDict
from zoneinfo import ZoneInfo

from stock_db.market_calendar import is_jpx_business_day
from stock_db.storage._refs import ensure_data_source
from stock_db.storage._util import utc_now_iso

STOOQ_PRICE_REFRESH_SOURCE = "stooq_prices"
STOOQ_PRICE_SOURCE = "stooq"
YAHOO_PRICE_SOURCE = "yahoo_finance_jp"
STOOQ_PRICE_REFRESH_COOLDOWN = timedelta(days=1)
STOOQ_LAGGED_PRICE_REFRESH_COOLDOWN = timedelta(hours=1)
PRICE_REFRESH_SOURCE = "price_refresh"
PRICE_REFRESH_COOLDOWN = timedelta(days=1)


def _ensure_prices_fresh_for_api(conn: sqlite3.Connection) -> None:
    from stock_db.sources.price_refresh import ensure_prices_fresh_for_api

    ensure_prices_fresh_for_api(conn)


def upsert_price(
    conn: sqlite3.Connection,
    ticker: str,
    price_date: str,
    close: float,
    volume: int | None,
    *,
    source: str,
) -> None:
    date.fromisoformat(price_date)
    if close <= 0:
        raise ValueError("close must be greater than zero")
    if volume is not None and volume < 0:
        raise ValueError("volume must be non-negative")
    if not source:
        raise ValueError("price source must not be empty")
    stock_exists = conn.execute(
        "SELECT 1 FROM stock_entities WHERE ticker = ?",
        (ticker,),
    ).fetchone()
    if stock_exists is None:
        raise ValueError(f"unknown stock ticker: {ticker}")

    source_id = ensure_data_source(conn, source)
    conn.execute(
        """
        INSERT INTO price_observations
            (ticker, date, source_id, close, volume, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, date, source_id) DO UPDATE SET
            close = excluded.close,
            volume = COALESCE(excluded.volume, price_observations.volume),
            updated_at = excluded.updated_at
        """,
        (ticker, price_date, source_id, close, volume, utc_now_iso()),
    )


def record_unresolved_price(
    conn: sqlite3.Connection,
    ticker: str,
    price_date: str,
    close: float,
    volume: int | None,
    *,
    source: str,
    reason: str,
) -> None:
    date.fromisoformat(price_date)
    if close <= 0:
        raise ValueError("close must be greater than zero")
    if volume is not None and volume < 0:
        raise ValueError("volume must be non-negative")
    if not source:
        raise ValueError("price source must not be empty")
    if not reason:
        raise ValueError("unresolved price reason must not be empty")
    source_id = ensure_data_source(conn, source)
    conn.execute(
        """
        INSERT INTO unresolved_price_observations
            (ticker, date, source_id, close, volume, reason, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(ticker, date, source_id) DO UPDATE SET
            close = excluded.close,
            volume = COALESCE(excluded.volume, unresolved_price_observations.volume),
            reason = excluded.reason,
            updated_at = excluded.updated_at
        """,
        (ticker, price_date, source_id, close, volume, reason, utc_now_iso()),
    )


def upsert_shares_outstanding(
    conn: sqlite3.Connection,
    ticker: str,
    shares: int,
) -> None:
    now = utc_now_iso()
    conn.execute(
        """
        INSERT INTO stock_entities (ticker, name, sector, market, updated_at)
        VALUES (?, '', '', '', ?)
        ON CONFLICT(ticker) DO UPDATE SET
            updated_at = excluded.updated_at
        """,
        (ticker, now),
    )
    conn.execute(
        """
        INSERT INTO stock_share_counts
            (ticker, shares_outstanding, shares_updated_at, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(ticker) DO UPDATE SET
            shares_outstanding = excluded.shares_outstanding,
            shares_updated_at = excluded.shares_updated_at,
            updated_at = excluded.updated_at
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
    _ensure_prices_fresh_for_api(conn)
    row = conn.execute(
        "SELECT close FROM prices WHERE ticker = ? ORDER BY date DESC LIMIT 1",
        (ticker,),
    ).fetchone()
    return row["close"] if row else None


def get_latest_price_date(conn: sqlite3.Connection) -> date | None:
    row = conn.execute("SELECT MAX(date) AS latest_date FROM prices").fetchone()
    if row is None or row["latest_date"] is None:
        return None
    return date.fromisoformat(row["latest_date"])


class PriceWithShares(TypedDict):
    price: float | None
    price_date: str | None
    shares_outstanding: int | None
    updated_at: str | None


def get_latest_price_with_shares(
    conn: sqlite3.Connection,
    ticker: str,
) -> PriceWithShares:
    _ensure_prices_fresh_for_api(conn)
    price_row = conn.execute(
        "SELECT close, date, updated_at FROM prices WHERE ticker = ? ORDER BY date DESC LIMIT 1",
        (ticker,),
    ).fetchone()
    shares_row = conn.execute(
        "SELECT shares_outstanding FROM stocks WHERE ticker = ?",
        (ticker,),
    ).fetchone()
    return PriceWithShares(
        price=price_row["close"] if price_row else None,
        price_date=price_row["date"] if price_row else None,
        shares_outstanding=shares_row["shares_outstanding"] if shares_row else None,
        updated_at=price_row["updated_at"] if price_row else None,
    )


def get_price_at_or_before(
    conn: sqlite3.Connection,
    ticker: str,
    date_str: str,
) -> float | None:
    """Return the closing price on or before *date_str* (YYYY-MM-DD)."""
    _ensure_prices_fresh_for_api(conn)
    row = conn.execute(
        "SELECT close FROM prices WHERE ticker = ? AND date <= ? ORDER BY date DESC LIMIT 1",
        (ticker, date_str),
    ).fetchone()
    return row["close"] if row else None


def is_price_stale(updated_at: str | None, stale_days: int) -> bool:
    if updated_at is None:
        return True
    ts = datetime.fromisoformat(updated_at)
    return datetime.now(timezone.utc) - ts > timedelta(days=stale_days)


def get_fresh_price_tickers(conn: sqlite3.Connection, stale_days: int) -> set[str]:
    _ensure_prices_fresh_for_api(conn)
    threshold = (
        datetime.now(timezone.utc) - timedelta(days=stale_days)
    ).isoformat()
    rows = conn.execute(
        "SELECT DISTINCT ticker FROM prices WHERE updated_at > ?",
        (threshold,),
    ).fetchall()
    return {r[0] for r in rows}


def _today_jst() -> date:
    return datetime.now(ZoneInfo("Asia/Tokyo")).date()


def get_latest_jpx_business_day(*, today: date | None = None) -> date:
    """Return the latest JPX business day on or before *today*."""
    current = today if today is not None else _today_jst()
    while not is_jpx_business_day(current):
        current -= timedelta(days=1)
    return current


def get_previous_jpx_business_day(*, today: date | None = None) -> date:
    """Return the latest completed JPX business day before *today*."""
    current = today if today is not None else _today_jst()
    return get_latest_jpx_business_day(today=current - timedelta(days=1))


def get_stale_price_tickers(
    conn: sqlite3.Connection,
    *,
    target_date: date,
    tickers: Iterable[str] | None = None,
) -> list[str]:
    """Return tickers whose latest stored price date is older than *target_date*."""
    target = target_date.isoformat()
    if tickers is None:
        rows = conn.execute(
            """
            WITH latest_price AS (
                SELECT ticker, MAX(date) AS latest_date
                FROM prices
                GROUP BY ticker
            )
            SELECT s.ticker
            FROM stocks s
            LEFT JOIN latest_price lp
              ON lp.ticker = s.ticker
            WHERE lp.latest_date IS NULL
               OR lp.latest_date < ?
            ORDER BY s.ticker
            """,
            (target,),
        ).fetchall()
        return [row["ticker"] for row in rows]

    ticker_list = [str(ticker) for ticker in tickers]
    if not ticker_list:
        return []

    placeholders = ", ".join("?" for _ in ticker_list)
    rows = conn.execute(
        f"""
        WITH latest_price AS (
            SELECT ticker, MAX(date) AS latest_date
            FROM prices
            GROUP BY ticker
        )
        SELECT s.ticker
        FROM stocks s
        LEFT JOIN latest_price lp
          ON lp.ticker = s.ticker
        WHERE s.ticker IN ({placeholders})
          AND (lp.latest_date IS NULL OR lp.latest_date < ?)
        ORDER BY s.ticker
        """,
        (*ticker_list, target),
    ).fetchall()
    return [row["ticker"] for row in rows]


def _get_source_refresh_check(
    conn: sqlite3.Connection,
    source: str,
) -> tuple[datetime, date] | None:
    try:
        row = conn.execute(
            "SELECT checked_at, target_date FROM source_refresh_log WHERE source = ?",
            (source,),
        ).fetchone()
    except sqlite3.OperationalError as exc:
        if "no such table" in str(exc):
            return None
        raise

    if row is None:
        return None

    checked_at = datetime.fromisoformat(row["checked_at"])
    if checked_at.tzinfo is None:
        checked_at = checked_at.replace(tzinfo=timezone.utc)
    return checked_at, date.fromisoformat(row["target_date"])


def get_stooq_price_update_checked_at(conn: sqlite3.Connection) -> datetime | None:
    check = _get_source_refresh_check(conn, STOOQ_PRICE_REFRESH_SOURCE)
    return check[0] if check is not None else None


def get_price_refresh_checked_at(conn: sqlite3.Connection) -> datetime | None:
    check = _get_source_refresh_check(conn, PRICE_REFRESH_SOURCE)
    return check[0] if check is not None else None


def record_stooq_price_update_check(
    conn: sqlite3.Connection,
    *,
    target_date: date,
    checked_at: datetime | None = None,
) -> None:
    _record_source_refresh_check(
        conn,
        source=STOOQ_PRICE_REFRESH_SOURCE,
        target_date=target_date,
        checked_at=checked_at,
    )


def record_price_refresh_check(
    conn: sqlite3.Connection,
    *,
    target_date: date,
    checked_at: datetime | None = None,
) -> None:
    _record_source_refresh_check(
        conn,
        source=PRICE_REFRESH_SOURCE,
        target_date=target_date,
        checked_at=checked_at,
    )


def _record_source_refresh_check(
    conn: sqlite3.Connection,
    *,
    source: str,
    target_date: date,
    checked_at: datetime | None = None,
) -> None:
    if checked_at is None:
        checked_at = datetime.now(timezone.utc)
    elif checked_at.tzinfo is None:
        checked_at = checked_at.replace(tzinfo=timezone.utc)

    source_id = ensure_data_source(conn, source)
    conn.execute(
        """
        INSERT INTO source_refresh_checks (source_id, target_date, checked_at)
        VALUES (?, ?, ?)
        ON CONFLICT(source_id) DO UPDATE SET
            target_date=excluded.target_date,
            checked_at=excluded.checked_at
        """,
        (
            source_id,
            target_date.isoformat(),
            checked_at.astimezone(timezone.utc).isoformat(),
        ),
    )


def _has_recent_source_refresh_check(
    conn: sqlite3.Connection,
    *,
    source: str,
    target_date: date,
    now: datetime,
    cooldown: timedelta,
) -> bool:
    check = _get_source_refresh_check(conn, source)
    if check is None:
        return False
    checked_at, checked_target_date = check
    if checked_target_date != target_date:
        return False
    return now.astimezone(timezone.utc) - checked_at.astimezone(timezone.utc) < cooldown


def has_recent_price_refresh_check(
    conn: sqlite3.Connection,
    *,
    target_date: date,
    now: datetime | None = None,
) -> bool:
    if now is None:
        now = datetime.now(timezone.utc)
    elif now.tzinfo is None:
        now = now.replace(tzinfo=timezone.utc)

    return _has_recent_source_refresh_check(
        conn,
        source=PRICE_REFRESH_SOURCE,
        target_date=target_date,
        now=now,
        cooldown=PRICE_REFRESH_COOLDOWN,
    )


def is_stooq_price_update_required(
    conn: sqlite3.Connection,
    *,
    today: date | None = None,
    now: datetime | None = None,
) -> bool:
    if now is None:
        now = datetime.now(timezone.utc)
    elif now.tzinfo is None:
        now = now.replace(tzinfo=timezone.utc)

    target_date = get_latest_jpx_business_day(today=today)
    stale_tickers = get_stale_price_tickers(conn, target_date=target_date)
    if not stale_tickers:
        return False

    cooldown = (
        STOOQ_PRICE_REFRESH_COOLDOWN
        if get_latest_price_date(conn) is None
        else STOOQ_LAGGED_PRICE_REFRESH_COOLDOWN
    )
    return not _has_recent_source_refresh_check(
        conn,
        source=STOOQ_PRICE_REFRESH_SOURCE,
        target_date=target_date,
        now=now,
        cooldown=cooldown,
    )
