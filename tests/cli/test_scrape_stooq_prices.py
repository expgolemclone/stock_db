from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.cli import scrape_stooq_prices as cli_module
from stock_db.sources.stooq import StooqDailyPriceUpdateResult


def test_main_uses_default_headless_setting_when_flag_is_absent(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    captured: dict[str, object] = {}

    def fake_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        captured.update(kwargs)
        return StooqDailyPriceUpdateResult(
            imported=1,
            date="20260429",
            label="0429_d",
            file_path=tmp_path / "raw" / "0429_d.csv",
        )

    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", fake_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path)])
    output = capsys.readouterr()

    assert rc == 0
    assert captured["db_path"] == db_path
    assert captured["headless"] is None
    assert "Imported 1 JP prices for 20260429" in output.err


def test_main_overrides_headless_setting_when_flag_is_present(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    captured: dict[str, object] = {}

    def fake_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        captured.update(kwargs)
        return StooqDailyPriceUpdateResult(
            imported=1,
            date="20260429",
            label="0429_d",
            file_path=tmp_path / "raw" / "0429_d.csv",
        )

    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", fake_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path), "--headless"])

    assert rc == 0
    assert captured["headless"] is True


def test_main_passes_output_dir(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    raw_dir = tmp_path / "raw"
    captured: dict[str, object] = {}

    def fake_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        captured.update(kwargs)
        return StooqDailyPriceUpdateResult(
            imported=1,
            date="20260429",
            label="0429_d",
            file_path=raw_dir / "0429_d.csv",
        )

    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", fake_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path), "--output-dir", str(raw_dir)])

    assert rc == 0
    assert captured["output_dir"] == raw_dir


def test_main_skips_update_when_if_needed_is_fresh(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"

    class FakeConnection:
        def close(self) -> None:
            pass

    def unexpected_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        del kwargs
        raise AssertionError("unexpected Stooq update")

    monkeypatch.setattr(cli_module, "get_connection", lambda _db_path: FakeConnection())
    monkeypatch.setattr(cli_module, "init_db", lambda _conn: None)
    monkeypatch.setattr(cli_module, "is_stooq_price_update_required", lambda _conn: False)
    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", unexpected_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path), "--if-needed"])
    output = capsys.readouterr()

    assert rc == 0
    assert output.err.strip() == "Stooq prices are fresh; no update needed"


def test_main_updates_when_if_needed_is_stale(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    captured: dict[str, object] = {}

    class FakeConnection:
        def close(self) -> None:
            pass

    def fake_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        captured.update(kwargs)
        return StooqDailyPriceUpdateResult(
            imported=1,
            date="20260429",
            label="0429_d",
            file_path=tmp_path / "raw" / "0429_d.csv",
        )

    monkeypatch.setattr(cli_module, "get_connection", lambda _db_path: FakeConnection())
    monkeypatch.setattr(cli_module, "init_db", lambda _conn: None)
    monkeypatch.setattr(cli_module, "is_stooq_price_update_required", lambda _conn: True)
    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", fake_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path), "--if-needed"])

    assert rc == 0
    assert captured["db_path"] == db_path


def test_main_returns_1_on_update_failure(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"

    def fake_update_stooq_daily_prices(**kwargs: object) -> StooqDailyPriceUpdateResult:
        del kwargs
        raise cli_module.StooqDailyPriceUpdateError("Unauthorized")

    monkeypatch.setattr(cli_module, "update_stooq_daily_prices", fake_update_stooq_daily_prices)

    rc = cli_module.main(["--db", str(db_path)])
    output = capsys.readouterr()

    assert rc == 1
    assert output.err.strip() == "Unauthorized"
