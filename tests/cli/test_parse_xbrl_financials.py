from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.cli import parse_xbrl_financials as cli
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import upsert_financial_item
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_stock


def _write_financial_xbrl(path: Path, revenue: float) -> Path:
    path.write_text(
        f"""
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
          <xbrli:context id="CurrentYearDuration">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period>
              <xbrli:startDate>2024-04-01</xbrli:startDate>
              <xbrli:endDate>2025-03-31</xbrli:endDate>
            </xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <jppfs_cor:NetSales contextRef="CurrentYearDuration" unitRef="JPY">{revenue}</jppfs_cor:NetSales>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )
    return path


def _write_share_class_xbrl(path: Path) -> Path:
    path.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:xbrldi="http://xbrl.org/2006/xbrldi"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor"
            xmlns:jpcrp_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2024-11-01/jpcrp_cor">
          <xbrli:context id="CurrentYearInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="FilingDateInstant_ClassAPreferredSharesMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jpcrp_cor:ClassesOfSharesAxis">jpcrp_cor:ClassAPreferredSharesMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period><xbrli:instant>2025-06-27</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <xbrli:unit id="shares"><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unit>
          <jppfs_cor:Assets contextRef="CurrentYearInstant" unitRef="JPY">1000</jppfs_cor:Assets>
          <jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc
              contextRef="FilingDateInstant_ClassAPreferredSharesMember" unitRef="shares" decimals="0">3800</jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )
    return path


def _build_db(db_path: Path, tmp_path: Path) -> None:
    conn = get_connection(db_path)
    init_db(conn)
    upsert_stock(conn, "1301", "Alpha", "Sector", "Prime")
    upsert_stock(conn, "1302", "Beta", "Sector", "Prime")
    upsert_sec_report(
        conn,
        ticker="1301",
        fiscal_year="2025-03",
        doc_id="S100TEST1",
        xbrl_path=str(_write_financial_xbrl(tmp_path / "1301.xbrl", 200.0)),
    )
    upsert_sec_report(
        conn,
        ticker="1302",
        fiscal_year="2025-03",
        doc_id="S100TEST2",
        xbrl_path=str(_write_financial_xbrl(tmp_path / "1302.xbrl", 300.0)),
    )
    upsert_financial_item(conn, "1301", "2025-03", "pl", "revenue", 100.0, "edinet_xbrl")
    conn.commit()
    conn.close()


def _financial_value(db_path: Path, ticker: str, item_name: str) -> float | None:
    conn = get_connection(db_path)
    try:
        row = conn.execute(
            """
            SELECT value FROM financial_items
            WHERE ticker = ? AND period = '2025-03' AND item_name = ?
            """,
            (ticker, item_name),
        ).fetchone()
        return None if row is None else row["value"]
    finally:
        conn.close()


def test_main_skips_existing_by_default(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path, tmp_path)
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main([])
    captured = capsys.readouterr()

    assert rc == 0
    assert _financial_value(db_path, "1301", "revenue") == pytest.approx(100.0)
    assert _financial_value(db_path, "1302", "revenue") == pytest.approx(300.0)
    assert "Done: 1 ok, 0 errors" in captured.err


def test_main_ticker_still_honors_default_skip_existing(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path, tmp_path)
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main(["--ticker", "1301"])
    captured = capsys.readouterr()

    assert rc == 0
    assert _financial_value(db_path, "1301", "revenue") == pytest.approx(100.0)
    assert "Done: 0 ok, 0 errors" in captured.err


def test_main_force_reparses_existing_ticker(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path, tmp_path)
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main(["--ticker", "1301", "--force"])
    captured = capsys.readouterr()

    assert rc == 0
    assert _financial_value(db_path, "1301", "revenue") == pytest.approx(200.0)
    assert "Done: 1 ok, 0 errors" in captured.err


def test_main_from_ticker_resumes_sorted_tickers(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    _build_db(db_path, tmp_path)
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main(["--from-ticker", "1302", "--force"])
    captured = capsys.readouterr()

    assert rc == 0
    assert _financial_value(db_path, "1301", "revenue") == pytest.approx(100.0)
    assert _financial_value(db_path, "1302", "revenue") == pytest.approx(300.0)
    assert "Done: 1 ok, 0 errors" in captured.err


def test_main_writes_share_classes(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    upsert_stock(conn, "1301", "Alpha", "Sector", "Prime")
    upsert_sec_report(
        conn,
        ticker="1301",
        fiscal_year="2025-03",
        doc_id="S100TEST1",
        xbrl_path=str(_write_share_class_xbrl(tmp_path / "1301.xbrl")),
    )
    conn.commit()
    conn.close()
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main(["--ticker", "1301", "--force"])
    captured = capsys.readouterr()

    conn = get_connection(db_path)
    try:
        preferred = conn.execute(
            """
            SELECT class_name, shares, is_preferred, source_kind
            FROM share_classes
            WHERE ticker = '1301'
            """
        ).fetchone()
        flag = conn.execute(
            """
            SELECT value FROM financial_items
            WHERE ticker = '1301' AND item_name = 'has_preferred_shares'
            """
        ).fetchone()
    finally:
        conn.close()

    assert rc == 0
    assert preferred["class_name"] == "Ａ種優先株式"
    assert preferred["shares"] == pytest.approx(3800.0)
    assert preferred["is_preferred"] == 1
    assert preferred["source_kind"] == "classes_of_shares_axis"
    assert flag["value"] == pytest.approx(1.0)
    assert "Done: 1 ok, 0 errors" in captured.err


def test_main_returns_1_when_no_xbrl_files(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    conn = get_connection(db_path)
    init_db(conn)
    conn.close()
    monkeypatch.setattr(cli, "STOCKS_DB_PATH", db_path)

    rc = cli.main([])
    captured = capsys.readouterr()

    assert rc == 1
    assert "No XBRL files to parse" in captured.err
