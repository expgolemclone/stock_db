from __future__ import annotations

import subprocess
from datetime import date
from pathlib import Path

import pytest

from stock_db.paths import PROJECT_ROOT
from stock_db.sources.stooq import updater as updater_module
from stock_db.sources.stooq.downloader import DownloadedStooqDailyFile
from stock_db.sources.stooq.exceptions import StooqDownloadError
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import get_stooq_price_update_checked_at, upsert_price
from stock_db.storage.schema import init_db


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


def test_update_stooq_daily_prices_imports_and_commits(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    raw_dir = tmp_path / "raw"
    captured_config: dict[str, object] = {}

    class CapturingBrowserServiceClient(FakeBrowserServiceClient):
        def __init__(self, *, config: dict[str, object]) -> None:
            super().__init__(config=config)
            captured_config.update(config)

    def fake_download_latest_daily_file(
        client: object,
        output_dir: Path,
        *,
        timeout: int | None = None,
        reuse_existing: bool = True,
    ) -> DownloadedStooqDailyFile:
        del client, timeout
        assert reuse_existing is True
        output_dir.mkdir(parents=True, exist_ok=True)
        file_path = output_dir / "0429_d.csv"
        file_path.write_text("placeholder", encoding="utf-8")
        return DownloadedStooqDailyFile(date="20260429", label="0429_d", file_path=file_path)

    def fake_ingest_daily_prices(conn: object, file_path: Path) -> int:
        assert file_path == raw_dir / "0429_d.csv"
        upsert_price(conn, "7203", "2026-04-29", 3067.0, None)
        return 1

    monkeypatch.setattr(updater_module, "BrowserServiceClient", CapturingBrowserServiceClient)
    monkeypatch.setattr(updater_module, "download_latest_daily_file", fake_download_latest_daily_file)
    monkeypatch.setattr(updater_module, "ingest_daily_prices", fake_ingest_daily_prices)

    result = updater_module.update_stooq_daily_prices(
        db_path=db_path,
        output_dir=raw_dir,
        headless=True,
    )

    assert result.imported == 1
    assert result.date == "20260429"
    assert result.file_path == raw_dir / "0429_d.csv"
    assert captured_config["headless"] is True

    conn = get_connection(db_path)
    try:
        row = conn.execute(
            "SELECT ticker, date, close, volume FROM prices WHERE ticker = ?",
            ("7203",),
        ).fetchone()
        checked_at = get_stooq_price_update_checked_at(conn)
    finally:
        conn.close()

    assert dict(row) == {
        "ticker": "7203",
        "date": "2026-04-29",
        "close": 3067.0,
        "volume": None,
    }
    assert checked_at is not None


def test_update_stooq_daily_prices_wraps_expected_failures(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_download_latest_daily_file(
        client: object,
        output_dir: Path,
        *,
        timeout: int | None = None,
        reuse_existing: bool = True,
    ) -> DownloadedStooqDailyFile:
        del client, output_dir, timeout, reuse_existing
        raise StooqDownloadError("Unauthorized")

    monkeypatch.setattr(updater_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(updater_module, "download_latest_daily_file", fake_download_latest_daily_file)

    with pytest.raises(updater_module.StooqDailyPriceUpdateError, match="Unauthorized"):
        updater_module.update_stooq_daily_prices(db_path=tmp_path / "stocks.db")


def test_run_stooq_price_update_command_uses_stock_db_cwd(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

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
            stderr="Imported 1 JP prices for 20260429",
        )

    monkeypatch.setattr(updater_module.subprocess, "run", fake_run)

    result = updater_module.run_stooq_price_update_command()

    assert captured == {
        "args": ["uv", "run", "scrape-stooq-prices"],
        "cwd": str(PROJECT_ROOT),
        "capture_output": True,
        "text": True,
        "timeout": 300,
    }
    assert result.stderr == "Imported 1 JP prices for 20260429"


def test_run_stooq_price_update_command_passes_paths(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}
    db_path = tmp_path / "custom-stocks.db"
    output_dir = tmp_path / "raw" / "stooq"

    def fake_run(
        args: list[str],
        *,
        cwd: str,
        capture_output: bool,
        text: bool,
        timeout: int,
    ) -> subprocess.CompletedProcess[str]:
        del cwd, capture_output, text, timeout
        captured["args"] = args
        return subprocess.CompletedProcess(args=args, returncode=0, stdout="", stderr="")

    monkeypatch.setattr(updater_module.subprocess, "run", fake_run)

    updater_module.run_stooq_price_update_command(
        db_path=db_path,
        output_dir=output_dir,
    )

    assert captured["args"] == [
        "uv",
        "run",
        "scrape-stooq-prices",
        "--db",
        str(db_path),
        "--output-dir",
        str(output_dir),
    ]


def test_run_stooq_price_update_command_passes_if_needed(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}
    db_path = tmp_path / "custom-stocks.db"

    def fake_run(
        args: list[str],
        *,
        cwd: str,
        capture_output: bool,
        text: bool,
        timeout: int,
    ) -> subprocess.CompletedProcess[str]:
        del cwd, capture_output, text, timeout
        captured["args"] = args
        return subprocess.CompletedProcess(args=args, returncode=0, stdout="", stderr="")

    monkeypatch.setattr(updater_module.subprocess, "run", fake_run)

    updater_module.run_stooq_price_update_command(
        db_path=db_path,
        if_needed=True,
    )

    assert captured["args"] == [
        "uv",
        "run",
        "scrape-stooq-prices",
        "--if-needed",
        "--db",
        str(db_path),
    ]


def test_api_auto_update_is_noop_inside_stock_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)

    def unexpected_run_stooq_price_update_command(**kwargs: object) -> object:
        del kwargs
        raise AssertionError("unexpected Stooq update")

    monkeypatch.setattr(
        updater_module,
        "run_stooq_price_update_command",
        unexpected_run_stooq_price_update_command,
    )

    try:
        assert updater_module.ensure_stooq_prices_fresh_for_api(
            conn,
            cwd=PROJECT_ROOT,
        ) is None
    finally:
        conn.close()


def test_api_auto_update_skips_fresh_external_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    upsert_price(conn, "7203", "2026-05-18", 3067.0, None)
    conn.commit()

    def unexpected_run_stooq_price_update_command(**kwargs: object) -> object:
        del kwargs
        raise AssertionError("unexpected Stooq update")

    monkeypatch.setattr(
        updater_module,
        "run_stooq_price_update_command",
        unexpected_run_stooq_price_update_command,
    )

    try:
        assert updater_module.ensure_stooq_prices_fresh_for_api(
            conn,
            today=date(2026, 5, 18),
            cwd=tmp_path,
        ) is None
    finally:
        conn.close()


def test_api_auto_update_runs_for_stale_external_db(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    upsert_price(conn, "7203", "2026-05-08", 3067.0, None)
    conn.commit()
    captured: dict[str, object] = {}

    def fake_run_stooq_price_update_command(**kwargs: object) -> updater_module.StooqPriceUpdateCommandResult:
        captured.update(kwargs)
        return updater_module.StooqPriceUpdateCommandResult(stdout="", stderr="updated")

    monkeypatch.setattr(
        updater_module,
        "run_stooq_price_update_command",
        fake_run_stooq_price_update_command,
    )

    try:
        result = updater_module.ensure_stooq_prices_fresh_for_api(
            conn,
            today=date(2026, 5, 11),
            cwd=tmp_path,
        )
    finally:
        conn.close()

    assert result == updater_module.StooqPriceUpdateCommandResult(stdout="", stderr="updated")
    assert captured == {
        "db_path": db_path,
        "if_needed": True,
    }


def test_run_stooq_price_update_command_raises_on_nonzero_exit(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: object, **kwargs: object) -> subprocess.CompletedProcess[str]:
        del args, kwargs
        return subprocess.CompletedProcess(
            args=["uv", "run", "scrape-stooq-prices"],
            returncode=1,
            stdout="",
            stderr="Captcha error",
        )

    monkeypatch.setattr(updater_module.subprocess, "run", fake_run)

    with pytest.raises(updater_module.StooqDailyPriceUpdateError, match="exit=1.*Captcha error"):
        updater_module.run_stooq_price_update_command(cwd=tmp_path)


def test_run_stooq_price_update_command_raises_on_timeout(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: object, **kwargs: object) -> subprocess.CompletedProcess[str]:
        del args, kwargs
        raise subprocess.TimeoutExpired(cmd="uv", timeout=300)

    monkeypatch.setattr(updater_module.subprocess, "run", fake_run)

    with pytest.raises(updater_module.StooqDailyPriceUpdateError, match="TimeoutExpired"):
        updater_module.run_stooq_price_update_command(cwd=tmp_path)
