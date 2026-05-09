from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_financials_parser import parse_xbrl_financials


def _xbrl_path(ticker: str) -> str:
    return f"/home/exp/projects/stock_db/var/raw/edinet/xbrl/{ticker}"


def test_extracts_current_financials_from_real_fixture() -> None:
    parsed = parse_xbrl_financials(_xbrl_path("2991"))

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
          <jpcrp_cor:DividendPaidPerShareSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">12</jpcrp_cor:DividendPaidPerShareSummaryOfBusinessResults>
          <jpcrp_cor:ForecastNetSalesSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">5500</jpcrp_cor:ForecastNetSalesSummaryOfBusinessResults>
          <jpcrp_cor:ForecastOperatingIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">750</jpcrp_cor:ForecastOperatingIncomeSummaryOfBusinessResults>
          <jpcrp_cor:ForecastOrdinaryIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">700</jpcrp_cor:ForecastOrdinaryIncomeSummaryOfBusinessResults>
          <jpcrp_cor:ForecastNetIncomeSummaryOfBusinessResults contextRef="CurrentYearDuration" unitRef="JPY">540</jpcrp_cor:ForecastNetIncomeSummaryOfBusinessResults>
        </xbrli:xbrl>
        """,
        encoding="utf-8",
    )

    parsed = parse_xbrl_financials(str(xbrl))

    current = parsed["2025-03"]
    assert current["bs"]["current_assets"] == pytest.approx(1000)
    assert current["pl"]["revenue"] == pytest.approx(5000)
    assert current["cf"]["operating_cf"] == pytest.approx(300)
    assert current["dividend"]["dps"] == pytest.approx(12)
    assert current["forecast"]["revenue"] == pytest.approx(5500)
    assert current["forecast"]["operating_income"] == pytest.approx(750)
    assert current["forecast"]["ordinary_income"] == pytest.approx(700)
    assert current["forecast"]["net_income"] == pytest.approx(540)
