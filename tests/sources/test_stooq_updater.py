from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.stooq import updater as updater_module
from stock_db.sources.stooq.downloader import DownloadedStooqDailyFile
from stock_db.sources.stooq.exceptions import StooqDownloadError
from stock_db.storage.connection import get_connection
from stock_db.storage.prices import upsert_price


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
    ) -> DownloadedStooqDailyFile:
        del client, timeout
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
    finally:
        conn.close()

    assert dict(row) == {
        "ticker": "7203",
        "date": "2026-04-29",
        "close": 3067.0,
        "volume": None,
    }


def test_update_stooq_daily_prices_wraps_expected_failures(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_download_latest_daily_file(
        client: object,
        output_dir: Path,
        *,
        timeout: int | None = None,
    ) -> DownloadedStooqDailyFile:
        del client, output_dir, timeout
        raise StooqDownloadError("Unauthorized")

    monkeypatch.setattr(updater_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(updater_module, "download_latest_daily_file", fake_download_latest_daily_file)

    with pytest.raises(updater_module.StooqDailyPriceUpdateError, match="Unauthorized"):
        updater_module.update_stooq_daily_prices(db_path=tmp_path / "stocks.db")
