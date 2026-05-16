from __future__ import annotations

import sqlite3
from datetime import date, datetime, timedelta, timezone
from typing import TypedDict
from zoneinfo import ZoneInfo

from stock_db.market_calendar import is_jpx_business_day
from stock_db.storage._util import utc_now_iso

STOOQ_PRICE_REFRESH_SOURCE = "stooq_prices"
STOOQ_PRICE_REFRESH_COOLDOWN = timedelta(days=1)


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
        (ticker, date, close, volume, utc_now_iso()),
    )


def upsert_shares_outstanding(
    conn: sqlite3.Connection,
    ticker: str,
    shares: int,
) -> None:
    now = utc_now_iso()
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


def get_latest_price_date(conn: sqlite3.Connection) -> date | None:
    row = conn.execute("SELECT MAX(date) AS latest_date FROM prices").fetchone()
    if row is None or row["latest_date"] is None:
        return None
    return date.fromisoformat(row["latest_date"])


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


def get_price_at_or_before(
    conn: sqlite3.Connection,
    ticker: str,
    date_str: str,
) -> float | None:
    """Return the closing price on or before *date_str* (YYYY-MM-DD)."""
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
    threshold = (
        datetime.now(timezone.utc) - timedelta(days=stale_days)
    ).isoformat()
    rows = conn.execute(
        "SELECT DISTINCT ticker FROM prices WHERE updated_at > ?",
        (threshold,),
    ).fetchall()
    return {r[0] for r in rows}


def get_stooq_price_update_checked_at(conn: sqlite3.Connection) -> datetime | None:
    try:
        row = conn.execute(
            "SELECT checked_at FROM source_refresh_log WHERE source = ?",
            (STOOQ_PRICE_REFRESH_SOURCE,),
        ).fetchone()
    except sqlite3.OperationalError as exc:
        if "no such table" in str(exc):
            return None
        raise

    if row is None:
        return None

    checked_at = datetime.fromisoformat(row["checked_at"])
    if checked_at.tzinfo is None:
        return checked_at.replace(tzinfo=timezone.utc)
    return checked_at


def record_stooq_price_update_check(
    conn: sqlite3.Connection,
    *,
    checked_at: datetime | None = None,
) -> None:
    if checked_at is None:
        checked_at = datetime.now(timezone.utc)
    elif checked_at.tzinfo is None:
        checked_at = checked_at.replace(tzinfo=timezone.utc)

    conn.execute(
        """
        INSERT INTO source_refresh_log (source, checked_at)
        VALUES (?, ?)
        ON CONFLICT(source) DO UPDATE SET
            checked_at=excluded.checked_at
        """,
        (STOOQ_PRICE_REFRESH_SOURCE, checked_at.astimezone(timezone.utc).isoformat()),
    )


def _has_recent_stooq_price_update_check(
    conn: sqlite3.Connection,
    *,
    now: datetime,
) -> bool:
    checked_at = get_stooq_price_update_checked_at(conn)
    if checked_at is None:
        return False
    return now.astimezone(timezone.utc) - checked_at.astimezone(timezone.utc) < STOOQ_PRICE_REFRESH_COOLDOWN


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

    latest_date = get_latest_price_date(conn)
    if latest_date is None:
        return not _has_recent_stooq_price_update_check(conn, now=now)

    if today is None:
        today = datetime.now(ZoneInfo("Asia/Tokyo")).date()

    if latest_date >= today:
        return False

    days_since_latest = (today - latest_date).days
    for offset in range(1, days_since_latest + 1):
        if is_jpx_business_day(latest_date + timedelta(days=offset)):
            return not _has_recent_stooq_price_update_check(conn, now=now)
    return False
