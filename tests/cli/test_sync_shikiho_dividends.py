from __future__ import annotations

import sqlite3
from pathlib import Path

from stock_db.cli.sync_shikiho_dividends import main
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db


def _build_shikiho_db(shikiho_db_path: Path) -> None:
    shikiho_db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(shikiho_db_path)
    conn.execute(
        """
        CREATE TABLE stock_dividends (
            stock_code TEXT NOT NULL,
            period TEXT NOT NULL,
            dividend REAL,
            is_forecast INTEGER NOT NULL,
            fetched_at TEXT NOT NULL,
            PRIMARY KEY (stock_code, period)
        )
        """
    )
    conn.executemany(
        """
        INSERT INTO stock_dividends
            (stock_code, period, dividend, is_forecast, fetched_at)
        VALUES (?, ?, ?, ?, ?)
        """,
        [
            ("8046", "25.3", 130.0, 0, "2026-05-01T00:00:00"),
            ("8046", "26.3", 130.0, 1, "2026-05-01T00:00:00"),
            ("8046", "27.3", 130.0, 1, "2026-05-01T00:00:00"),
            ("9999", "27.3", 50.0, 1, "2026-05-01T00:00:00"),
        ],
    )
    conn.commit()
    conn.close()


def test_sync_applies_dividend_overrides(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)
    overrides_path = tmp_path / "overrides.toml"
    overrides_path.write_text(
        """
        [dividend_overrides."8046"]
        "26.3" = 40.0
        "27.3" = 38.0
        """,
        encoding="utf-8",
    )

    rc = main(
        [
            "--db",
            str(db_path),
            "--shikiho-db",
            str(shikiho_db_path),
            "--dividend-overrides",
            str(overrides_path),
        ]
    )
    assert rc == 0

    conn = get_connection(db_path)
    try:
        rows = conn.execute(
            """
            SELECT ticker, period, value FROM financial_items
            WHERE source = 'shikiho' AND statement = 'dividend'
            ORDER BY ticker, period
            """
        ).fetchall()
        values = {(row["ticker"], row["period"]): row["value"] for row in rows}
        assert values[("8046", "25.3")] == 130.0
        assert values[("8046", "26.3")] == 40.0
        assert values[("8046", "27.3")] == 38.0
        assert values[("9999", "27.3")] == 50.0
    finally:
        conn.close()


def test_sync_uses_upstream_values_without_override_file(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()

    shikiho_db_path = tmp_path / "stock_performance.db"
    _build_shikiho_db(shikiho_db_path)

    rc = main(
        [
            "--db",
            str(db_path),
            "--shikiho-db",
            str(shikiho_db_path),
            "--dividend-overrides",
            str(tmp_path / "missing.toml"),
        ]
    )
    assert rc == 0

    conn = get_connection(db_path)
    try:
        row = conn.execute(
            """
            SELECT value FROM financial_items
            WHERE ticker = '8046' AND period = '27.3'
              AND source = 'shikiho' AND statement = 'dividend'
            """
        ).fetchone()
        assert row["value"] == 130.0
    finally:
        conn.close()
