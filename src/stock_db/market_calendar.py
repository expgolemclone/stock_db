from __future__ import annotations

from datetime import date
from functools import lru_cache

from stock_db.paths import jpx_market_holidays


@lru_cache(maxsize=None)
def _jpx_market_holidays_for_year(year: int) -> frozenset[date]:
    configured = jpx_market_holidays().get("holidays", {})
    raw_dates = configured.get(str(year))
    if raw_dates is None:
        raise ValueError(f"JPX market holidays are not configured for {year}")
    return frozenset(date.fromisoformat(raw_date) for raw_date in raw_dates)


def is_jpx_business_day(day: date) -> bool:
    holidays = _jpx_market_holidays_for_year(day.year)
    return day.weekday() < 5 and day not in holidays
