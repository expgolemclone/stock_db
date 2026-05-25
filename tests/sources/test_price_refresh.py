from __future__ import annotations

import subprocess
from datetime import date
from pathlib import Path

import pytest

from stock_db.sources import price_refresh as refresh_module
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import record_price_refresh_check, upsert_price
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import upsert_stock, upsert_yf_suffix


class FakeBrowserServiceClient:
    def __init__(self, *, config: dict[str, object]) -> None:
        self.config = config

    def __enter__(self) -> "FakeBrowserServiceClient":
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        return None


def _init_price_db(
    db_path: Path,
    rows: dict[str, str | None],
    *,
    suffixes: dict[str, str] | None = None,
) -> None:
    conn = get_connection(db_path)
    try:
        init_db(conn)
        for ticker, latest_date in rows.items():
            upsert_stock(conn, ticker, ticker, "", "")
            if suffixes is not None and ticker in suffixes:
                upsert_yf_suffix(conn, ticker, suffixes[ticker])
            if latest_date is not None:
                upsert_price(conn, ticker, latest_date, 100.0, 1000)
        conn.commit()
    finally:
        conn.close()


def test_refresh_prices_skips_when_if_needed_and_all_tickers_are_fresh(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-08"})

    def unexpected_update_stooq_daily_prices(**kwargs: object) -> object:
        del kwargs
        raise AssertionError("unexpected Stooq update")

    monkeypatch.setattr(
        refresh_module,
        "update_stooq_daily_prices",
        unexpected_update_stooq_daily_prices,
    )

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is None


def test_refresh_prices_uses_yahoo_for_stale_tickers_after_stooq(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(
        db_path,
        {"1234": "2026-05-07", "9999": "2026-05-08"},
        suffixes={"1234": "N"},
    )
    captured: dict[str, object] = {}

    def fake_scrape_and_store(
        client: object,
        conn: object,
        tickers: list[str],
        *,
        skip_existing: bool,
        min_date: str | None,
        fail_fast: bool,
        allowed_suffixes: tuple[str, ...],
        discover_missing_suffix: bool,
    ) -> tuple[int, int]:
        del client
        captured["tickers"] = tickers
        captured["skip_existing"] = skip_existing
        captured["min_date"] = min_date
        captured["fail_fast"] = fail_fast
        captured["allowed_suffixes"] = allowed_suffixes
        captured["discover_missing_suffix"] = discover_missing_suffix
        upsert_price(conn, "1234", "2026-05-08", 110.0, 1000)
        return 1, 0

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )
    monkeypatch.setattr(refresh_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(refresh_module, "scrape_and_store", fake_scrape_and_store)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert result.yahoo_ok == 1
    assert result.stale_after_stooq == 1
    assert captured == {
        "tickers": ["1234"],
        "skip_existing": False,
        "min_date": "2026-05-08",
        "fail_fast": False,
        "allowed_suffixes": ("N", "S", "F"),
        "discover_missing_suffix": False,
    }


def test_refresh_prices_reports_unresolved_when_yahoo_leaves_stale_tickers(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(
        db_path,
        {"1234": "2026-05-07", "9999": "2026-05-08"},
        suffixes={"1234": "N"},
    )

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )
    monkeypatch.setattr(refresh_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(
        refresh_module,
        "scrape_and_store",
        lambda *_args, **_kwargs: (1, 0),
    )

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert result.stale_after_yahoo == 1
    assert result.unresolved_tickers == ("1234",)
    assert "unresolved_stale=1 (1234)" in refresh_module.describe_price_refresh_result(result)


def test_refresh_prices_forces_stooq_download_when_refreshing_stale_prices(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    captured: dict[str, object] = {}

    def fake_update_stooq_daily_prices(**kwargs: object) -> refresh_module.StooqDailyPriceUpdateResult:
        captured.update(kwargs)
        conn = get_connection(db_path)
        try:
            upsert_price(conn, "1234", "2026-05-08", 110.0, 1000)
            conn.commit()
        finally:
            conn.close()
        return refresh_module.StooqDailyPriceUpdateResult(
            imported=1,
            date="20260508",
            label="20260508_d",
            file_path=tmp_path / "raw" / "20260508_d.txt",
        )

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: True,
    )
    monkeypatch.setattr(
        refresh_module,
        "update_stooq_daily_prices",
        fake_update_stooq_daily_prices,
    )

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert result.stooq_result is not None
    assert captured["reuse_existing"] is False


def test_refresh_prices_only_uses_non_tse_yahoo_suffixes(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(
        db_path,
        {
            "1111": "2026-05-07",
            "2222": "2026-05-07",
            "3333": "2026-05-07",
            "9999": "2026-05-08",
        },
        suffixes={"1111": "T", "2222": "S"},
    )
    captured: dict[str, object] = {}

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )
    monkeypatch.setattr(refresh_module, "BrowserServiceClient", FakeBrowserServiceClient)

    def fake_scrape_and_store(
        client: object,
        conn: object,
        tickers: list[str],
        **kwargs: object,
    ) -> tuple[int, int]:
        del client, kwargs
        captured["tickers"] = tickers
        upsert_price(conn, "2222", "2026-05-08", 110.0, 1000)
        return 1, 0

    monkeypatch.setattr(refresh_module, "scrape_and_store", fake_scrape_and_store)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert captured["tickers"] == ["2222"]
    assert result.yahoo_ok == 1
    assert result.yahoo_skipped_tickers == 2
    assert result.unresolved_tickers == ("1111", "3333")


def test_refresh_prices_skips_yahoo_when_only_tse_or_unknown_suffixes_remain(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(
        db_path,
        {"1111": "2026-05-07", "2222": "2026-05-07", "9999": "2026-05-08"},
        suffixes={"1111": "T"},
    )

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )

    def unexpected_client(*_args: object, **_kwargs: object) -> object:
        raise AssertionError("unexpected Yahoo browser")

    monkeypatch.setattr(refresh_module, "BrowserServiceClient", unexpected_client)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert result.yahoo_ok == 0
    assert result.yahoo_skipped_reason == "no non-TSE Yahoo fallback tickers"
    assert result.yahoo_skipped_tickers == 2
    assert result.unresolved_tickers == ("1111", "2222")


def test_refresh_prices_skips_yahoo_when_stooq_has_not_reached_target_date(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07", "5678": "2026-05-07"})

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )

    def unexpected_scrape_and_store(*_args: object, **_kwargs: object) -> object:
        raise AssertionError("unexpected Yahoo refresh")

    monkeypatch.setattr(refresh_module, "scrape_and_store", unexpected_scrape_and_store)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert result.yahoo_ok == 0
    assert result.yahoo_skipped_reason == (
        "stooq_latest_date=2026-05-07 is older than target_date=2026-05-08"
    )
    assert result.unresolved_tickers == ("1234", "5678")
    assert "yahoo_skipped=stooq_latest_date=2026-05-07" in (
        refresh_module.describe_price_refresh_result(result)
    )


def test_refresh_prices_skips_recent_attempt_when_if_needed(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    conn = get_connection(db_path)
    try:
        record_price_refresh_check(conn)
        conn.commit()
    finally:
        conn.close()

    def unexpected_scrape_and_store(*_args: object, **_kwargs: object) -> object:
        raise AssertionError("unexpected Yahoo refresh")

    monkeypatch.setattr(refresh_module, "scrape_and_store", unexpected_scrape_and_store)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is None


def test_run_price_refresh_command_uses_refresh_prices_cli(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}
    db_path = tmp_path / "stocks.db"

    def fake_run(
        args: list[str],
        *,
        cwd: str,
        capture_output: bool,
        text: bool,
        timeout: int,
    ) -> subprocess.CompletedProcess[str]:
        captured.update(
            {
                "args": args,
                "cwd": cwd,
                "capture_output": capture_output,
                "text": text,
                "timeout": timeout,
            }
        )
        return subprocess.CompletedProcess(
            args=args,
            returncode=0,
            stdout="",
            stderr="Stock prices are fresh or were refreshed recently; no update needed",
        )

    monkeypatch.setattr(refresh_module.subprocess, "run", fake_run)

    result = refresh_module.run_price_refresh_command(
        cwd=tmp_path,
        db_path=db_path,
        if_needed=True,
    )

    assert captured == {
        "args": ["uv", "run", "refresh-prices", "--if-needed", "--db", str(db_path)],
        "cwd": str(tmp_path),
        "capture_output": True,
        "text": True,
        "timeout": 7200,
    }
    assert result.stderr == "Stock prices are fresh or were refreshed recently; no update needed"


def test_api_auto_update_runs_refresh_command_for_stale_external_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    conn = get_connection(db_path)
    captured: dict[str, object] = {}

    def fake_run_price_refresh_command(**kwargs: object) -> refresh_module.PriceRefreshCommandResult:
        captured.update(kwargs)
        return refresh_module.PriceRefreshCommandResult(stdout="", stderr="updated")

    monkeypatch.setattr(
        refresh_module,
        "run_price_refresh_command",
        fake_run_price_refresh_command,
    )

    try:
        result = refresh_module.ensure_prices_fresh_for_api(
            conn,
            today=date(2026, 5, 11),
            cwd=tmp_path,
        )
    finally:
        conn.close()

    assert result == refresh_module.PriceRefreshCommandResult(stdout="", stderr="updated")
    assert captured == {
        "db_path": db_path,
        "if_needed": True,
        "headless": True,
        "stream": True,
    }


def test_api_auto_update_skips_fresh_external_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-08"})
    conn = get_connection(db_path)

    def unexpected_run_price_refresh_command(**kwargs: object) -> object:
        del kwargs
        raise AssertionError("unexpected stock price refresh")

    monkeypatch.setattr(
        refresh_module,
        "run_price_refresh_command",
        unexpected_run_price_refresh_command,
    )

    try:
        assert refresh_module.ensure_prices_fresh_for_api(
            conn,
            today=date(2026, 5, 11),
            cwd=tmp_path,
        ) is None
    finally:
        conn.close()


def test_api_auto_update_skips_recent_refresh_attempt_for_stale_external_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    conn = get_connection(db_path)
    try:
        record_price_refresh_check(conn)
        conn.commit()
    finally:
        conn.close()

    conn = get_connection(db_path)

    def unexpected_run_price_refresh_command(**kwargs: object) -> object:
        del kwargs
        raise AssertionError("unexpected stock price refresh")

    monkeypatch.setattr(
        refresh_module,
        "run_price_refresh_command",
        unexpected_run_price_refresh_command,
    )

    try:
        assert refresh_module.ensure_prices_fresh_for_api(
            conn,
            today=date(2026, 5, 11),
            cwd=tmp_path,
        ) is None
    finally:
        conn.close()


def test_refresh_prices_uses_explicit_target_date(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )
    monkeypatch.setattr(
        refresh_module,
        "BrowserServiceClient",
        FakeBrowserServiceClient,
    )
    monkeypatch.setattr(
        refresh_module,
        "scrape_and_store",
        lambda *_args, **_kwargs: (0, 0),
    )

    result = refresh_module.refresh_prices(
        db_path=db_path,
        target_date=date(2026, 5, 7),
    )

    assert result is not None
    assert result.target_date == date(2026, 5, 7)
