from __future__ import annotations

from pathlib import Path

from stock_db.cli.purge_irbank_financials import main
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.schema import init_db


def _build_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    init_db(conn)
    upsert_financial_item(conn, "1301", "2025-03", "bs", "current_assets", 100.0, "irbank")
    upsert_financial_item(conn, "1301", "2025-03", "bs", "inventories", 50.0, "irbank_bs")
    upsert_financial_item(conn, "1301", "2026-03", "forecast", "revenue", 300.0, "irbank_forecast")
    upsert_financial_item(conn, "1301", "2025-03", "pl", "revenue", 200.0, "edinet_xbrl")
    upsert_financial_item(conn, "1301", "2025-03", "bs", "cash_and_deposits", 80.0, "manual")
    conn.commit()
    conn.close()


def test_main_purges_only_irbank_rows(tmp_path: Path, capsys: object) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)

    rc = main(["--db", str(db_path)])
    captured = capsys.readouterr()

    assert rc == 0
    assert "irbank rows before purge: 3" in captured.err
    assert "irbank rows deleted: 3" in captured.err
    assert "irbank rows after purge: 0" in captured.err

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            """
            SELECT statement, item_name, source
            FROM financial_items
            ORDER BY statement, item_name, source
            """
        ).fetchall()
    finally:
        conn.close()

    assert [tuple(row) for row in rows] == [
        ("bs", "cash_and_deposits", "manual"),
        ("pl", "revenue", "edinet_xbrl"),
    ]


def test_main_returns_2_for_missing_db(tmp_path: Path, capsys: object) -> None:
    db_path = tmp_path / "missing.db"

    rc = main(["--db", str(db_path)])
    captured = capsys.readouterr()

    assert rc == 2
    assert f"DB not found: {db_path}" in captured.err
