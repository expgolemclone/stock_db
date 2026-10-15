from __future__ import annotations

import subprocess
from datetime import date, datetime, timezone
from pathlib import Path

import pytest

from stock_db.sources import price_refresh as refresh_module
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import (
    get_price_refresh_checked_at,
    record_price_refresh_check,
    upsert_price,
)
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
                upsert_price(conn, ticker, latest_date, 100.0, 1000, source="test")
        conn.commit()
    finally:
        conn.close()


def test_refresh_prices_skips_when_all_tickers_are_fresh(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-08"})

    monkeypatch.setattr(
        refresh_module,
        "update_stooq_daily_prices",
        lambda **_kwargs: (_ for _ in ()).throw(AssertionError("unexpected Stooq update")),
    )

    assert refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    ) is None


def test_refresh_prices_uses_yahoo_for_stale_non_tse_tickers(
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
        **kwargs: object,
    ) -> tuple[int, int]:
        del client
        captured["tickers"] = tickers
        captured.update(kwargs)
        upsert_price(conn, "1234", "2026-05-08", 110.0, 1000, source="test")
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
    assert captured["tickers"] == ["1234"]
    assert captured["min_date"] == "2026-05-08"
    assert captured["allowed_suffixes"] == ("N", "S", "F")
    conn = get_connection(db_path)
    try:
        assert get_price_refresh_checked_at(conn) is not None
    finally:
        conn.close()


def test_refresh_prices_reports_unresolved_tickers(
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
    assert result.unresolved_tickers == ("1234",)
    conn = get_connection(db_path)
    try:
        assert get_price_refresh_checked_at(conn) is None
    finally:
        conn.close()


def test_refresh_passes_target_date_to_stooq_update(
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
            upsert_price(conn, "1234", "2026-05-08", 110.0, 1000, source="test")
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
    assert captured["reuse_existing"] is False
    assert captured["target_date"] == date(2026, 5, 8)


def test_recent_refresh_check_only_suppresses_same_target(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    conn = get_connection(db_path)
    try:
        record_price_refresh_check(
            conn,
            target_date=date(2026, 5, 7),
            checked_at=datetime(2026, 5, 8, 0, 0, tzinfo=timezone.utc),
        )
        conn.commit()
    finally:
        conn.close()

    monkeypatch.setattr(
        refresh_module,
        "is_stooq_price_update_required",
        lambda *_args, **_kwargs: False,
    )
    monkeypatch.setattr(refresh_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(
        refresh_module,
        "scrape_and_store",
        lambda *_args, **_kwargs: (0, 0),
    )

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        target_date=date(2026, 5, 8),
    )

    assert result is not None
    assert result.target_date == date(2026, 5, 8)


def test_recent_refresh_check_suppresses_same_target(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_price_db(db_path, {"1234": "2026-05-07"})
    conn = get_connection(db_path)
    try:
        record_price_refresh_check(
            conn,
            target_date=date(2026, 5, 8),
        )
        conn.commit()
    finally:
        conn.close()

    monkeypatch.setattr(
        refresh_module,
        "scrape_and_store",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("unexpected Yahoo refresh")),
    )

    assert refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        target_date=date(2026, 5, 8),
    ) is None


def test_refresh_only_uses_non_tse_yahoo_suffixes(
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
        upsert_price(conn, "2222", "2026-05-08", 110.0, 1000, source="test")
        return 1, 0

    monkeypatch.setattr(refresh_module, "scrape_and_store", fake_scrape_and_store)

    result = refresh_module.refresh_prices(
        db_path=db_path,
        if_needed=True,
        today=date(2026, 5, 11),
    )

    assert result is not None
    assert captured["tickers"] == ["2222"]
    assert result.unresolved_tickers == ("1111", "3333")


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
        return subprocess.CompletedProcess(args=args, returncode=0, stdout="", stderr="ok")

    monkeypatch.setattr(refresh_module.subprocess, "run", fake_run)
    result = refresh_module.run_price_refresh_command(
        cwd=tmp_path,
        db_path=db_path,
        if_needed=True,
    )

    assert captured["args"] == [
        "uv",
        "run",
        "refresh-prices",
        "--if-needed",
        "--db",
        str(db_path),
    ]
    assert result.stderr == "ok"
