from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db.cli.sync_shikiho_forecasts import main
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.schema import init_db


def _build_shikiho_db(shikiho_db_path: Path) -> None:
    shikiho_db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(shikiho_db_path)
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS stock_forecasts (
            stock_code TEXT NOT NULL,
            company_name TEXT NOT NULL,
            forecast_type TEXT NOT NULL,
            period TEXT NOT NULL,
            operating_profit INTEGER,
            net_income INTEGER,
            fetched_at TEXT NOT NULL,
            PRIMARY KEY (stock_code, forecast_type, period)
        )
        """
    )
    conn.executemany(
        """
        INSERT INTO stock_forecasts
            (stock_code, company_name, forecast_type, period, operating_profit, net_income, fetched_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        [
            ("1301", "Test Co", "shikiho", "26.3", 7500, 5400, "2025-01-01T00:00:00"),
            ("1301", "Test Co", "shikiho", "27.3", 8200, 6000, "2025-01-01T00:00:00"),
            ("1302", "Other Co", "shikiho", "26.3", None, None, "2025-01-01T00:00:00"),
            ("1303", "Single Co", "shikiho", "26.3", 3000, 2000, "2025-01-01T00:00:00"),
            ("1301", "Test Co", "company", "26.3", 7000, 5000, "2025-01-01T00:00:00"),
        ],
    )
    conn.commit()
    conn.close()


def test_sync_stores_current_and_next_forecasts(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    rc = main(["--db", str(db_path), "--shikiho-db", str(shikiho_db_path)])
    assert rc == 0

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            """
            SELECT item_name, value, source FROM financial_items
            WHERE ticker = '1301' AND statement = 'forecast'
            ORDER BY item_name
            """
        ).fetchall()
        assert len(rows) == 2
        assert rows[0]["item_name"] == "net_income_current"
        assert rows[0]["value"] == 5_400_000_000
        assert rows[0]["source"] == "shikiho"
        assert rows[1]["item_name"] == "net_income_next"
        assert rows[1]["value"] == 6_000_000_000
        assert rows[1]["source"] == "shikiho"
    finally:
        conn.close()


def test_sync_skips_null_net_income(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    main(["--db", str(db_path), "--shikiho-db", str(shikiho_db_path)])

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            "SELECT * FROM financial_items WHERE ticker = '1302'"
        ).fetchall()
        assert len(rows) == 0
    finally:
        conn.close()


def test_sync_handles_single_period_forecast(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    main(["--db", str(db_path), "--shikiho-db", str(shikiho_db_path)])

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            """
            SELECT item_name, value FROM financial_items
            WHERE ticker = '1303' AND statement = 'forecast'
            """
        ).fetchall()
        assert len(rows) == 1
        assert rows[0]["item_name"] == "net_income_current"
        assert rows[0]["value"] == 2_000_000_000
    finally:
        conn.close()


def test_sync_ignores_company_forecasts(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    main(["--db", str(db_path), "--shikiho-db", str(shikiho_db_path)])

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            "SELECT * FROM financial_items WHERE source = 'shikiho' AND item_name LIKE 'net_income%' AND period = '26.3' AND ticker = '1301'"
        ).fetchall()
        item_names = {r["item_name"] for r in rows}
        assert "net_income_current" in item_names
        assert "net_income_next" not in item_names or any(
            r["item_name"] == "net_income_next" for r in conn.execute(
                "SELECT item_name FROM financial_items WHERE ticker = '1301' AND source = 'shikiho'"
            ).fetchall()
        )
    finally:
        conn.close()


def test_sync_replaces_existing_shikiho_data(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    upsert_financial_item(conn, "1301", "25.3", "forecast", "net_income_current", 3_000_000_000.0, "shikiho")
    conn.commit()
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    main(["--db", str(db_path), "--shikiho-db", str(shikiho_db_path)])

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            "SELECT period, item_name, value FROM financial_items WHERE ticker = '1301' AND source = 'shikiho' ORDER BY item_name"
        ).fetchall()
        assert len(rows) == 2
        assert rows[0]["period"] == "26.3"
        assert rows[0]["value"] == 5_400_000_000
    finally:
        conn.close()
