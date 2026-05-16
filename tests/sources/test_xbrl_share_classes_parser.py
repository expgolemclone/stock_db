from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_financials_parser import parse_xbrl_financials
from stock_db.sources.edinet.xbrl_share_classes_parser import parse_xbrl_share_classes


def test_parses_classes_of_shares_axis_and_preferred_flag(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
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
          <xbrli:context id="FilingDateInstant_OrdinaryShareMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jpcrp_cor:ClassesOfSharesAxis">jpcrp_cor:OrdinaryShareMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period><xbrli:instant>2025-06-27</xbrli:instant></xbrli:period>
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
              contextRef="FilingDateInstant_OrdinaryShareMember" unitRef="shares" decimals="0">1000000</jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc>
          <jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc
              contextRef="FilingDateInstant_ClassAPreferredSharesMember" unitRef="shares" decimals="0">50</jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc>
          <jpcrp_cor:NumberOfIssuedSharesAsOfFilingDateIssuedSharesTotalNumberOfSharesEtc
              contextRef="FilingDateInstant_ClassAPreferredSharesMember" unitRef="shares" decimals="0">60</jpcrp_cor:NumberOfIssuedSharesAsOfFilingDateIssuedSharesTotalNumberOfSharesEtc>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    share_classes = parse_xbrl_share_classes(str(xbrl))
    preferred = next(row for row in share_classes if row["is_preferred"])

    assert {row["class_name"] for row in share_classes} == {"普通株式", "Ａ種優先株式"}
    assert preferred["period"] == "2025-03"
    assert preferred["shares"] == pytest.approx(50)
    assert preferred["source_kind"] == "classes_of_shares_axis"

    parsed = parse_xbrl_financials(str(xbrl))
    assert parsed["2025-03"]["bs"]["has_preferred_shares"] == pytest.approx(1.0)


def test_zero_preferred_shares_do_not_set_preferred_flag(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
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
              contextRef="FilingDateInstant_ClassAPreferredSharesMember" unitRef="shares" decimals="0">0</jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    assert parse_xbrl_share_classes(str(xbrl))[0]["shares"] == pytest.approx(0)
    parsed = parse_xbrl_financials(str(xbrl))
    assert parsed["2025-03"]["bs"]["has_preferred_shares"] == pytest.approx(0.0)


def test_parses_extension_preferred_share_concept_with_label(tmp_path: Path) -> None:
    public_doc = tmp_path / "XBRL" / "PublicDoc"
    public_doc.mkdir(parents=True)
    xbrl = public_doc / "sample.xbrl"
    lab = public_doc / "sample_lab.xml"
    xbrl.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:xbrldi="http://xbrl.org/2006/xbrldi"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor"
            xmlns:ext="http://example.test/ext">
          <xbrli:context id="CurrentYearInstant_NonConsolidatedMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jppfs_cor:ConsolidatedOrNonConsolidatedAxis">jppfs_cor:NonConsolidatedMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:unit id="shares"><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unit>
          <ext:ClassAPreferredSharesTotalNumberOfIssuedSharesSummaryOfBusinessResults
              contextRef="CurrentYearInstant_NonConsolidatedMember" unitRef="shares" decimals="0">3800</ext:ClassAPreferredSharesTotalNumberOfIssuedSharesSummaryOfBusinessResults>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )
    lab.write_text(
        """
        <link:linkbase xmlns:link="http://www.xbrl.org/2003/linkbase"
            xmlns:xlink="http://www.w3.org/1999/xlink"
            xmlns:xml="http://www.w3.org/XML/1998/namespace">
          <link:labelLink xlink:type="extended" xlink:role="http://www.xbrl.org/2003/role/link">
            <link:loc xlink:type="locator" xlink:href="sample.xsd#ClassAPreferredSharesTotalNumberOfIssuedSharesSummaryOfBusinessResults" xlink:label="concept" />
            <link:label xlink:type="resource" xlink:label="label" xml:lang="ja" xlink:role="http://www.xbrl.org/2003/role/label">Ａ種優先株式</link:label>
            <link:labelArc xlink:type="arc" xlink:from="concept" xlink:to="label" xlink:arcrole="http://www.xbrl.org/2003/arcrole/concept-label" />
          </link:labelLink>
        </link:linkbase>
        """,
        encoding="utf-8",
    )

    share_classes = parse_xbrl_share_classes(str(tmp_path))

    assert share_classes == [
        {
            "period": "2025-03",
            "class_key": "http://example.test/ext#ClassAPreferredSharesTotalNumberOfIssuedSharesSummaryOfBusinessResults",
            "class_name": "Ａ種優先株式",
            "shares": 3800.0,
            "is_preferred": True,
            "source_kind": "class_specific_concept",
        }
    ]
