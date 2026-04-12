from __future__ import annotations

import sqlite3
from datetime import datetime, timezone


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


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
