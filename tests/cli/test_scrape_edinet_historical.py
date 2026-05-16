from __future__ import annotations

import json
import sqlite3
from datetime import date
from pathlib import Path

from stock_db.cli import scrape_edinet_historical as cli
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import upsert_stock


def _init_stock_db(db_path: Path, ticker: str = "7203") -> None:
    conn = get_connection(db_path)
    try:
        init_db(conn)
        upsert_stock(conn, ticker, "Toyota", "Auto", "Prime")
        conn.commit()
    finally:
        conn.close()


def test_date_years_ago_handles_leap_day() -> None:
    assert cli._date_years_ago(date(2024, 2, 29), 1) == date(2023, 2, 28)


def test_main_uses_to_date_minus_years_when_from_date_is_omitted(
    tmp_path: Path, monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    _init_stock_db(db_path)
    captured: dict[str, str] = {}

    def fake_discover_historical_reports(**kwargs):
        captured["from_date"] = kwargs["from_date"]
        captured["to_date"] = kwargs["to_date"]
        return {}

    monkeypatch.setenv("EDINET_API_KEY", "dummy")
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "discover_historical_reports", fake_discover_historical_reports)

    assert cli.main(["--to-date", "2026-05-16", "--years", "10"]) == 0

    assert captured == {"from_date": "2016-05-16", "to_date": "2026-05-16"}


def test_main_syncs_existing_raw_xbrl_without_redownloading(
    tmp_path: Path, monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    raw_dir = tmp_path / "raw" / "edinet"
    artifact = raw_dir / "xbrl" / "7203" / "S100HIST"
    (artifact / "XBRL").mkdir(parents=True)
    (artifact / "XBRL" / "report.xhtml").write_text("<html></html>", encoding="utf-8")
    (artifact.parent / "S100HIST.zip").write_bytes(b"zip")
    _init_stock_db(db_path)

    def fake_discover_historical_reports(**kwargs):
        return {
            "7203": [
                {
                    "doc_id": "S100HIST",
                    "fiscal_year": "FY2024",
                    "period_end": "2024-03-31",
                    "submit_date": "2024-06-25T15:00:00",
                    "filer_name": "Toyota",
                }
            ]
        }

    def fail_download(*args, **kwargs):
        raise AssertionError("existing raw XBRL should not be downloaded again")

    monkeypatch.setenv("EDINET_API_KEY", "dummy")
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "_EDINET_RAW_DIR", raw_dir)
    monkeypatch.setattr(cli, "discover_historical_reports", fake_discover_historical_reports)
    monkeypatch.setattr(cli, "download_xbrl_package", fail_download)

    assert cli.main(["--from-date", "2024-01-01", "--to-date", "2024-01-01"]) == 0

    conn = get_connection(db_path)
    try:
        conn.row_factory = sqlite3.Row
        row = conn.execute("SELECT fiscal_year, xbrl_path FROM sec_reports").fetchone()
    finally:
        conn.close()

    assert row["fiscal_year"] == "FY2024"
    assert row["xbrl_path"] == str(artifact.resolve())


def test_main_writes_discovery_checkpoint_json(
    tmp_path: Path, monkeypatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    raw_dir = tmp_path / "raw" / "edinet"
    checkpoint = raw_dir / "discovery" / "checkpoint.json"
    artifact = raw_dir / "xbrl" / "7203" / "S100JSON"
    doc_info = {
        "doc_id": "S100JSON",
        "fiscal_year": "FY2024",
        "period_end": "2024-03-31",
        "submit_date": "2024-06-25T15:00:00",
        "filer_name": "Toyota",
    }
    (artifact / "XBRL").mkdir(parents=True)
    (artifact / "XBRL" / "report.xhtml").write_text("<html></html>", encoding="utf-8")
    (artifact.parent / "S100JSON.zip").write_bytes(b"zip")
    _init_stock_db(db_path)

    def fake_discover_historical_reports(**kwargs):
        kwargs["on_day_scanned"]("2024-06-25", [("7203", doc_info)], 1)
        return {"7203": [doc_info]}

    monkeypatch.setenv("EDINET_API_KEY", "dummy")
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "_EDINET_RAW_DIR", raw_dir)
    monkeypatch.setattr(cli, "discover_historical_reports", fake_discover_historical_reports)

    assert cli.main([
        "--from-date",
        "2024-01-01",
        "--to-date",
        "2024-12-31",
        "--discovery-json",
        str(checkpoint),
    ]) == 0

    payload = json.loads(checkpoint.read_text(encoding="utf-8"))
    assert payload["from_date"] == "2024-01-01"
    assert payload["to_date"] == "2024-12-31"
    assert payload["completed_dates"] == ["2024-06-25"]
    assert payload["reports"]["7203"] == [doc_info]


def test_load_discovery_checkpoint_reuses_matching_range(tmp_path: Path) -> None:
    checkpoint = tmp_path / "checkpoint.json"
    checkpoint.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "from_date": "2024-01-01",
                "to_date": "2024-12-31",
                "completed_dates": ["2024-06-25"],
                "reports": {
                    "7203": [{"doc_id": "S100JSON"}],
                    "ABCD": [{"doc_id": "S100SKIP"}],
                },
            }
        ),
        encoding="utf-8",
    )

    reports, completed_dates = cli._load_discovery_checkpoint(
        checkpoint,
        from_date="2024-01-01",
        to_date="2024-12-31",
        target_tickers={"7203"},
    )

    assert reports == {"7203": [{"doc_id": "S100JSON"}]}
    assert completed_dates == {"2024-06-25"}
