from __future__ import annotations

from pathlib import Path

from stock_db.cli import parse_xbrl_financials as cli
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_stock


def _build_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    init_db(conn)
    upsert_stock(conn, "1301", "Alpha", "Sector", "Prime")
    upsert_stock(conn, "1302", "Beta", "Sector", "Prime")
    upsert_sec_report(
        conn,
        ticker="1301",
        fiscal_year="2025-03",
        doc_id="S100TEST1",
        xbrl_path="/tmp/1301.xhtml",
    )
    upsert_sec_report(
        conn,
        ticker="1302",
        fiscal_year="2025-03",
        doc_id="S100TEST2",
        xbrl_path="/tmp/1302.xhtml",
    )
    upsert_financial_item(conn, "1301", "2025-03", "pl", "revenue", 100.0, "edinet_xbrl")
    conn.commit()
    conn.close()


def test_main_skips_existing_by_default(tmp_path: Path, monkeypatch: object, capsys: object) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)

    parse_calls: list[str] = []
    replace_calls: list[dict[str, object]] = []

    def fake_parse(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
        parse_calls.append(xbrl_path)
        return {"2025-03": {"pl": {"revenue": 200.0}}}

    def fake_replace(
        conn: object,
        *,
        ticker: str,
        sources: tuple[str, ...],
        rows: list[dict[str, object]],
    ) -> None:
        del conn
        replace_calls.append({"ticker": ticker, "sources": sources, "rows": rows})

    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "parse_xbrl_financials", fake_parse)
    monkeypatch.setattr(cli, "replace_financial_items_for_ticker_sources", fake_replace)

    rc = cli.main([])
    captured = capsys.readouterr()

    assert rc == 0
    assert parse_calls == ["/tmp/1302.xhtml"]
    assert [call["ticker"] for call in replace_calls] == ["1302"]
    assert "Done: 1 ok, 0 errors" in captured.err


def test_main_ticker_still_honors_default_skip_existing(
    tmp_path: Path, monkeypatch: object, capsys: object,
) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)

    parse_calls: list[str] = []
    replace_calls: list[dict[str, object]] = []

    def fake_parse(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
        parse_calls.append(xbrl_path)
        return {"2025-03": {"pl": {"revenue": 200.0}}}

    def fake_replace(
        conn: object,
        *,
        ticker: str,
        sources: tuple[str, ...],
        rows: list[dict[str, object]],
    ) -> None:
        del conn, sources, rows
        replace_calls.append({"ticker": ticker})

    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "parse_xbrl_financials", fake_parse)
    monkeypatch.setattr(cli, "replace_financial_items_for_ticker_sources", fake_replace)

    rc = cli.main(["--ticker", "1301"])
    captured = capsys.readouterr()

    assert rc == 0
    assert parse_calls == []
    assert replace_calls == []
    assert "Done: 0 ok, 0 errors" in captured.err


def test_main_force_reparses_existing_ticker(tmp_path: Path, monkeypatch: object, capsys: object) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path)

    parse_calls: list[str] = []
    replace_calls: list[dict[str, object]] = []

    def fake_parse(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
        parse_calls.append(xbrl_path)
        return {"2025-03": {"bs": {"cash_and_deposits": 50.0}, "pl": {"revenue": 200.0}}}

    def fake_replace(
        conn: object,
        *,
        ticker: str,
        sources: tuple[str, ...],
        rows: list[dict[str, object]],
    ) -> None:
        del conn
        replace_calls.append({"ticker": ticker, "sources": sources, "rows": rows})

    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)
    monkeypatch.setattr(cli, "parse_xbrl_financials", fake_parse)
    monkeypatch.setattr(cli, "replace_financial_items_for_ticker_sources", fake_replace)

    rc = cli.main(["--ticker", "1301", "--force"])
    captured = capsys.readouterr()

    assert rc == 0
    assert parse_calls == ["/tmp/1301.xhtml"]
    assert len(replace_calls) == 1
    assert replace_calls[0]["ticker"] == "1301"
    assert replace_calls[0]["sources"] == cli._REPLACED_SOURCES
    assert replace_calls[0]["rows"] == [
        {
            "ticker": "1301",
            "period": "2025-03",
            "statement": "bs",
            "item_name": "cash_and_deposits",
            "value": 50.0,
            "source": "edinet_xbrl",
        },
        {
            "ticker": "1301",
            "period": "2025-03",
            "statement": "pl",
            "item_name": "revenue",
            "value": 200.0,
            "source": "edinet_xbrl",
        },
    ]
    assert "Done: 1 ok, 0 errors" in captured.err
