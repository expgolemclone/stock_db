from __future__ import annotations

from pathlib import Path

from stock_db.cli.report_edinet_progress import main
from stock_db.storage.connection import get_connection
from stock_db.storage.schema import init_db
from stock_db.storage.sec_reports import upsert_sec_report
from stock_db.storage.stocks import upsert_company_metadata, upsert_stock


def _build_db(db_path: Path) -> None:
    conn = get_connection(db_path)
    init_db(conn)

    upsert_stock(conn, "1111", "Alpha", "Auto", "Prime", edinet_code="E11111")
    upsert_stock(conn, "1480", "Excluded ETF", "ETF", "ETF")
    upsert_stock(conn, "2222", "Beta", "Tech", "Prime")
    upsert_stock(conn, "3333", "Gamma", "Retail", "Standard")
    upsert_stock(conn, "4444", "Delta", "Retail", "Standard")

    upsert_company_metadata(
        conn, "2222", securities_report_url="https://example.test/S100NOREP.pdf",
    )
    upsert_company_metadata(
        conn, "3333", securities_report_url="https://example.test/S100NOXBRL.pdf",
    )
    upsert_company_metadata(
        conn, "4444", securities_report_url="https://example.test/S100DONE.pdf",
    )

    upsert_sec_report(
        conn,
        ticker="3333",
        fiscal_year="latest",
        doc_id="S100NOXBRL",
        xbrl_path=None,
    )
    upsert_sec_report(
        conn,
        ticker="4444",
        fiscal_year="latest",
        doc_id="S100DONE",
        xbrl_path="var/raw/edinet/xbrl/4444/S100DONE",
    )
    conn.commit()
    conn.close()


class TestReportEdinetProgress:
    def test_prints_summary_and_writes_exports(
        self, tmp_path: Path, capsys: object,
    ) -> None:
        db_path = tmp_path / "stocks.db"
        out_dir = tmp_path / "reports"
        _build_db(db_path)

        rc = main([
            "--db", str(db_path),
            "--output-dir", str(out_dir),
            "--label", "pass1",
        ])
        captured = capsys.readouterr()

        assert rc == 0
        assert "total_stocks: 5" in captured.out
        assert "phase1_pending: 2" in captured.out
        assert "phase1_excluded: 1" in captured.out
        assert "phase1_pending_actionable: 1" in captured.out
        assert "phase2_pending: 2" in captured.out
        assert "with_url_no_report: 1" in captured.out
        assert "with_url_report_no_xbrl: 1" in captured.out

        phase1_path = out_dir / "edinet_phase1_unresolved_pass1.tsv"
        phase1_excluded_path = out_dir / "edinet_phase1_excluded_pass1.tsv"
        no_report_path = out_dir / "edinet_phase2_no_report_pass1.tsv"
        no_xbrl_path = out_dir / "edinet_phase2_no_xbrl_pass1.tsv"

        assert phase1_path.exists()
        assert phase1_excluded_path.exists()
        assert no_report_path.exists()
        assert no_xbrl_path.exists()

        assert "1111\tAlpha\tE11111" in phase1_path.read_text(encoding="utf-8")
        excluded_text = phase1_excluded_path.read_text(encoding="utf-8")
        assert "1480\tExcluded ETF" in excluded_text
        assert "ETF。EDINET提出主体が銘柄名と一致せず" in excluded_text
        assert "2222\tBeta\thttps://example.test/S100NOREP.pdf" in no_report_path.read_text(
            encoding="utf-8",
        )
        no_xbrl_text = no_xbrl_path.read_text(encoding="utf-8")
        assert "3333\tGamma\thttps://example.test/S100NOXBRL.pdf\tS100NOXBRL" in no_xbrl_text

    def test_returns_2_for_missing_db(self, tmp_path: Path, capsys: object) -> None:
        db_path = tmp_path / "missing.db"

        rc = main(["--db", str(db_path)])
        captured = capsys.readouterr()

        assert rc == 2
        assert f"DB not found: {db_path}" in captured.err
