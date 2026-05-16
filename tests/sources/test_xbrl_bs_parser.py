"""Tests for EDINET XBRL inventories parser."""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_bs_parser import (
    InventoriesTagMismatchError,
    is_valid_xbrl_path,
    parse_xbrl_bs,
)


def _xbrl_path(ticker: str) -> str:
    ticker_dir = Path(f"/home/exp/projects/stock_db/var/raw/edinet/xbrl/{ticker}")
    db_path = Path("/home/exp/projects/stock_db/var/db/stocks.db")
    if db_path.exists():
        conn = sqlite3.connect(db_path)
        try:
            row = conn.execute(
                """
                SELECT xbrl_path FROM sec_reports
                WHERE ticker = ? AND fiscal_year = 'latest' AND xbrl_path IS NOT NULL
                ORDER BY updated_at DESC, doc_id DESC
                LIMIT 1
                """,
                (ticker,),
            ).fetchone()
        finally:
            conn.close()
        if row and Path(row[0]).is_dir():
            return str(Path(row[0]))
    return str(next(path for path in sorted(ticker_dir.iterdir(), reverse=True) if path.is_dir()))


def _write_api_artifact(root: Path) -> Path:
    artifact = root / "S100PKG"
    public_doc = artifact / "XBRL" / "PublicDoc"
    public_doc.mkdir(parents=True)
    (root / "S100PKG.zip").write_bytes(b"zip")
    (public_doc / "report.xhtml").write_text(
        """
        <html xmlns="http://www.w3.org/1999/xhtml"
              xmlns:ix="http://www.xbrl.org/2008/inlineXBRL"
              xmlns:xbrli="http://www.xbrl.org/2003/instance"
              xmlns:link="http://www.xbrl.org/2003/linkbase"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
              xmlns:jpdei_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpdei/2013-08-31/jpdei_cor"
              xmlns:ext="http://example.com/ext">
          <head></head>
          <body>
            <xbrli:context id="CurrentYearInstant">
              <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
              <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
            </xbrli:context>
            <xbrli:context id="Prior1YearInstant">
              <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
              <xbrli:period><xbrli:instant>2024-03-31</xbrli:instant></xbrli:period>
            </xbrli:context>
            <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
            <ix:hidden>
              <ix:nonnumeric contextRef="CurrentYearInstant" name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>
            </ix:hidden>
            <ix:references>
              <link:schemaref xlink:type="simple" xlink:href="ext.xsd"></link:schemaref>
            </ix:references>
            <ix:nonfraction contextRef="CurrentYearInstant" unitRef="JPY" name="ext:SpecialProjectInventory">600</ix:nonfraction>
            <ix:nonfraction contextRef="Prior1YearInstant" unitRef="JPY" name="ext:SpecialProjectInventory">400</ix:nonfraction>
          </body>
        </html>
        """,
        encoding="utf-8",
    )
    (public_doc / "ext.xsd").write_text(
        """
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                   targetNamespace="http://example.com/ext"
                   xmlns:ext="http://example.com/ext"
                   elementFormDefault="qualified">
          <xs:element id="ext_Inventories" name="Inventories" type="xs:decimal"/>
          <xs:element id="ext_SpecialProjectInventory" name="SpecialProjectInventory" type="xs:decimal"/>
        </xs:schema>
        """,
        encoding="utf-8",
    )
    (public_doc / "ext_pre.xml").write_text(
        """
        <link:linkbase xmlns:link="http://www.xbrl.org/2003/linkbase"
                       xmlns:xlink="http://www.w3.org/1999/xlink">
          <link:presentationLink xlink:type="extended" xlink:role="urn:test:inventories">
            <link:loc xlink:type="locator" xlink:label="loc_root" xlink:href="ext.xsd#ext_Inventories"/>
            <link:loc xlink:type="locator" xlink:label="loc_child" xlink:href="ext.xsd#ext_SpecialProjectInventory"/>
            <link:presentationArc xlink:type="arc" xlink:from="loc_root" xlink:to="loc_child"/>
          </link:presentationLink>
        </link:linkbase>
        """,
        encoding="utf-8",
    )
    return artifact


def _write_single_xbrl(
    path: Path,
    *,
    fact_name: str,
    fact_value: str,
    fact_attrs: str = "",
) -> Path:
    path.write_text(
        f"""
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
          <xbrli:context id="CurrentYearInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <jppfs_cor:{fact_name} contextRef="CurrentYearInstant" unitRef="JPY"{fact_attrs}>{fact_value}</jppfs_cor:{fact_name}>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )
    return path


class TestRealFixtures:
    def test_yoshicon_sums_real_estate_components(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("5280"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(32_983_204_000)
        assert parsed["2024-03"]["inventories"] == pytest.approx(28_505_747_000)

    def test_kudan_prefers_direct_total(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("4425"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(39_840_000)

    def test_toyota_does_not_double_count_ifrs_components(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("7203"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(4_598_232_000_000)

    def test_nyk_ignores_construction_in_progress(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("9001"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(30_621_000_000)

    def test_mitsubishi_heavy_sums_work_in_process(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("6597"))

        assert parsed["2025-06"]["inventories"] == pytest.approx(775_897_000)

    def test_obayashi_picks_cns_raw_materials(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("1934"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(3_422_000_000)

    def test_8881_sums_real_estate_business_costs(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("8881"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(42_683_591_000)

    def test_1802_extracts_current_inventory_totals(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("1802"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(131_646_000_000)
        assert parsed["2024-03"]["inventories"] == pytest.approx(116_948_000_000)

    def test_143a_fixture_contains_inventory_data(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("143A"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(9_003_000)
        assert parsed["2024-03"]["inventories"] == pytest.approx(4_852_000)

    def test_3477_extracts_non_consolidated_inventory_components(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("3477"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(5_596_869_000)


class TestValidationHelpers:
    def test_saved_fixture_path_validation(self) -> None:
        path = Path(_xbrl_path("5280"))
        assert is_valid_xbrl_path(path) is True

    def test_saved_artifact_dir_validation(self, tmp_path: Path) -> None:
        artifact = _write_api_artifact(tmp_path)

        assert is_valid_xbrl_path(artifact) is True

    def test_invalid_saved_path_validation(self, tmp_path: Path) -> None:
        invalid = tmp_path / "header.xhtml"
        invalid.write_text("<html><body>header only</body></html>", encoding="utf-8")

        assert is_valid_xbrl_path(invalid) is False


class TestSyntheticCases:
    def test_returns_empty_for_legacy_flat_dir(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "test.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            "<body><p>header only</p></body></html>",
            encoding="utf-8",
        )

        assert parse_xbrl_bs(str(tmp_path)) == {}

    def test_parses_negative_inventory_value(self, tmp_path: Path) -> None:
        xbrl = _write_single_xbrl(
            tmp_path / "sample.xbrl",
            fact_name="Inventories",
            fact_value="500",
            fact_attrs=' decimals="-3" scale="3" sign="negative"',
        )

        parsed = parse_xbrl_bs(str(xbrl))
        assert parsed["2025-03"]["inventories"] == pytest.approx(-500_000)

    @pytest.mark.parametrize(
        "fact_name",
        [
            "InventoriesCAIFRSIFRS",
            "ProgramRightsAndWorkInProcess",
        ],
    )
    def test_parses_inventory_tags_seen_in_real_filings(self, tmp_path: Path, fact_name: str) -> None:
        xbrl = _write_single_xbrl(
            tmp_path / "sample.xbrl",
            fact_name=fact_name,
            fact_value="500",
            fact_attrs=' decimals="-3" scale="3"',
        )

        parsed = parse_xbrl_bs(str(xbrl))
        assert parsed["2025-03"]["inventories"] == pytest.approx(500_000)

    def test_raises_for_unknown_inventory_like_tag(self, tmp_path: Path) -> None:
        xbrl = _write_single_xbrl(
            tmp_path / "sample.xbrl",
            fact_name="MysteryInventoriesCA",
            fact_value="500",
            fact_attrs=' decimals="-3" scale="3"',
        )

        with pytest.raises(InventoriesTagMismatchError, match="MysteryInventoriesCA"):
            parse_xbrl_bs(str(xbrl))

    def test_parses_investment_in_real_estate_for_sale_component(self, tmp_path: Path) -> None:
        xbrl = _write_single_xbrl(
            tmp_path / "sample.xbrl",
            fact_name="InvestmentInRealEstateForSaleCA",
            fact_value="500",
            fact_attrs=' decimals="-3" scale="3"',
        )

        parsed = parse_xbrl_bs(str(xbrl))
        assert parsed["2025-03"]["inventories"] == pytest.approx(500_000)

    @pytest.mark.parametrize(
        "fact_name",
        [
            "StocksAssetsInvestmentSecuritiesBNK",
            "StocksAssetsINS",
        ],
    )
    def test_ignores_financial_sector_stock_asset_tags(self, tmp_path: Path, fact_name: str) -> None:
        xbrl = _write_single_xbrl(
            tmp_path / "sample.xbrl",
            fact_name=fact_name,
            fact_value="500",
            fact_attrs=' decimals="-3" scale="3"',
        )

        assert parse_xbrl_bs(str(xbrl)) == {}

    def test_ignores_unrelated_shares_facts_with_same_month_values(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "sample.xbrl"
        xbrl.write_text(
            """
            <xbrli:xbrl
                xmlns:xbrli="http://www.xbrl.org/2003/instance"
                xmlns:xbrldi="http://xbrl.org/2006/xbrldi"
                xmlns:jpcrp_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2024-11-01/jpcrp_cor">
              <xbrli:context id="FilingDateInstant">
                <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                <xbrli:period><xbrli:instant>2026-03-24</xbrli:instant></xbrli:period>
              </xbrli:context>
              <xbrli:context id="RecordDateInstant">
                <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                <xbrli:period><xbrli:instant>2026-03-31</xbrli:instant></xbrli:period>
              </xbrli:context>
              <xbrli:unit id="shares"><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unit>
              <jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors
                  contextRef="FilingDateInstant" unitRef="shares" decimals="0">2541051</jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors>
              <jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors
                  contextRef="RecordDateInstant" unitRef="shares" decimals="0">2484500</jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors>
            </xbrli:xbrl>
            """,
            encoding="utf-8",
        )

        assert parse_xbrl_bs(str(xbrl)) == {}

    def test_parses_artifact_dir_via_presentation_fallback(self, tmp_path: Path) -> None:
        artifact = _write_api_artifact(tmp_path)

        parsed = parse_xbrl_bs(str(artifact))

        assert parsed["2025-03"]["inventories"] == pytest.approx(600)
        assert parsed["2024-03"]["inventories"] == pytest.approx(400)

    def test_prefers_publicdoc_xbrl_over_html(self, tmp_path: Path) -> None:
        """When PublicDoc/*.xbrl exists, it should be used instead of iXBRL HTML."""
        artifact = tmp_path / "S100PKG"
        public_doc = artifact / "XBRL" / "PublicDoc"
        public_doc.mkdir(parents=True)
        (tmp_path / "S100PKG.zip").write_bytes(b"zip")

        # iXBRL HTML with different (wrong) value
        (public_doc / "report.xhtml").write_text(
            """
            <html xmlns="http://www.w3.org/1999/xhtml"
                  xmlns:ix="http://www.xbrl.org/2008/inlineXBRL"
                  xmlns:xbrli="http://www.xbrl.org/2003/instance"
                  xmlns:link="http://www.xbrl.org/2003/linkbase"
                  xmlns:xlink="http://www.w3.org/1999/xlink"
                  xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
                  xmlns:jpdei_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpdei/2013-08-31/jpdei_cor"
                  xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
              <head></head>
              <body>
                <xbrli:context id="CurrentYearInstant">
                  <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                  <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
                </xbrli:context>
                <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
                <ix:hidden>
                  <ix:nonnumeric contextRef="CurrentYearInstant" name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>
                </ix:hidden>
                <ix:nonfraction contextRef="CurrentYearInstant" unitRef="JPY" name="jppfs_cor:Inventories">999</ix:nonfraction>
              </body>
            </html>
            """,
            encoding="utf-8",
        )

        # Canonical .xbrl with correct value
        (public_doc / "report.xbrl").write_text(
            """
            <xbrli:xbrl
                xmlns:xbrli="http://www.xbrl.org/2003/instance"
                xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
                xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
              <xbrli:context id="CurrentYearInstant">
                <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
              </xbrli:context>
              <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
              <jppfs_cor:Inventories contextRef="CurrentYearInstant" unitRef="JPY">1234</jppfs_cor:Inventories>
            </xbrli:xbrl>
            """,
            encoding="utf-8",
        )

        parsed = parse_xbrl_bs(str(artifact))
        assert parsed["2025-03"]["inventories"] == pytest.approx(1234)

    def test_falls_back_to_html_when_no_publicdoc_xbrl(self, tmp_path: Path) -> None:
        """When no PublicDoc/*.xbrl exists, iXBRL HTML should still work."""
        artifact = _write_api_artifact(tmp_path)

        parsed = parse_xbrl_bs(str(artifact))
        assert parsed["2025-03"]["inventories"] == pytest.approx(600)
