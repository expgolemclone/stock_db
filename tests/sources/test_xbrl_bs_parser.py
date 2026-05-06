"""Tests for EDINET XBRL inventories parser."""

from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_bs_parser import (
    InventoriesTagMismatchError,
    is_valid_xbrl_path,
    parse_xbrl_bs,
)


def _xbrl_path(ticker: str) -> str:
    return f"/home/exp/projects/stock_db/var/raw/edinet/xbrl/{ticker}"


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

    def test_valid_consolidated_xbrl_without_inventory_tags_returns_zero(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("1802"))

        assert parsed["2025-03"]["inventories"] == 0.0
        assert parsed["2024-03"]["inventories"] == 0.0

    def test_header_only_html_returns_empty(self) -> None:
        assert parse_xbrl_bs(_xbrl_path("143A")) == {}


class TestValidationHelpers:
    def test_saved_fixture_path_validation(self) -> None:
        path = next(Path(_xbrl_path("5280")).glob("*.xhtml"))
        assert is_valid_xbrl_path(path) is True

    def test_saved_artifact_dir_validation(self, tmp_path: Path) -> None:
        artifact = _write_api_artifact(tmp_path)

        assert is_valid_xbrl_path(artifact) is True

    def test_invalid_saved_path_validation(self, tmp_path: Path) -> None:
        invalid = tmp_path / "header.xhtml"
        invalid.write_text("<html><body>header only</body></html>", encoding="utf-8")

        assert is_valid_xbrl_path(invalid) is False


class TestSyntheticCases:
    def test_returns_empty_for_tiny_header_only_file(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "test.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            "<body><p>header only</p></body></html>",
            encoding="utf-8",
        )

        assert parse_xbrl_bs(str(tmp_path)) == {}

    def test_parses_negative_inventory_value(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "S9999.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            '<ix:nonnumeric contextref="FilingDateInstant" '
            'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
            '<body><ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:Inventories" decimals="-3" scale="3" sign="negative">'
            "500</ix:nonfraction></body></html>",
            encoding="utf-8",
        )

        parsed = parse_xbrl_bs(str(tmp_path))
        assert parsed["2025-03"]["inventories"] == pytest.approx(-500_000)

    def test_raises_for_unknown_inventory_like_tag(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "S9998.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            '<ix:nonnumeric contextref="FilingDateInstant" '
            'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
            '<body><ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:MysteryInventoriesCA" decimals="-3" scale="3">'
            "500</ix:nonfraction></body></html>",
            encoding="utf-8",
        )

        with pytest.raises(InventoriesTagMismatchError, match="MysteryInventoriesCA"):
            parse_xbrl_bs(str(tmp_path))

    def test_parses_artifact_dir_via_presentation_fallback(self, tmp_path: Path) -> None:
        artifact = _write_api_artifact(tmp_path)

        parsed = parse_xbrl_bs(str(artifact))

        assert parsed["2025-03"]["inventories"] == pytest.approx(600)
        assert parsed["2024-03"]["inventories"] == pytest.approx(400)
