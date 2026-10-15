from __future__ import annotations

import sqlite3
from datetime import date, datetime, timezone

import pytest

from stock_db.storage import prices as prices_module
from stock_db.storage.prices import (
    get_fresh_price_tickers,
    get_latest_price,
    get_latest_price_date,
    get_latest_price_with_shares,
    get_previous_jpx_business_day,
    get_stooq_price_update_checked_at,
    get_stale_price_tickers,
    get_tickers_with_shares,
    is_price_stale,
    is_stooq_price_update_required,
    record_stooq_price_update_check,
    upsert_price as _upsert_price,
    upsert_shares_outstanding,
)
from stock_db.storage.stocks import upsert_stock


def upsert_price(
    conn: sqlite3.Connection,
    ticker: str,
    price_date: str,
    close: float,
    volume: int | None,
    *,
    source: str,
) -> None:
    upsert_stock(conn, ticker, ticker, "", "")
    _upsert_price(
        conn,
        ticker,
        price_date,
        close,
        volume,
        source=source,
    )


class TestPrices:
    def test_upsert_and_get_latest(
        self,
        db_conn: sqlite3.Connection,
        monkeypatch,
    ) -> None:
        checked: list[sqlite3.Connection] = []
        monkeypatch.setattr(
            prices_module,
            "_ensure_prices_fresh_for_api",
            lambda conn: checked.append(conn),
        )

        upsert_price(db_conn, "1234", "2024-01-01", 100.0, 1000, source="test")
        upsert_price(db_conn, "1234", "2024-01-02", 110.0, 2000, source="test")
        db_conn.commit()

        result = get_latest_price(db_conn, "1234")

        assert result == 110.0
        assert checked == [db_conn]

    def test_returns_none_for_missing(self, db_conn: sqlite3.Connection) -> None:
        assert get_latest_price(db_conn, "9999") is None

    def test_source_priority_is_independent_of_write_order(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_stock(db_conn, "1234", "Example", "", "")
        _upsert_price(
            db_conn,
            "1234",
            "2026-05-08",
            101.0,
            None,
            source="stooq",
        )
        _upsert_price(
            db_conn,
            "1234",
            "2026-05-08",
            100.0,
            1_000,
            source="yahoo_finance_jp",
        )

        observations = db_conn.execute(
            """
            SELECT ds.source_code, po.close, po.volume
            FROM price_observations AS po
            JOIN data_sources AS ds ON ds.source_id = po.source_id
            ORDER BY ds.source_code
            """
        ).fetchall()
        canonical = db_conn.execute(
            "SELECT close, source FROM prices WHERE ticker = '1234'"
        ).fetchone()

        assert [tuple(row) for row in observations] == [
            ("stooq", 101.0, None),
            ("yahoo_finance_jp", 100.0, 1_000),
        ]
        assert tuple(canonical) == (101.0, "stooq")

    def test_rejects_unknown_ticker(self, db_conn: sqlite3.Connection) -> None:
        with pytest.raises(ValueError, match="unknown stock ticker"):
            _upsert_price(
                db_conn,
                "9999",
                "2026-05-08",
                100.0,
                None,
                source="test",
            )


class TestGetLatestPriceDate:
    def test_returns_none_when_prices_are_empty(self, db_conn: sqlite3.Connection) -> None:
        assert get_latest_price_date(db_conn) is None

    def test_returns_max_price_date(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        upsert_price(db_conn, "5678", "2026-05-11", 200.0, 1000, source="test")
        db_conn.commit()

        assert get_latest_price_date(db_conn) == date(2026, 5, 11)


class TestShares:
    def test_upsert_shares(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "テスト", "", "")
        upsert_shares_outstanding(db_conn, "1234", 1_000_000)
        db_conn.commit()

        result = get_tickers_with_shares(db_conn)
        assert "1234" in result

    def test_upsert_shares_creates_bare_row(self, db_conn: sqlite3.Connection) -> None:
        upsert_shares_outstanding(db_conn, "9999", 500_000)
        db_conn.commit()

        row = db_conn.execute("SELECT * FROM stocks WHERE ticker='9999'").fetchone()
        assert row["shares_outstanding"] == 500_000
        assert row["name"] == ""

    def test_get_latest_price_with_shares(
        self,
        db_conn: sqlite3.Connection,
        monkeypatch,
    ) -> None:
        checked: list[sqlite3.Connection] = []
        monkeypatch.setattr(
            prices_module,
            "_ensure_prices_fresh_for_api",
            lambda conn: checked.append(conn),
        )

        upsert_shares_outstanding(db_conn, "1234", 1_000_000)
        upsert_price(db_conn, "1234", "2024-01-01", 500.0, 100, source="test")
        db_conn.commit()

        result = get_latest_price_with_shares(db_conn, "1234")

        assert result["price"] == 500.0
        assert result["price_date"] == "2024-01-01"
        assert result["shares_outstanding"] == 1_000_000
        assert checked == [db_conn]


class TestIsPriceStale:
    def test_none_is_stale(self) -> None:
        assert is_price_stale(None, stale_days=1) is True

    def test_recent_is_not_stale(self) -> None:
        now = datetime.now(timezone.utc).isoformat()

        assert is_price_stale(now, stale_days=1) is False


class TestGetFreshPriceTickers:
    def test_returns_fresh_tickers(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2024-01-01", 100.0, 1000, source="test")
        db_conn.commit()

        result = get_fresh_price_tickers(db_conn, stale_days=99999)

        assert "1234" in result


class TestPriceFreshnessTarget:
    def test_uses_previous_business_day(self) -> None:
        assert get_previous_jpx_business_day(today=date(2026, 5, 11)) == date(2026, 5, 8)

    def test_skips_jpx_holidays(self) -> None:
        assert get_previous_jpx_business_day(today=date(2026, 5, 7)) == date(2026, 5, 1)

    def test_returns_tickers_older_than_target_date(self, db_conn: sqlite3.Connection) -> None:
        upsert_stock(db_conn, "1234", "fresh", "", "")
        upsert_stock(db_conn, "5678", "old", "", "")
        upsert_stock(db_conn, "9999", "missing", "", "")
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        upsert_price(db_conn, "5678", "2026-05-07", 100.0, 1000, source="test")
        db_conn.commit()

        result = get_stale_price_tickers(db_conn, target_date=date(2026, 5, 8))

        assert result == ["5678", "9999"]


class TestIsStooqPriceUpdateRequired:
    def test_empty_prices_require_update(self, db_conn: sqlite3.Connection) -> None:
        assert is_stooq_price_update_required(db_conn, today=date(2026, 5, 10)) is True

    def test_sunday_after_friday_is_fresh(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        db_conn.commit()

        assert is_stooq_price_update_required(db_conn, today=date(2026, 5, 10)) is False

    def test_monday_after_friday_requires_update(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        db_conn.commit()

        assert is_stooq_price_update_required(db_conn, today=date(2026, 5, 11)) is True

    def test_one_fresh_ticker_does_not_hide_other_stale_tickers(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_price(db_conn, "1234", "2026-05-11", 100.0, 1000, source="test")
        upsert_price(db_conn, "5678", "2026-05-08", 100.0, 1000, source="test")
        db_conn.commit()

        assert is_stooq_price_update_required(db_conn, today=date(2026, 5, 11)) is True

    def test_recent_stooq_check_suppresses_same_target_retry(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        record_stooq_price_update_check(
            db_conn,
            target_date=date(2026, 5, 11),
            checked_at=datetime(2026, 5, 11, 0, 0, tzinfo=timezone.utc),
        )
        db_conn.commit()

        assert (
            is_stooq_price_update_required(
                db_conn,
                today=date(2026, 5, 11),
                now=datetime(2026, 5, 11, 0, 30, tzinfo=timezone.utc),
            )
            is False
        )

    def test_recent_check_for_old_target_does_not_suppress_new_target(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        record_stooq_price_update_check(
            db_conn,
            target_date=date(2026, 5, 11),
            checked_at=datetime(2026, 5, 12, 0, 0, tzinfo=timezone.utc),
        )
        db_conn.commit()

        assert (
            is_stooq_price_update_required(
                db_conn,
                today=date(2026, 5, 12),
                now=datetime(2026, 5, 12, 0, 30, tzinfo=timezone.utc),
            )
            is True
        )

    def test_lagged_old_stooq_check_allows_update(
        self,
        db_conn: sqlite3.Connection,
    ) -> None:
        upsert_price(db_conn, "1234", "2026-05-08", 100.0, 1000, source="test")
        record_stooq_price_update_check(
            db_conn,
            target_date=date(2026, 5, 11),
            checked_at=datetime(2026, 5, 11, 0, 0, tzinfo=timezone.utc),
        )
        db_conn.commit()

        assert (
            is_stooq_price_update_required(
                db_conn,
                today=date(2026, 5, 11),
                now=datetime(2026, 5, 11, 2, 0, tzinfo=timezone.utc),
            )
            is True
        )

    def test_jpx_holidays_after_latest_price_are_fresh(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2026-05-01", 100.0, 1000, source="test")
        db_conn.commit()

        assert is_stooq_price_update_required(db_conn, today=date(2026, 5, 6)) is False

    def test_missing_holiday_config_year_raises(self, db_conn: sqlite3.Connection) -> None:
        upsert_price(db_conn, "1234", "2027-12-31", 100.0, 1000, source="test")
        db_conn.commit()

        try:
            is_stooq_price_update_required(db_conn, today=date(2028, 1, 1))
        except ValueError as exc:
            assert "2028" in str(exc)
        else:
            raise AssertionError("expected missing JPX holiday config to raise")


class TestStooqPriceUpdateCheck:
    def test_records_latest_check_time(self, db_conn: sqlite3.Connection) -> None:
        checked_at = datetime(2026, 5, 11, 0, 0, tzinfo=timezone.utc)

        record_stooq_price_update_check(
            db_conn,
            target_date=date(2026, 5, 11),
            checked_at=checked_at,
        )
        db_conn.commit()

        assert get_stooq_price_update_checked_at(db_conn) == checked_at
