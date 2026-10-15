from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

import stock_db.cli.plot_xbrl_pl_trends as plot_xbrl_pl_trends
from stock_db.cli.plot_xbrl_pl_trends import build_pl_trend_payload, main, render_html
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.stocks import upsert_stock


def _source_id(conn: sqlite3.Connection) -> int:
    conn.execute(
        """
        INSERT INTO data_sources (source_code) VALUES ('edinet_xbrl')
        ON CONFLICT(source_code) DO NOTHING
        """
    )
    return int(
        conn.execute(
            "SELECT source_id FROM data_sources WHERE source_code = 'edinet_xbrl'"
        ).fetchone()["source_id"]
    )


def _insert_pl_fact(
    conn: sqlite3.Connection,
    *,
    source_id: int,
    period: str,
    doc_id: str,
    scope: str,
    concept: str,
    value: float,
    role_uri: str = "http://example.test/role/rol_ConsolidatedStatementOfIncome",
    dimensions_json: str = "[]",
    context_id: str = "CurrentYearDuration",
) -> None:
    conn.execute(
        """
        INSERT INTO xbrl_profit_loss_facts
            (
                ticker,
                doc_id,
                period,
                source_id,
                consolidation_scope,
                context_id,
                dimensions_json,
                concept_namespace,
                concept_name,
                role_uri,
                unit,
                unit_ref,
                value,
                updated_at
            )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            "4776",
            doc_id,
            period,
            source_id,
            scope,
            context_id,
            dimensions_json,
            "http://example.test/taxonomy",
            concept,
            role_uri,
            "JPY",
            "JPY",
            value,
            "2026-06-01T00:00:00+00:00",
        ),
    )


def _insert_edge(
    conn: sqlite3.Connection,
    *,
    source_id: int,
    doc_id: str,
    parent: str,
    child: str,
    child_label: str,
    order: float,
) -> None:
    conn.execute(
        """
        INSERT INTO xbrl_statement_presentation_edges
            (
                ticker,
                doc_id,
                statement,
                source_id,
                role_uri,
                parent_namespace,
                parent_name,
                parent_label,
                child_namespace,
                child_name,
                child_order,
                child_label,
                updated_at
            )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            "4776",
            doc_id,
            "pl",
            source_id,
            "http://example.test/role/rol_ConsolidatedStatementOfIncome",
            "http://example.test/taxonomy",
            parent,
            parent,
            "http://example.test/taxonomy",
            child,
            order,
            child_label,
            "2026-06-01T00:00:00+00:00",
        ),
    )


def _build_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    try:
        init_db(conn)
        upsert_stock(conn, "4776", "Cybozu", "Info", "Prime")
        source_id = _source_id(conn)
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2023-12",
            doc_id="S100OLD",
            scope="non_consolidated",
            concept="NetSales",
            value=999.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2023-12",
            doc_id="S100OLD",
            scope="consolidated",
            concept="NetSales",
            value=100.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2023-12",
            doc_id="S100OLD",
            scope="consolidated",
            concept="OldExpense",
            value=12.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2023-12",
            doc_id="S100ZZZ",
            scope="consolidated",
            concept="NetSales",
            value=101.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2023-12",
            doc_id="S100ZZZ",
            scope="consolidated",
            concept="OldExpense",
            value=13.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2024-12",
            doc_id="S100NEW",
            scope="consolidated",
            concept="NetSales",
            value=200.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2024-12",
            doc_id="S100NEW",
            scope="consolidated",
            concept="NetSales",
            value=200.0,
            role_uri="http://example.test/role/rol_StatementOfIncome",
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2024-12",
            doc_id="S100NEW",
            scope="consolidated",
            concept="SegmentProfit",
            value=30.0,
        )
        _insert_pl_fact(
            conn,
            source_id=source_id,
            period="2024-12",
            doc_id="S100NEW",
            scope="consolidated",
            concept="SegmentProfit",
            value=999.0,
            dimensions_json='[{"dimension":"axis","member":"segment"}]',
            context_id="CurrentYearDuration_Segment",
        )
        _insert_edge(
            conn,
            source_id=source_id,
            doc_id="S100NEW",
            parent="StatementOfIncomeLineItems",
            child="NetSales",
            child_label="売上高",
            order=1.0,
        )
        _insert_edge(
            conn,
            source_id=source_id,
            doc_id="S100NEW",
            parent="StatementOfIncomeLineItems",
            child="SegmentProfit",
            child_label="セグメント利益",
            order=2.0,
        )
        conn.commit()
    finally:
        conn.close()


def test_build_payload_selects_consolidated_reports_and_dedupes_values(tmp_path: Path) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)
    conn = sqlite3.connect(f"file:{db_path.resolve()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    try:
        payload = build_pl_trend_payload(conn, ticker="4776", n_periods=2)
    finally:
        conn.close()

    assert payload["periods"] == ["2023-12", "2024-12"]
    assert [row["consolidation_scope"] for row in payload["selected_reports"]] == [
        "consolidated",
        "consolidated",
    ]
    assert payload["selected_reports"][0]["doc_id"] == "S100ZZZ"
    items = {item["concept_name"]: item for item in payload["items"]}
    assert items["NetSales"]["label"] == "売上高"
    assert items["NetSales"]["values"] == [101.0, 200.0]
    assert items["SegmentProfit"]["values"] == [None, 30.0]
    assert items["OldExpense"]["values"] == [13.0, None]


def test_render_html_embeds_data() -> None:
    payload = {
        "ticker": "4776",
        "name": "Cybozu",
        "source": "edinet_xbrl",
        "requested_scope": "auto",
        "periods": ["2024-12"],
        "selected_reports": [
            {
                "period": "2024-12",
                "doc_id": "S100NEW",
                "consolidation_scope": "consolidated",
                "fact_count": 1,
                "valued_count": 1,
                "role_fact_count": 1,
            }
        ],
        "items": [
            {
                "concept_name": "NetSales",
                "label": "売上高",
                "values": [200.0],
                "latest_value": 200.0,
                "non_null_count": 1,
                "min_value": 200.0,
                "max_value": 200.0,
                "total_abs_value": 200.0,
            }
        ],
    }

    rendered = render_html(payload)

    assert "window.PL_TREND_DATA" in rendered
    assert "4776" in rendered
    assert "売上高" in rendered
    assert "detailChart" in rendered


def test_render_html_supports_manual_multi_series_overlay() -> None:
    payload = {
        "ticker": "4776",
        "name": "Cybozu",
        "source": "edinet_xbrl",
        "requested_scope": "auto",
        "periods": ["2023-12", "2024-12"],
        "selected_reports": [
            {
                "period": "2023-12",
                "doc_id": "S100OLD",
                "consolidation_scope": "consolidated",
                "fact_count": 2,
                "valued_count": 2,
                "role_fact_count": 2,
            },
            {
                "period": "2024-12",
                "doc_id": "S100NEW",
                "consolidation_scope": "consolidated",
                "fact_count": 2,
                "valued_count": 2,
                "role_fact_count": 2,
            },
        ],
        "items": [
            {
                "concept_name": "NetSales",
                "label": "売上高",
                "values": [100.0, 200.0],
                "latest_value": 200.0,
                "non_null_count": 2,
                "min_value": 100.0,
                "max_value": 200.0,
                "total_abs_value": 300.0,
            },
            {
                "concept_name": "OperatingIncome",
                "label": "営業利益",
                "values": [10.0, 30.0],
                "latest_value": 30.0,
                "non_null_count": 2,
                "min_value": 10.0,
                "max_value": 30.0,
                "total_abs_value": 40.0,
            },
        ],
    }

    rendered = render_html(payload)

    assert "selectedConcepts" in rendered
    assert "toggleItemSelection" in rendered
    assert "aria-pressed" in rendered
    assert "legend-chip" in rendered
    assert "latestValues" in rendered
    assert "scaledSeries" in rendered
    assert "tooltip-row" in rendered


def test_render_html_contains_responsive_layout_guards() -> None:
    payload = {
        "ticker": "4776",
        "name": "Cybozu",
        "source": "edinet_xbrl",
        "requested_scope": "auto",
        "periods": ["2023-12", "2024-12"],
        "selected_reports": [
            {
                "period": "2023-12",
                "doc_id": "S100OLD",
                "consolidation_scope": "consolidated",
                "fact_count": 1,
                "valued_count": 1,
                "role_fact_count": 1,
            },
            {
                "period": "2024-12",
                "doc_id": "S100NEW",
                "consolidation_scope": "consolidated",
                "fact_count": 1,
                "valued_count": 1,
                "role_fact_count": 1,
            },
        ],
        "items": [
            {
                "concept_name": "NetSales",
                "label": "売上高",
                "values": [100.0, 200.0],
                "latest_value": 200.0,
                "non_null_count": 2,
                "min_value": 100.0,
                "max_value": 200.0,
                "total_abs_value": 300.0,
            }
        ],
    }

    rendered = render_html(payload)

    assert "@media (max-width: 1200px)" in rendered
    assert "contain: inline-size layout paint;" in rendered
    assert "overflow-x: clip;" in rendered
    assert "ResizeObserver" in rendered


def test_main_writes_html(tmp_path: Path, capsys: object) -> None:
    db_path = tmp_path / "stocks.db"
    output_path = tmp_path / "pl.html"
    _build_db(db_path)

    rc = main(["4776", "--db", str(db_path), "--periods", "2", "--output", str(output_path)])
    captured = capsys.readouterr()

    assert rc == 0
    assert output_path.is_file()
    assert "2 periods, 3 PL items" in captured.out
    assert "売上高" in output_path.read_text(encoding="utf-8")


def test_main_can_open_html_with_playwright(
    tmp_path: Path,
    capsys: object,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    output_path = tmp_path / "pl.html"
    screenshot_path = tmp_path / "pl.png"
    browser_path = tmp_path / "chrome"
    browser_path.write_text("#!/bin/sh\n", encoding="utf-8")
    _build_db(db_path)
    calls: list[dict[str, object]] = []

    def fake_open_html_with_playwright(
        html_path: Path,
        *,
        screenshot_path: Path | None,
        headed: bool,
        hold_ms: int,
        browser_executable: Path | None,
        timeout_ms: int,
        viewport_width: int = 1440,
        viewport_height: int = 1000,
    ) -> plot_xbrl_pl_trends.PlaywrightOpenResult:
        calls.append(
            {
                "html_path": html_path,
                "screenshot_path": screenshot_path,
                "headed": headed,
                "hold_ms": hold_ms,
                "browser_executable": browser_executable,
                "timeout_ms": timeout_ms,
                "viewport_width": viewport_width,
                "viewport_height": viewport_height,
            }
        )
        return {
            "title": "4776 Cybozu PL",
            "item_count": 3,
            "canvas_width": 900,
            "canvas_height": 500,
            "drawn_pixels": 1234,
            "screenshot_path": str(screenshot_path) if screenshot_path is not None else None,
        }

    monkeypatch.setattr(
        plot_xbrl_pl_trends,
        "open_html_with_playwright",
        fake_open_html_with_playwright,
    )

    rc = main(
        [
            "4776",
            "--db",
            str(db_path),
            "--periods",
            "2",
            "--output",
            str(output_path),
            "--open-with-playwright",
            "--playwright-screenshot",
            str(screenshot_path),
            "--playwright-headed",
            "--playwright-hold-ms",
            "250",
            "--playwright-browser-executable",
            str(browser_path),
            "--playwright-timeout-ms",
            "12345",
        ]
    )
    captured = capsys.readouterr()

    assert rc == 0
    assert output_path.is_file()
    assert calls == [
        {
            "html_path": output_path,
            "screenshot_path": screenshot_path,
            "headed": True,
            "hold_ms": 250,
            "browser_executable": browser_path,
            "timeout_ms": 12345,
            "viewport_width": 1440,
            "viewport_height": 1000,
        }
    ]
    assert "Playwright opened" in captured.out
    assert "1234 drawn canvas pixels" in captured.out
    assert f"Playwright screenshot: {screenshot_path}" in captured.out


def test_main_returns_2_for_invalid_playwright_timeout(
    tmp_path: Path,
    capsys: object,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    output_path = tmp_path / "pl.html"
    _build_db(db_path)
    monkeypatch.setattr(
        plot_xbrl_pl_trends,
        "open_html_with_playwright",
        lambda *args, **kwargs: pytest.fail("should not open Playwright"),
    )

    rc = main(
        [
            "4776",
            "--db",
            str(db_path),
            "--periods",
            "2",
            "--output",
            str(output_path),
            "--open-with-playwright",
            "--playwright-timeout-ms",
            "0",
        ]
    )
    captured = capsys.readouterr()

    assert rc == 2
    assert "--playwright-timeout-ms must be >= 1" in captured.err


def test_main_returns_2_for_invalid_playwright_hold(
    tmp_path: Path,
    capsys: object,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "stocks.db"
    output_path = tmp_path / "pl.html"
    _build_db(db_path)
    monkeypatch.setattr(
        plot_xbrl_pl_trends,
        "open_html_with_playwright",
        lambda *args, **kwargs: pytest.fail("should not open Playwright"),
    )

    rc = main(
        [
            "4776",
            "--db",
            str(db_path),
            "--periods",
            "2",
            "--output",
            str(output_path),
            "--open-with-playwright",
            "--playwright-hold-ms",
            "-1",
        ]
    )
    captured = capsys.readouterr()

    assert rc == 2
    assert "--playwright-hold-ms must be >= 0" in captured.err


def test_main_returns_1_for_missing_ticker(tmp_path: Path, capsys: object) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)

    rc = main(["9999", "--db", str(db_path)])
    captured = capsys.readouterr()

    assert rc == 1
    assert "No PL XBRL facts found for ticker 9999" in captured.err


def test_main_returns_2_for_missing_db(tmp_path: Path, capsys: object) -> None:
    db_path = tmp_path / "missing.db"

    rc = main(["4776", "--db", str(db_path)])
    captured = capsys.readouterr()

    assert rc == 2
    assert f"DB not found: {db_path}" in captured.err
