from __future__ import annotations

from pathlib import Path

from stock_db.cli import scrape_stooq_prices as cli_module


class FakeBrowserServiceClient:
    def __init__(self, *, config: dict[str, object], browser_service_dir: str | Path | None = None) -> None:
        self.config = config
        self.browser_service_dir = browser_service_dir

    def __enter__(self) -> "FakeBrowserServiceClient":
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        return None


def test_main_imports_prices_into_db(
    tmp_path: Path,
    monkeypatch: object,
    capsys: object,
) -> None:
    db_path = tmp_path / "stocks.db"
    raw_dir = tmp_path / "raw"

    def fake_download_latest_daily_file(
        client: object,
        output_dir: Path,
        *,
        timeout: int | None = None,
    ) -> object:
        output_dir.mkdir(parents=True, exist_ok=True)
        file_path = output_dir / "0429_d.csv"
        file_path.write_text(
            "\n".join([
                "<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>",
                "7203.JP,D,20260429,000000,3065,3101,3057,3067,19390900,0",
            ]),
            encoding="utf-8",
        )
        return cli_module.DownloadedStooqDailyFile(
            date="20260429",
            label="0429_d",
            file_path=file_path,
        )

    monkeypatch.setattr(cli_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(cli_module, "download_latest_daily_file", fake_download_latest_daily_file)

    rc = cli_module.main(["--db", str(db_path), "--output-dir", str(raw_dir)])
    captured = capsys.readouterr()

    assert rc == 0
    assert "Imported 1 JP prices for 20260429" in captured.err

    conn = cli_module.get_connection(db_path)
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


def test_main_returns_1_on_download_failure(
    tmp_path: Path,
    monkeypatch: object,
    capsys: object,
) -> None:
    db_path = tmp_path / "stocks.db"

    def fake_download_latest_daily_file(
        client: object,
        output_dir: Path,
        *,
        timeout: int | None = None,
    ) -> object:
        raise cli_module.StooqDownloadError("Unauthorized")

    monkeypatch.setattr(cli_module, "BrowserServiceClient", FakeBrowserServiceClient)
    monkeypatch.setattr(cli_module, "download_latest_daily_file", fake_download_latest_daily_file)

    rc = cli_module.main(["--db", str(db_path)])
    captured = capsys.readouterr()

    assert rc == 1
    assert captured.err.strip() == "Unauthorized"
