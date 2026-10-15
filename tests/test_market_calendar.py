from __future__ import annotations

from datetime import date

import pytest

from stock_db.market_calendar import is_jpx_business_day


def test_weekday_regular_market_day_is_business_day() -> None:
    assert is_jpx_business_day(date(2026, 5, 7)) is True


def test_weekend_is_not_business_day() -> None:
    assert is_jpx_business_day(date(2026, 5, 9)) is False


def test_jpx_market_holiday_is_not_business_day() -> None:
    assert is_jpx_business_day(date(2026, 5, 6)) is False


def test_missing_config_year_raises() -> None:
    with pytest.raises(ValueError, match="2028"):
        is_jpx_business_day(date(2028, 1, 1))
