from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError
from stock_db.sources.edinet import xbrl_financials_parser


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


def test_extracts_current_financials_from_real_fixture() -> None:
    parsed = xbrl_financials_parser.parse_xbrl_financials(_xbrl_path("2991"))

    current = parsed["2025-07"]
    assert current["bs"]["current_assets"] == pytest.approx(28_512_698_000)
    assert current["bs"]["current_liabilities"] == pytest.approx(16_911_247_000)
    assert current["bs"]["non_current_liabilities"] == pytest.approx(7_389_962_000)
    assert current["bs"]["inventories"] == pytest.approx(22_763_885_000)
    assert current["bs"]["total_assets"] == pytest.approx(35_386_392_000)
    assert current["bs"]["stockholders_equity"] == pytest.approx(11_059_678_000)
    assert current["bs"]["total_equity"] == pytest.approx(11_085_182_000)
    assert current["bs"]["short_term_debt"] == pytest.approx(9_415_228_000)
    assert current["bs"]["long_term_debt"] == pytest.approx(6_493_630_000)

    assert current["pl"]["revenue"] == pytest.approx(95_992_728_000)
    assert current["pl"]["operating_income"] == pytest.approx(3_744_080_000)
    assert current["pl"]["ordinary_income"] == pytest.approx(3_311_397_000)
    assert current["pl"]["net_income"] == pytest.approx(2_384_052_000)

    assert current["cf"]["operating_cf"] == pytest.approx(-2_601_014_000)
    assert current["cf"]["investing_cf"] == pytest.approx(-1_744_019_000)
    assert current["cf"]["financing_cf"] == pytest.approx(4_841_898_000)

    assert current["dividend"]["dps"] == pytest.approx(20.0)
    assert "forecast" not in current


def test_extracts_non_consolidated_current_financials_from_real_fixture() -> None:
    parsed = xbrl_financials_parser.parse_xbrl_financials(_xbrl_path("3477"))

    current = parsed["2025-03"]
    assert current["bs"]["current_assets"] == pytest.approx(9_278_918_000)
    assert current["bs"]["current_liabilities"] == pytest.approx(5_532_920_000)
    assert current["bs"]["non_current_liabilities"] == pytest.approx(112_069_000)
    assert current["bs"]["inventories"] == pytest.approx(5_596_869_000)

    assert current["pl"]["revenue"] == pytest.approx(14_771_438_000)
    assert current["pl"]["operating_income"] == pytest.approx(591_307_000)
    assert current["pl"]["net_income"] == pytest.approx(550_784_000)

    assert current["cf"]["operating_cf"] == pytest.approx(-736_508_000)
    assert current["cf"]["investing_cf"] == pytest.approx(840_371_000)


def test_extracts_ifrs_cash_flow_tags_from_real_fixture() -> None:
    parsed = xbrl_financials_parser.parse_xbrl_financials(_xbrl_path("6555"))

    current = parsed["2025-02"]
    assert current["cf"]["cash_equivalents"] == pytest.approx(578_930_000)
    assert current["cf"]["operating_cf"] == pytest.approx(406_812_000)
    assert current["cf"]["investing_cf"] == pytest.approx(-130_134_000)
    assert current["cf"]["financing_cf"] == pytest.approx(-30_382_000)


def test_parses_synthetic_forecast_tags(tmp_path: Path) -> None:
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
          <xbrli:context id="CurrentYearDuration">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period>
              <xbrli:startDate>2024-04-01</xbrli:startDate>
              <xbrli:endDate>2025-03-31</xbrli:endDate>
            </xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <xbrli:unit id="JPYPerShares">
            <xbrli:divide>
              <xbrli:unitNumerator><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unitNumerator>
              <xbrli:unitDenominator><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unitDenominator>
            </xbrli:divide>
          </xbrli:unit>
          <jppfs_cor:CurrentAssets contextRef="CurrentYearInstant" unitRef="JPY">1000</jppfs_cor:CurrentAssets>
          <jppfs_cor:CurrentLiabilities contextRef="CurrentYearInstant" unitRef="JPY">400</jppfs_cor:CurrentLiabilities>
          <jppfs_cor:NoncurrentLiabilities contextRef="CurrentYearInstant" unitRef="JPY">100</jppfs_cor:NoncurrentLiabilities>
          <jppfs_cor:Assets contextRef="CurrentYearInstant" unitRef="JPY">1600</jppfs_cor:Assets>
          <jppfs_cor:ShareholdersEquity contextRef="CurrentYearInstant" unitRef="JPY">1100</jppfs_cor:ShareholdersEquity>
          <jppfs_cor:NetAssets contextRef="CurrentYearInstant" unitRef="JPY">1100</jppfs_cor:NetAssets>
          <jppfs_cor:ShortTermLoansPayable contextRef="CurrentYearInstant" unitRef="JPY">200</jppfs_cor:ShortTermLoansPayable>
          <jppfs_cor:LongTermLoansPayable contextRef="CurrentYearInstant" unitRef="JPY">300</jppfs_cor:LongTermLoansPayable>
          <jppfs_cor:NetSales contextRef="CurrentYearDuration" unitRef="JPY">5000</jppfs_cor:NetSales>
          <jppfs_cor:OperatingIncome contextRef="CurrentYearDuration" unitRef="JPY">700</jppfs_cor:OperatingIncome>
          <jppfs_cor:OrdinaryIncome contextRef="CurrentYearDuration" unitRef="JPY">650</jppfs_cor:OrdinaryIncome>
          <jppfs_cor:ProfitLossAttributableToOwnersOfParent contextRef="CurrentYearDuration" unitRef="JPY">500</jppfs_cor:ProfitLossAttributableToOwnersOfParent>
          <jppfs_cor:NetCashProvidedByUsedInOperatingActivities contextRef="CurrentYearDuration" unitRef="JPY">300</jppfs_cor:NetCashProvidedByUsedInOperatingActivities>
          <jppfs_cor:NetCashProvidedByUsedInInvestmentActivities contextRef="CurrentYearDuration" unitRef="JPY">-100</jppfs_cor:NetCashProvidedByUsedInInvestmentActivities>
          <jppfs_cor:NetCashProvidedByUsedInFinancingActivities contextRef="CurrentYearDuration" unitRef="JPY">50</jppfs_cor:NetCashProvidedByUsedInFinancingActivities>
          <jpcrp_cor:DividendPaidPerShareSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPYPerShares">12</jpcrp_cor:DividendPaidPerShareSummaryOfBusinessResults>
          <jpcrp_cor:ForecastNetSalesSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">5500</jpcrp_cor:ForecastNetSalesSummaryOfBusinessResults>
          <jpcrp_cor:ForecastOperatingIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">750</jpcrp_cor:ForecastOperatingIncomeSummaryOfBusinessResults>
          <jpcrp_cor:ForecastOrdinaryIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">700</jpcrp_cor:ForecastOrdinaryIncomeSummaryOfBusinessResults>
          <jpcrp_cor:ForecastNetIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">540</jpcrp_cor:ForecastNetIncomeSummaryOfBusinessResults>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    parsed = xbrl_financials_parser.parse_xbrl_financials(str(xbrl))

    current = parsed["2025-03"]
    assert current["bs"]["current_assets"] == pytest.approx(1000)
    assert current["pl"]["revenue"] == pytest.approx(5000)
    assert current["cf"]["operating_cf"] == pytest.approx(300)
    assert current["dividend"]["dps"] == pytest.approx(12)
    assert current["forecast"]["revenue"] == pytest.approx(5500)
    assert current["forecast"]["operating_income"] == pytest.approx(750)
    assert current["forecast"]["ordinary_income"] == pytest.approx(700)
    assert current["forecast"]["net_income"] == pytest.approx(540)


def test_parses_directors_retirement_benefits_tags(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
          <xbrli:context id="CurrentYearInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="CurrentYearDuration">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period>
              <xbrli:startDate>2024-04-01</xbrli:startDate>
              <xbrli:endDate>2025-03-31</xbrli:endDate>
            </xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <jppfs_cor:ProvisionForDirectorsRetirementBenefits contextRef="CurrentYearInstant" unitRef="JPY">321</jppfs_cor:ProvisionForDirectorsRetirementBenefits>
          <jppfs_cor:ProvisionForDirectorsRetirementBenefitsSGA contextRef="CurrentYearDuration" unitRef="JPY">11</jppfs_cor:ProvisionForDirectorsRetirementBenefitsSGA>
          <jppfs_cor:ReversalOfProvisionForDirectorsRetirementBenefitsNOI contextRef="CurrentYearDuration" unitRef="JPY">12</jppfs_cor:ReversalOfProvisionForDirectorsRetirementBenefitsNOI>
          <jppfs_cor:ReversalOfProvisionForDirectorsRetirementBenefitsEI contextRef="CurrentYearDuration" unitRef="JPY">13</jppfs_cor:ReversalOfProvisionForDirectorsRetirementBenefitsEI>
          <jppfs_cor:ProvisionForDirectorsRetirementBenefitsEL contextRef="CurrentYearDuration" unitRef="JPY">14</jppfs_cor:ProvisionForDirectorsRetirementBenefitsEL>
          <jppfs_cor:IncreaseDecreaseInProvisionForDirectorsRetirementBenefitsOpeCF contextRef="CurrentYearDuration" unitRef="JPY">-15</jppfs_cor:IncreaseDecreaseInProvisionForDirectorsRetirementBenefitsOpeCF>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    current = xbrl_financials_parser.parse_xbrl_financials(str(xbrl))["2025-03"]

    assert current["bs"]["provision_for_directors_retirement_benefits"] == pytest.approx(321)
    assert current["pl"]["provision_for_directors_retirement_benefits_sga"] == pytest.approx(11)
    assert current["pl"]["reversal_of_provision_for_directors_retirement_benefits_noi"] == pytest.approx(12)
    assert current["pl"]["reversal_of_provision_for_directors_retirement_benefits_ei"] == pytest.approx(13)
    assert current["pl"]["provision_for_directors_retirement_benefits_el"] == pytest.approx(14)
    assert current["cf"][
        "increase_decrease_in_provision_for_directors_retirement_benefits_ope_cf"
    ] == pytest.approx(-15)


def test_mixed_retirement_benefits_cf_tag_is_not_directors_item(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
        """
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
          <jppfs_cor:IncreaseDecreaseInProvisionForRetirementBenefitsAndDirectorsRetirementBenefitsOpeCF contextRef="CurrentYearDuration" unitRef="JPY">99</jppfs_cor:IncreaseDecreaseInProvisionForRetirementBenefitsAndDirectorsRetirementBenefitsOpeCF>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    current = xbrl_financials_parser.parse_xbrl_financials(str(xbrl))["2025-03"]

    assert (
        "increase_decrease_in_provision_for_directors_retirement_benefits_ope_cf"
        not in current.get("cf", {})
    )


def test_parses_shares_outstanding_without_other_shares_facts(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor"
            xmlns:jpcrp_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2024-11-01/jpcrp_cor">
          <xbrli:context id="CurrentYearInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="FilingDateInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-06-27</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="RecordDateInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-06-30</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <xbrli:unit id="shares"><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unit>
          <jppfs_cor:Assets contextRef="CurrentYearInstant" unitRef="JPY">1600</jppfs_cor:Assets>
          <jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc
              contextRef="FilingDateInstant" unitRef="shares" decimals="0">1000000</jpcrp_cor:NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc>
          <jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors
              contextRef="FilingDateInstant" unitRef="shares" decimals="0">123</jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors>
          <jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors
              contextRef="RecordDateInstant" unitRef="shares" decimals="0">456</jpcrp_cor:NumberOfSharesHeldOrdinarySharesInformationAboutDirectorsAndCorporateAuditors>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    current = xbrl_financials_parser.parse_xbrl_financials(str(xbrl))["2025-03"]

    assert current["bs"]["total_assets"] == pytest.approx(1600)
    assert current["bs"]["shares_outstanding"] == pytest.approx(1_000_000)


def test_primary_context_wins_over_non_consolidated_fallback(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:xbrldi="http://xbrl.org/2006/xbrldi"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
          <xbrli:context id="CurrentYearInstant">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="CurrentYearDuration">
            <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
            <xbrli:period>
              <xbrli:startDate>2024-04-01</xbrli:startDate>
              <xbrli:endDate>2025-03-31</xbrli:endDate>
            </xbrli:period>
          </xbrli:context>
          <xbrli:context id="CurrentYearInstant_NonConsolidatedMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jppfs_cor:ConsolidatedOrNonConsolidatedAxis">jppfs_cor:NonConsolidatedMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:context id="CurrentYearDuration_NonConsolidatedMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jppfs_cor:ConsolidatedOrNonConsolidatedAxis">jppfs_cor:NonConsolidatedMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period>
              <xbrli:startDate>2024-04-01</xbrli:startDate>
              <xbrli:endDate>2025-03-31</xbrli:endDate>
            </xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <jppfs_cor:CurrentAssets contextRef="CurrentYearInstant" unitRef="JPY">1000</jppfs_cor:CurrentAssets>
          <jppfs_cor:CurrentAssets contextRef="CurrentYearInstant_NonConsolidatedMember" unitRef="JPY">9000</jppfs_cor:CurrentAssets>
          <jppfs_cor:NetSales contextRef="CurrentYearDuration" unitRef="JPY">2000</jppfs_cor:NetSales>
          <jppfs_cor:NetSales contextRef="CurrentYearDuration_NonConsolidatedMember" unitRef="JPY">8000</jppfs_cor:NetSales>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    parsed = xbrl_financials_parser.parse_xbrl_financials(str(xbrl))

    current = parsed["2025-03"]
    assert current["bs"]["current_assets"] == pytest.approx(1000)
    assert current["pl"]["revenue"] == pytest.approx(2000)


def test_segment_non_consolidated_context_is_not_total_fallback(tmp_path: Path) -> None:
    xbrl = tmp_path / "sample.xbrl"
    xbrl.write_text(
        """
        <xbrli:xbrl
            xmlns:xbrli="http://www.xbrl.org/2003/instance"
            xmlns:xbrldi="http://xbrl.org/2006/xbrldi"
            xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
            xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor"
            xmlns:jpcrp_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2024-11-01/jpcrp_cor">
          <xbrli:context id="CurrentYearInstant_NonConsolidatedMember_SegmentMember">
            <xbrli:entity>
              <xbrli:identifier scheme="test">E1</xbrli:identifier>
              <xbrli:segment>
                <xbrldi:explicitMember dimension="jppfs_cor:ConsolidatedOrNonConsolidatedAxis">jppfs_cor:NonConsolidatedMember</xbrldi:explicitMember>
                <xbrldi:explicitMember dimension="jpcrp_cor:OperatingSegmentsAxis">jpcrp_cor:ReportableSegmentsMember</xbrldi:explicitMember>
              </xbrli:segment>
            </xbrli:entity>
            <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
          </xbrli:context>
          <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
          <jppfs_cor:CurrentAssets contextRef="CurrentYearInstant_NonConsolidatedMember_SegmentMember" unitRef="JPY">777</jppfs_cor:CurrentAssets>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    assert xbrl_financials_parser.parse_xbrl_financials(str(xbrl)) == {}


def test_rust_parse_financials_returns_dict() -> None:
    """Verify Rust-backed parse_financials returns the expected shape for a real fixture."""
    from stock_db._edinet_xbrl import parse_financials

    parsed = parse_financials(_xbrl_path("2991"))
    assert "2025-07" in parsed
    assert "bs" in parsed["2025-07"]
