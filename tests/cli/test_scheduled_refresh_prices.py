from __future__ import annotations

from datetime import date, datetime
from zoneinfo import ZoneInfo

import pytest

from stock_db.cli import scheduled_refresh_prices as cli_module
from stock_db.sources import price_refresh as refresh_module


def test_noop_before_16_00_jst(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 7, 15, 59, tzinfo=tz)),
        }),
    )

    def unexpected_refresh(**kwargs: object) -> object:
        raise AssertionError("unexpected refresh")

    monkeypatch.setattr(cli_module, "refresh_prices", unexpected_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 0
    assert "before 16:00 JST" in output.err


def test_noop_on_weekend(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # 2026-05-09 is Saturday
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 9, 16, 0, tzinfo=tz)),
        }),
    )

    def unexpected_refresh(**kwargs: object) -> object:
        raise AssertionError("unexpected refresh")

    monkeypatch.setattr(cli_module, "refresh_prices", unexpected_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 0
    assert "not a JPX business day" in output.err


def test_noop_on_jpx_holiday(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # 2026-05-06 is a JPX holiday (Constitution Memorial Day observed)
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 6, 16, 0, tzinfo=tz)),
        }),
    )

    def unexpected_refresh(**kwargs: object) -> object:
        raise AssertionError("unexpected refresh")

    monkeypatch.setattr(cli_module, "refresh_prices", unexpected_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 0
    assert "not a JPX business day" in output.err


def test_runs_on_business_day_after_16(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # 2026-05-07 is a Thursday (business day)
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 7, 16, 30, tzinfo=tz)),
        }),
    )

    captured: dict[str, object] = {}

    def fake_refresh(**kwargs: object) -> None:
        captured.update(kwargs)
        return None

    monkeypatch.setattr(cli_module, "refresh_prices", fake_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 0
    assert captured == {
        "target_date": date(2026, 5, 7),
        "if_needed": True,
        "headless": True,
    }
    assert "no update needed" in output.err


def test_returns_1_when_refresh_leaves_unresolved_tickers(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 7, 16, 30, tzinfo=tz)),
        }),
    )

    def fake_refresh(**kwargs: object) -> refresh_module.PriceRefreshResult:
        del kwargs
        return refresh_module.PriceRefreshResult(
            target_date=date(2026, 5, 7),
            stale_before=1,
            stale_after_stooq=1,
            stale_after_yahoo=1,
            unresolved_tickers=("7203",),
            stooq_result=None,
            yahoo_ok=0,
            yahoo_errors=0,
            yahoo_skipped_reason="stooq_latest_date=2026-05-01 is older than target_date=2026-05-07",
        )

    monkeypatch.setattr(cli_module, "refresh_prices", fake_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()

    assert rc == 1
    assert "unresolved_stale=1 (7203)" in output.err


def test_exit_1_on_missing_holiday_year(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # 2028-01-01 has no holiday config
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2028, 1, 1, 16, 0, tzinfo=tz)),
        }),
    )

    def unexpected_refresh(**kwargs: object) -> object:
        raise AssertionError("unexpected refresh")

    monkeypatch.setattr(cli_module, "refresh_prices", unexpected_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 1
    assert "2028" in output.err


def test_exit_1_on_refresh_failure(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        cli_module,
        "datetime",
        type("_DT", (), {
            "now": staticmethod(lambda tz: datetime(2026, 5, 7, 16, 30, tzinfo=tz)),
        }),
    )

    def failing_refresh(**kwargs: object) -> None:
        raise refresh_module.PriceRefreshError("Stooq failed")

    monkeypatch.setattr(cli_module, "refresh_prices", failing_refresh)

    rc = cli_module.main(["--headless"])
    output = capsys.readouterr()
    assert rc == 1
    assert "Stooq failed" in output.err
