from __future__ import annotations

from pathlib import Path

from stock_db.cli.inspect_stock_db import main
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.prices import upsert_price
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_stock


def _build_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    init_db(conn)
    upsert_stock(conn, "7203", "Toyota", "Auto", "Prime", edinet_code="E12345")
    upsert_price(conn, "7203", "2026-04-21", 3000.0, 100)
    upsert_sec_report(
        conn,
        ticker="7203",
        fiscal_year="latest",
        doc_id="S100TEST1",
        file_path="var/raw/edinet/markdown/7203/latest.md",
        xbrl_path="var/raw/edinet/markdown/xbrl/7203/S100TEST1.xhtml",
        page_count=100,
        char_count=2000,
    )
    upsert_financial_item(conn, "7203", "2025-03", "pl", "revenue", 1000.0, "yfinance")
    conn.commit()
    conn.close()


class TestInspectStockDb:
    def test_prints_all_sections(self, tmp_path: Path, capsys: object) -> None:
        db_path = tmp_path / "stocks.db"
        _build_db(db_path)

        rc = main(["7203", "--db", str(db_path), "--limit", "3"])
        captured = capsys.readouterr()

        assert rc == 0
        assert "[stocks]" in captured.out
        assert "[prices]" in captured.out
        assert "[sec_reports]" in captured.out
        assert "[financial_items]" in captured.out
        assert '"ticker": "7203"' in captured.out
        assert '"item_name": "revenue"' in captured.out
        assert captured.err == ""

    def test_returns_1_for_missing_ticker(self, tmp_path: Path, capsys: object) -> None:
        db_path = tmp_path / "stocks.db"
        _build_db(db_path)

        rc = main(["9999", "--db", str(db_path)])
        captured = capsys.readouterr()

        assert rc == 1
        assert "No rows found for ticker 9999" in captured.err

    def test_returns_2_for_missing_db(self, tmp_path: Path, capsys: object) -> None:
        db_path = tmp_path / "missing.db"

        rc = main(["7203", "--db", str(db_path)])
        captured = capsys.readouterr()

        assert rc == 2
        assert f"DB not found: {db_path}" in captured.err
