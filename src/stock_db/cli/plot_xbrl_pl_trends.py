from __future__ import annotations

import argparse
import html
import json
import sqlite3
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, Sequence, TypedDict

from stock_db.paths import STOCKS_DB_PATH

ScopeArg = Literal["auto", "consolidated", "non_consolidated"]

# TODO: 同一意味のconcept（例: ImpairmentLoss / ImpairmentLossEL）を名寄せして
#   正規名 → 日本語ラベル の形に統一したい。現状はXBRL concept名ごとに個別エントリ。
_JAPANESE_PL_LABELS: dict[str, str] = {
    "AdvertisingExpensesSGA": "広告宣伝費",
    "BeginningMerchandiseAndFinishedGoodsCOS": "期首商品・製品棚卸高",
    "BusinessConsignmentExpensesSGA": "業務委託費",
    "BusinessStructureImprovementExpensesEL": "事業構造改善費用",
    "CoSponsorFeeNOI": "共催料収入",
    "CommissionFeeNOI": "受取手数料",
    "CommissionForPurchaseOfTreasuryStockNOE": "自己株式取得手数料",
    "CommissionForUnderwritingSecondaryDistributionAndSolicitationForSellingAndOthersForProfessionalInvestorsORSEC": "引受・売出・私募等手数料",
    "CommissionReceivedORSEC": "受取手数料",
    "CommissionToConsigneesORSEC": "委託手数料",
    "CompensationExpensesEL": "補償費",
    "ContractCostsNOE": "業務受託費",
    "ConsumedGoodsInTheCompanyCOS": "社内消費高",
    "CostOfProductsManufactured": "製品製造原価",
    "CostOfPurchasedGoods": "仕入高",
    "CostOfSales": "売上原価",
    "DepreciationSGA": "減価償却費",
    "DirectorsRetirementBenefitsEL": "役員退職慰労金",
    "DisasterLossEL": "災害損失",
    "DividendsIncomeNOI": "受取配当金",
    "EarlyExtraRetirementPaymentsEL": "早期優遇退職給付金",
    "EndingMerchandiseAndFinishedGoodsCOS": "期末商品・製品棚卸高",
    "EquityInEarningsOfAffiliatesNOI": "持分法による投資利益",
    "EquityInLossesOfAffiliatesNOE": "持分法による投資損失",
    "ExtraordinaryIncome": "特別利益",
    "ExtraordinaryLoss": "特別損失",
    "FeeForOfferingSecondaryDistributionAndSolicitationForSellingAndOthersForProfessionalInvestorsORSEC": "売出・私募等に係る費用",
    "FinancialExpensesSEC": "金融費用",
    "FinancialRevenueORSEC": "金融収益",
    "ForeignExchangeGainsNOI": "為替差益",
    "ForeignExchangeLossesNOE": "為替差損",
    "GainOnDonationOfNoncurrentAssetsEI": "固定資産受贈益",
    "GainOnInvestmentsInPartnershipNOI": "投資事業組合運用益",
    "GainOnNegativeGoodwillEI": "負ののれん発生益",
    "GainOnReversalOfSubscriptionRightsToSharesEI": "新株予約権消滅益",
    "GainOnSalesOfInvestmentSecuritiesEI": "投資有価証券売却益",
    "GainOnSalesOfNoncurrentAssetsEI": "固定資産売却益",
    "GainOnSalesOfSubsidiariesAndAffiliatesStocksEI": "関係会社株式売却益",
    "GainOnStepAcquisitionsEI": "段階的取得による利益",
    "GainOnTransferOfBenefitObligationRelatingToEmployeesPensionFundEI": "厚生年金基金代行部分移転益",
    "GrossProfit": "売上総利益",
    "GrossProfitNetGP": "売上総利益純額",
    "ImpairmentLossEL": "減損損失",
    "ImpairmentLossOnGoodwillEL": "のれん減損損失",
    "IncomeBeforeIncomeTaxes": "税金等調整前当期純利益",
    "IncomeBeforeMinorityInterests": "少数株主利益控除前利益",
    "IncomeTaxes": "法人税等",
    "IncomeTaxesCurrent": "法人税等合計",
    "IncomeTaxesDeferred": "法人税等調整額",
    "InsuranceIncomeEI": "保険収入",
    "InterestExpensesNOE": "支払利息",
    "InterestIncomeNOI": "受取利息",
    "InterestOnSecuritiesNOI": "有価証券利息",
    "LossOnBusinessOfSubsidiariesAndAffiliatesEL": "関係会社事業損失",
    "LossOnDisasterEL": "災害損失",
    "LossOnInvestmentsInPartnershipNOE": "投資事業組合運用損",
    "LossOnRedemptionOfInvestmentSecuritiesEL": "投資有価証券償還損",
    "LossOnSalesAndRetirementOfNoncurrentAssetsEL": "固定資産売却除却損",
    "LossOnSalesOfAccountsReceivableNOE": "売上債権売却損",
    "LossOnSalesOfInvestmentSecuritiesEL": "投資有価証券売却損",
    "LossOnValuationOfInvestmentSecuritiesEL": "投資有価証券評価損",
    "LossOnValuationOfInvestmentsInCapitalOfSubsidiariesAndAffiliatesEL": "関係会社出資評価損",
    "LossOnValuationOfStocksOfSubsidiariesAndAffiliatesEL": "関係会社株式評価損",
    "MembershipFeeIncomeNOI": "会費収入",
    "MinorityInterestsInIncome": "少数株主利益",
    "NetIncome": "当期純利益",
    "NetOperatingRevenueSEC": "営業収益純額",
    "NetSales": "売上高",
    "NetSalesOfFinishedGoodsRevOA": "製品売上高",
    "NetSalesOfGoodsRevOA": "商品売上高",
    "NetTradingIncomeORSEC": "トレーディング純収益",
    "NonOperatingExpenses": "営業外費用",
    "NonOperatingIncome": "営業外収益",
    "OfficeCostSGA": "事務費",
    "OfficeTransferExpensesEL": "事務所移転費用",
    "OperatingIncome": "営業利益",
    "OperatingRevenueSEC": "営業収益",
    "OrdinaryIncome": "経常利益",
    "OtherEI": "その他特別利益",
    "OtherEL": "その他特別損失",
    "OtherFeesReceivedORSEC": "その他受取手数料",
    "OtherNOE": "その他営業外費用",
    "OtherNOI": "その他営業外収益",
    "OtherSGA": "その他販管費",
    "PersonalExpensesSGA": "人件費",
    "ProfitLoss": "当期純利益",
    "ProfitLossAttributableToNonControllingInterests": "非支配株主に帰属する当期純利益",
    "ProfitLossAttributableToOwnersOfParent": "親会社株主に帰属する当期純利益",
    "ProvisionForDirectorsBonusesSGA": "役員賞与引当金繰入額",
    "ProvisionForLossOnDisasterEL": "災害損失引当金繰入額",
    "ProvisionForSalesReturnsGP": "返品調整引当金繰入額",
    "ProvisionOfAllowanceForDoubtfulAccountsEL": "貸倒引当金繰入額",
    "ProvisionOfAllowanceForDoubtfulAccountsSGA": "貸倒引当金繰入額",
    "ProvisionOfReserveForCommoditiesTransactionLiabilitiesEL": "商品取引引当金繰入額",
    "ProvisionOfReserveForFinancialProductsTransactionLiabilitiesELSEC": "金融商品取引引当金繰入額",
    "PurchaseDiscountsNOI": "仕入割引",
    "RealEstateExpensesSGASEC": "不動産費用",
    "RefundedConsumptionTaxesNOI": "消費税等還付金",
    "ResearchAndDevelopmentExpensesSGA": "研究開発費",
    "RetirementBenefitExpensesSGA": "退職給付費用",
    "ReversalOfAssetRetirementObligationEISEC": "除却債務戻入益",
    "ReversalOfProvisionForSalesReturnsGP": "返品調整引当金戻入額",
    "ReversalOfReserveForCommoditiesTransactionLiabilitiesEI": "商品取引引当金戻入益",
    "ReversalOfReserveForFinancialProductsTransactionLiabilitiesEISEC": "金融商品取引引当金戻入益",
    "SalesDiscountsNOE": "売上割引",
    "SellingGeneralAndAdministrativeExpenses": "販売費及び一般管理費",
    "SubsidyIncomeNOIBounty": "助成金収入",
    "SurrenderValueOfInsuranceEI": "保険解約返戻金",
    "TaxesAndDuesSGA": "租税公課",
    "TechnicalAdviseFeeNOI": "技術指導料収入",
    "TechnicalAdvisoryFeeNOI": "技術助成金収入",
    "TotalBeginningAndCostPurchasedMerchandiseAndFinishedGoodsCOS": "期首商品・製品棚卸高及び仕入高合計",
    "TradingRelatedExpensesSGASEC": "トレーディング関連費用",
}


class SelectedReport(TypedDict):
    period: str
    doc_id: str
    consolidation_scope: str
    fact_count: int
    valued_count: int
    role_fact_count: int


class PlTrendItem(TypedDict):
    concept_name: str
    label: str
    values: list[float | None]
    latest_value: float | None
    non_null_count: int
    min_value: float | None
    max_value: float | None
    total_abs_value: float


class PlTrendPayload(TypedDict):
    ticker: str
    name: str
    source: str
    requested_scope: str
    periods: list[str]
    selected_reports: list[SelectedReport]
    items: list[PlTrendItem]


class PlaywrightOpenResult(TypedDict):
    title: str
    item_count: int
    canvas_width: int
    canvas_height: int
    drawn_pixels: int
    screenshot_path: str | None


@dataclass(frozen=True)
class _CandidateReport:
    period: str
    doc_id: str
    consolidation_scope: str
    fact_count: int
    valued_count: int
    role_fact_count: int


def _connect_readonly(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(f"file:{db_path.resolve()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    return conn


def _period_placeholders(periods: Sequence[str]) -> str:
    return ",".join("?" for _ in periods)


def _get_stock_name(conn: sqlite3.Connection, ticker: str) -> str:
    row = conn.execute(
        "SELECT name FROM stocks WHERE ticker = ?",
        (ticker,),
    ).fetchone()
    return "" if row is None else str(row["name"] or "")


def _get_latest_periods(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    n_periods: int,
) -> list[str]:
    rows = conn.execute(
        """
        SELECT DISTINCT f.period
        FROM xbrl_profit_loss_facts AS f
        JOIN data_sources AS ds
          ON ds.source_id = f.source_id
        WHERE f.ticker = ?
          AND ds.source_code = ?
          AND f.value IS NOT NULL
        ORDER BY f.period DESC
        LIMIT ?
        """,
        (ticker, source, n_periods),
    ).fetchall()
    return [str(row["period"]) for row in reversed(rows)]


def _candidate_rank(candidate: _CandidateReport, requested_scope: ScopeArg) -> tuple:
    if requested_scope == "consolidated":
        scope_rank = 0 if candidate.consolidation_scope == "consolidated" else 1
    elif requested_scope == "non_consolidated":
        scope_rank = 0 if candidate.consolidation_scope == "non_consolidated" else 1
    elif candidate.consolidation_scope == "consolidated" and candidate.role_fact_count > 0:
        scope_rank = 0
    elif candidate.consolidation_scope == "non_consolidated" and candidate.role_fact_count > 0:
        scope_rank = 1
    elif candidate.consolidation_scope == "consolidated":
        scope_rank = 2
    else:
        scope_rank = 3
    return (
        scope_rank,
        -candidate.role_fact_count,
        -candidate.fact_count,
        _descending_text_key(candidate.doc_id),
    )


def _descending_text_key(value: str) -> tuple[int, ...]:
    return tuple(-ord(ch) for ch in value)


def _select_reports(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    periods: list[str],
    requested_scope: ScopeArg,
) -> dict[str, _CandidateReport]:
    if not periods:
        return {}

    scope_filter = ""
    params: list[str] = [ticker, source, *periods]
    if requested_scope != "auto":
        scope_filter = "AND f.consolidation_scope = ?"
        params.append(requested_scope)

    rows = conn.execute(
        f"""
        SELECT
            f.period,
            f.doc_id,
            f.consolidation_scope,
            COUNT(*) AS fact_count,
            SUM(CASE WHEN f.value IS NOT NULL THEN 1 ELSE 0 END) AS valued_count,
            SUM(CASE WHEN f.role_uri <> '' THEN 1 ELSE 0 END) AS role_fact_count
        FROM xbrl_profit_loss_facts AS f
        JOIN data_sources AS ds
          ON ds.source_id = f.source_id
        WHERE f.ticker = ?
          AND ds.source_code = ?
          AND f.period IN ({_period_placeholders(periods)})
          AND f.value IS NOT NULL
          {scope_filter}
        GROUP BY f.period, f.doc_id, f.consolidation_scope
        """,
        params,
    ).fetchall()

    candidates_by_period: dict[str, list[_CandidateReport]] = {}
    for row in rows:
        candidate = _CandidateReport(
            period=str(row["period"]),
            doc_id=str(row["doc_id"]),
            consolidation_scope=str(row["consolidation_scope"]),
            fact_count=int(row["fact_count"]),
            valued_count=int(row["valued_count"]),
            role_fact_count=int(row["role_fact_count"] or 0),
        )
        candidates_by_period.setdefault(candidate.period, []).append(candidate)

    selected: dict[str, _CandidateReport] = {}
    for period in periods:
        candidates = candidates_by_period.get(period, [])
        if candidates:
            selected[period] = sorted(
                candidates,
                key=lambda candidate: _candidate_rank(candidate, requested_scope),
            )[0]
    return selected


def _fact_rank(row: sqlite3.Row, scope: str) -> tuple:
    dimensions_json = str(row["dimensions_json"] or "")
    role_uri = str(row["role_uri"] or "")
    preferred_role = (
        scope == "consolidated" and "Consolidated" in role_uri
    ) or (
        scope == "non_consolidated" and "Consolidated" not in role_uri
    )
    return (
        0 if dimensions_json == "[]" else 1,
        len(dimensions_json),
        0 if preferred_role else 1,
        0 if role_uri else 1,
        role_uri,
        str(row["context_id"] or ""),
    )


def _label_score(label: str, concept_name: str) -> tuple[int, str]:
    if label and label != concept_name and any(ord(ch) > 127 for ch in label):
        return (0, label)
    if label and label != concept_name:
        return (1, label)
    return (2, concept_name)


def _display_label(concept_name: str, labels: dict[str, str]) -> str:
    mapped = _JAPANESE_PL_LABELS.get(concept_name)
    if mapped is not None:
        return mapped
    label = labels.get(concept_name, concept_name)
    if label != concept_name and any(ord(ch) > 127 for ch in label):
        return label
    return concept_name


def _get_latest_doc_for_order(selected_reports: dict[str, _CandidateReport]) -> _CandidateReport:
    return selected_reports[sorted(selected_reports)[-1]]


def _get_presentation_order_and_labels(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    doc_id: str,
    scope: str,
) -> tuple[list[str], dict[str, str]]:
    rows = conn.execute(
        """
        SELECT
            e.role_uri,
            e.parent_namespace,
            e.parent_name,
            e.parent_label,
            e.child_namespace,
            e.child_name,
            e.child_label,
            e.child_order
        FROM xbrl_statement_presentation_edges AS e
        JOIN data_sources AS ds
          ON ds.source_id = e.source_id
        WHERE e.ticker = ?
          AND e.doc_id = ?
          AND e.statement = 'pl'
          AND ds.source_code = ?
        ORDER BY e.role_uri, e.parent_name, e.child_order, e.child_name
        """,
        (ticker, doc_id, source),
    ).fetchall()

    labels: dict[tuple[str, str, str], str] = {}
    best_label_by_concept: dict[str, str] = {}
    children_by_parent: dict[tuple[str, str, str], list[tuple[float, tuple[str, str, str]]]] = {}
    parents: set[tuple[str, str, str]] = set()
    children: set[tuple[str, str, str]] = set()

    for row in rows:
        role_uri = str(row["role_uri"])
        parent = (role_uri, str(row["parent_namespace"]), str(row["parent_name"]))
        child = (role_uri, str(row["child_namespace"]), str(row["child_name"]))
        parent_label = str(row["parent_label"] or parent[2])
        child_label = str(row["child_label"] or child[2])
        labels[parent] = parent_label
        labels[child] = child_label
        parents.add(parent)
        children.add(child)
        children_by_parent.setdefault(parent, []).append((float(row["child_order"] or 0), child))

        concept_name = child[2]
        current = best_label_by_concept.get(concept_name, concept_name)
        if _label_score(child_label, concept_name) < _label_score(current, concept_name):
            best_label_by_concept[concept_name] = child_label
        else:
            best_label_by_concept.setdefault(concept_name, current)

    preferred_consolidated = scope == "consolidated"

    def role_rank(key: tuple[str, str, str]) -> tuple[int, str, str]:
        role_uri = key[0]
        if preferred_consolidated:
            rank = 0 if "Consolidated" in role_uri else 1
        else:
            rank = 0 if "Consolidated" not in role_uri else 1
        return (rank, role_uri, key[2])

    ordered: list[str] = []
    seen: set[str] = set()

    def visit(key: tuple[str, str, str]) -> None:
        concept_name = key[2]
        if concept_name not in seen:
            seen.add(concept_name)
            ordered.append(concept_name)
        for _order, child in sorted(
            children_by_parent.get(key, []),
            key=lambda item: (
                item[0],
                role_rank(item[1]),
                labels.get(item[1], item[1][2]),
                item[1][2],
            ),
        ):
            visit(child)

    for root in sorted(parents - children, key=role_rank):
        visit(root)

    return ordered, best_label_by_concept


def _get_values_by_period(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str,
    periods: list[str],
    selected_reports: dict[str, _CandidateReport],
    labels: dict[str, str],
) -> dict[str, list[float | None]]:
    values: dict[str, list[float | None]] = {}

    for index, period in enumerate(periods):
        selected = selected_reports[period]
        rows = conn.execute(
            """
            SELECT
                f.concept_name,
                f.role_uri,
                f.context_id,
                f.dimensions_json,
                f.value
            FROM xbrl_profit_loss_facts AS f
            JOIN data_sources AS ds
              ON ds.source_id = f.source_id
            WHERE f.ticker = ?
              AND f.period = ?
              AND f.doc_id = ?
              AND f.consolidation_scope = ?
              AND ds.source_code = ?
              AND f.value IS NOT NULL
            """,
            (
                ticker,
                period,
                selected.doc_id,
                selected.consolidation_scope,
                source,
            ),
        ).fetchall()

        for row in sorted(rows, key=lambda value: _fact_rank(value, selected.consolidation_scope)):
            concept_name = str(row["concept_name"])
            bucket = values.setdefault(concept_name, [None] * len(periods))
            if bucket[index] is None:
                bucket[index] = row["value"]
                labels.setdefault(concept_name, concept_name)

    return values


def build_pl_trend_payload(
    conn: sqlite3.Connection,
    *,
    ticker: str,
    source: str = "edinet_xbrl",
    n_periods: int = 10,
    scope: ScopeArg = "auto",
) -> PlTrendPayload:
    periods = _get_latest_periods(conn, ticker=ticker, source=source, n_periods=n_periods)
    if not periods:
        raise ValueError(f"No PL XBRL facts found for ticker {ticker}")

    selected_reports = _select_reports(
        conn,
        ticker=ticker,
        source=source,
        periods=periods,
        requested_scope=scope,
    )
    missing_periods = [period for period in periods if period not in selected_reports]
    if missing_periods:
        joined = ", ".join(missing_periods)
        raise ValueError(f"No {scope} PL XBRL facts found for {ticker}: {joined}")

    latest_report = _get_latest_doc_for_order(selected_reports)
    presentation_order, labels = _get_presentation_order_and_labels(
        conn,
        ticker=ticker,
        source=source,
        doc_id=latest_report.doc_id,
        scope=latest_report.consolidation_scope,
    )
    values = _get_values_by_period(
        conn,
        ticker=ticker,
        source=source,
        periods=periods,
        selected_reports=selected_reports,
        labels=labels,
    )

    order_index = {concept_name: index for index, concept_name in enumerate(presentation_order)}
    concepts = sorted(
        values,
        key=lambda concept_name: (
            order_index.get(concept_name, len(order_index)),
            labels.get(concept_name, concept_name),
            concept_name,
        ),
    )

    items: list[PlTrendItem] = []
    for concept_name in concepts:
        item_values = values[concept_name]
        non_null_values = [value for value in item_values if value is not None]
        latest_value = next((value for value in reversed(item_values) if value is not None), None)
        items.append(
            {
                "concept_name": concept_name,
                "label": _display_label(concept_name, labels),
                "values": item_values,
                "latest_value": latest_value,
                "non_null_count": len(non_null_values),
                "min_value": min(non_null_values) if non_null_values else None,
                "max_value": max(non_null_values) if non_null_values else None,
                "total_abs_value": sum(abs(value) for value in non_null_values),
            }
        )

    selected_rows: list[SelectedReport] = [
        {
            "period": period,
            "doc_id": selected_reports[period].doc_id,
            "consolidation_scope": selected_reports[period].consolidation_scope,
            "fact_count": selected_reports[period].fact_count,
            "valued_count": selected_reports[period].valued_count,
            "role_fact_count": selected_reports[period].role_fact_count,
        }
        for period in periods
    ]
    return {
        "ticker": ticker,
        "name": _get_stock_name(conn, ticker),
        "source": source,
        "requested_scope": scope,
        "periods": periods,
        "selected_reports": selected_rows,
        "items": items,
    }


def _json_for_script(payload: PlTrendPayload) -> str:
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")


def render_html(payload: PlTrendPayload) -> str:
    title = f"{payload['ticker']} 損益計算書推移"
    escaped_title = html.escape(title)
    data_json = _json_for_script(payload)
    return f"""<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{escaped_title}</title>
<style>
:root {{
  color-scheme: dark;
  --paper: #0d100f;
  --ink: #f0eadb;
  --muted: #a8ad9f;
  --line: #34392f;
  --panel: #151917;
  --panel-2: #1f2a24;
  --accent: #55d6c2;
  --accent-2: #ff6b57;
  --accent-3: #f4b942;
  --warn: #f2a65a;
  --shadow: rgba(0, 0, 0, 0.45);
}}
* {{ box-sizing: border-box; }}
html {{
  width: 100%;
  max-width: 100%;
  overflow-x: clip;
}}
body {{
  margin: 0;
  width: 100%;
  max-width: 100%;
  overflow-x: clip;
  background:
    radial-gradient(circle at 12% 8%, rgba(85,214,194,0.09), transparent 28%),
    linear-gradient(90deg, rgba(240,234,219,0.045) 1px, transparent 1px) 0 0 / 22px 22px,
    linear-gradient(rgba(240,234,219,0.035) 1px, transparent 1px) 0 0 / 22px 22px,
    var(--paper);
  color: var(--ink);
  font-family: "IBM Plex Sans", "Yu Gothic", "Hiragino Kaku Gothic ProN", sans-serif;
  letter-spacing: 0;
}}
button, input {{ font: inherit; }}
.shell {{
  width: 100%;
  min-width: 0;
  min-height: 100vh;
  display: grid;
  grid-template-rows: auto 1fr;
  overflow-x: clip;
}}
header {{
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 24px;
  align-items: end;
  padding: 28px clamp(18px, 3vw, 44px) 20px;
  border-bottom: 2px solid var(--ink);
  background: rgba(13, 16, 15, 0.94);
  min-width: 0;
}}
header > * {{
  min-width: 0;
}}
.kicker {{
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  color: var(--muted);
  font-size: 12px;
  text-transform: uppercase;
}}
h1 {{
  margin: 4px 0 0;
  font-family: Georgia, "Yu Mincho", "Hiragino Mincho ProN", serif;
  font-size: clamp(30px, 5vw, 64px);
  line-height: 0.95;
  font-weight: 700;
}}
.meta {{
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  min-width: min(100%, 320px);
}}
.stat {{
  min-width: 0;
  min-height: 58px;
  padding: 9px 12px;
  border: 1px solid var(--ink);
  background: var(--panel);
  box-shadow: 4px 4px 0 rgba(240,234,219,0.2);
}}
.stat b {{
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 18px;
  line-height: 1.1;
}}
.stat span {{
  color: var(--muted);
  font-size: 11px;
}}
main {{
  display: grid;
  grid-template-columns: minmax(280px, 380px) minmax(0, 1fr);
  width: 100%;
  min-width: 0;
  min-height: 0;
  overflow-x: clip;
}}
.rail {{
  min-width: 0;
  border-right: 2px solid var(--ink);
  min-height: calc(100vh - 120px);
  background: rgba(13,16,15,0.72);
}}
.controls {{
  position: sticky;
  top: 0;
  z-index: 3;
  display: grid;
  gap: 10px;
  padding: 16px;
  border-bottom: 1px solid var(--ink);
  background: var(--panel);
}}
.search {{
  width: 100%;
  height: 40px;
  border: 1px solid var(--ink);
  background: #0f1311;
  color: var(--ink);
  padding: 0 12px;
  outline-offset: 3px;
}}
.seg {{
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  border: 1px solid var(--ink);
}}
.seg button {{
  height: 36px;
  border: 0;
  border-right: 1px solid var(--ink);
  background: #0f1311;
  color: var(--ink);
  cursor: pointer;
}}
.seg button:last-child {{ border-right: 0; }}
.seg button.active {{
  background: var(--ink);
  color: var(--panel);
}}
.items {{
  display: grid;
}}
.item {{
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) 92px;
  gap: 10px;
  align-items: center;
  min-height: 70px;
  padding: 11px 14px;
  border-bottom: 1px solid var(--line);
  background: rgba(21,25,23,0.78);
  cursor: pointer;
  text-align: left;
}}
.item:hover, .item.selected {{
  background: var(--panel-2);
}}
.item.selected {{
  box-shadow: inset 5px 0 0 var(--accent);
}}
.item-toggle {{
  width: 18px;
  height: 18px;
  border: 1px solid var(--ink);
  background: #0f1311;
  box-shadow: 2px 2px 0 rgba(240,234,219,0.16);
}}
.item.selected .item-toggle {{
  border-color: var(--series-color);
  background: var(--series-color);
  box-shadow: 0 0 0 2px rgba(240,234,219,0.18);
}}
.item-name {{
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 700;
}}
.item-code {{
  display: block;
  margin-top: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--muted);
  font-size: 11px;
}}
.item-value {{
  display: block;
  margin-top: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
  color: var(--accent);
  font-weight: 700;
}}
.spark {{
  width: 92px;
  height: 45px;
}}
.stage {{
  min-width: 0;
  width: 100%;
  max-width: 100%;
  padding: clamp(16px, 3vw, 36px);
  overflow-x: clip;
}}
.chart-panel {{
  min-width: 0;
  max-width: 100%;
  border: 2px solid var(--ink);
  background: var(--panel);
  box-shadow: 8px 8px 0 rgba(240,234,219,0.16);
}}
.chart-head {{
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 16px;
  padding: 18px 18px 10px;
  border-bottom: 1px solid var(--ink);
  min-width: 0;
}}
.selected-label {{
  margin: 0;
  overflow-wrap: anywhere;
  font-family: Georgia, "Yu Mincho", "Hiragino Mincho ProN", serif;
  font-size: clamp(24px, 3.8vw, 48px);
  line-height: 1;
}}
.selected-code {{
  margin-top: 5px;
  color: var(--muted);
  overflow-wrap: anywhere;
  font-size: 12px;
}}
.latest {{
  min-width: min(240px, 100%);
  text-align: left;
  font-variant-numeric: tabular-nums;
}}
.latest-list {{
  display: grid;
  gap: 5px;
  margin-top: 6px;
}}
.latest-row {{
  display: grid;
  grid-template-columns: 9px minmax(0, 1fr) auto;
  gap: 7px;
  align-items: center;
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 12px;
}}
.latest-row b {{
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 700;
}}
.latest-row strong {{
  color: var(--series-color);
  font-size: 13px;
}}
.series-dot {{
  width: 9px;
  height: 9px;
  background: var(--series-color);
  box-shadow: 0 0 0 1px var(--ink);
}}
.legend {{
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 10px;
}}
.legend-chip {{
  display: inline-grid;
  grid-template-columns: 9px minmax(0, 1fr);
  gap: 7px;
  align-items: center;
  max-width: 220px;
  padding: 5px 8px;
  border: 1px solid var(--line);
  background: #101411;
  color: var(--ink);
  font-size: 12px;
}}
.legend-chip span:last-child {{
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}}
.canvas-wrap {{
  position: relative;
  width: 100%;
  height: clamp(300px, 48vh, 560px);
  padding: 14px;
}}
#detailChart {{
  width: 100%;
  height: 100%;
  display: block;
}}
.tooltip {{
  position: absolute;
  pointer-events: none;
  display: none;
  min-width: 180px;
  max-width: min(320px, calc(100% - 16px));
  padding: 8px 10px;
  border: 1px solid var(--ink);
  background: #101411;
  box-shadow: 3px 3px 0 rgba(240,234,219,0.18);
  font-size: 12px;
}}
.tooltip-row {{
  display: grid;
  grid-template-columns: 9px minmax(0, 1fr) auto;
  gap: 7px;
  align-items: center;
  margin-top: 6px;
}}
.tooltip-row span:nth-child(2) {{
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}}
.tooltip-row strong {{
  color: var(--series-color);
  font-weight: 700;
}}
.period-strip {{
  display: grid;
  grid-template-columns: repeat(var(--period-count), minmax(46px, 1fr));
  max-width: 100%;
  overflow-x: auto;
  overscroll-behavior-x: contain;
  border-top: 1px solid var(--ink);
}}
.period-cell {{
  min-width: 46px;
  padding: 8px 6px;
  border-right: 1px solid var(--line);
  text-align: center;
  color: var(--muted);
  font-size: 11px;
}}
.period-cell:last-child {{ border-right: 0; }}
.table-panel {{
  margin-top: 28px;
  width: 100%;
  max-width: 100%;
  border: 1px solid var(--ink);
  background: rgba(21,25,23,0.8);
  overflow-x: auto;
  overscroll-behavior-x: contain;
  contain: inline-size layout paint;
}}
table {{
  width: max-content;
  border-collapse: collapse;
  min-width: 760px;
}}
th, td {{
  padding: 9px 10px;
  border-bottom: 1px solid var(--line);
  text-align: right;
  font-variant-numeric: tabular-nums;
}}
th:first-child, td:first-child {{
  position: sticky;
  left: 0;
  z-index: 1;
  text-align: left;
  background: var(--panel);
  min-width: 260px;
}}
th {{
  background: var(--ink);
  color: var(--paper);
  font-size: 12px;
}}
td:first-child b {{
  display: block;
  overflow-wrap: anywhere;
}}
td:first-child span {{
  color: var(--muted);
  font-size: 11px;
  overflow-wrap: anywhere;
}}
tr.selected-row td {{
  background: rgba(85,214,194,0.08);
}}
tr.selected-row td:first-child {{
  box-shadow: inset 4px 0 0 var(--accent);
}}
.empty {{
  padding: 28px 16px;
  color: var(--muted);
}}
@media (max-width: 1200px) {{
  header {{
    grid-template-columns: 1fr;
  }}
  .meta {{
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }}
  main {{
    grid-template-columns: 1fr;
  }}
  .rail {{
    min-height: auto;
    border-right: 0;
    border-bottom: 2px solid var(--ink);
  }}
  .controls {{
    position: static;
  }}
  .items {{
    max-height: 360px;
    overflow: auto;
  }}
}}
@media (max-width: 640px) {{
  header {{
    padding: 22px 18px 16px;
  }}
  .meta {{
    grid-template-columns: 1fr;
    min-width: 0;
  }}
  h1 {{
    font-size: clamp(30px, 10vw, 42px);
  }}
  .item {{
    grid-template-columns: 24px minmax(0, 1fr) 80px;
  }}
  .spark {{
    width: 80px;
  }}
  .chart-head {{
    grid-template-columns: 1fr;
  }}
  .latest {{
    text-align: left;
  }}
  .canvas-wrap {{
    height: clamp(260px, 48vh, 420px);
    padding: 10px;
  }}
  .period-strip {{
    grid-template-columns: repeat(var(--period-count), minmax(64px, 1fr));
  }}
  .period-cell {{
    min-width: 64px;
  }}
  table {{
    min-width: 720px;
  }}
}}
</style>
</head>
<body>
<div class="shell">
  <header>
    <div>
      <div class="kicker">
        <span>EDINET XBRL</span>
        <span id="scopeLabel"></span>
        <span id="sourceLabel"></span>
      </div>
      <h1 id="title"></h1>
    </div>
    <div class="meta">
      <div class="stat"><b id="periodCount"></b><span>対象期間</span></div>
      <div class="stat"><b id="itemCount"></b><span>PL項目</span></div>
      <div class="stat"><b id="latestDoc"></b><span>最新書類</span></div>
    </div>
  </header>
  <main>
    <aside class="rail">
      <div class="controls">
        <input id="search" class="search" type="search" autocomplete="off" placeholder="項目名・XBRL名で検索">
        <div class="seg" role="group" aria-label="unit">
          <button type="button" data-unit="1">円</button>
          <button type="button" data-unit="1000000" class="active">百万円</button>
          <button type="button" data-unit="100000000">億円</button>
        </div>
      </div>
      <div id="items" class="items"></div>
    </aside>
    <section class="stage">
      <div class="chart-panel">
        <div class="chart-head">
          <div>
            <h2 id="selectedLabel" class="selected-label"></h2>
            <div id="selectedCode" class="selected-code"></div>
            <div id="legend" class="legend"></div>
          </div>
          <div class="latest">
            <span>最新値</span>
            <div id="latestValues" class="latest-list"></div>
          </div>
        </div>
        <div class="canvas-wrap">
          <canvas id="detailChart"></canvas>
          <div id="tooltip" class="tooltip"></div>
        </div>
        <div id="periodStrip" class="period-strip"></div>
      </div>
      <div class="table-panel">
        <table>
          <thead id="tableHead"></thead>
          <tbody id="tableBody"></tbody>
        </table>
      </div>
    </section>
  </main>
</div>
<script>
window.PL_TREND_DATA = {data_json};
</script>
<script>
const data = window.PL_TREND_DATA;
let unit = 1000000;
const initialItem = data.items.find((item) => item.concept_name === "NetSales") || data.items[0];
let selectedConcepts = initialItem ? [initialItem.concept_name] : [];
let chartResizeFrame = 0;
const colors = ["#55d6c2", "#ff6b57", "#f4b942", "#9ad66d", "#b58cff", "#5da9ff", "#ff8fb3", "#d8d456", "#8ce0a3", "#d89b6a"];
const canvasTheme = {{
  grid: "#34392f",
  text: "#a8ad9f",
  zero: "#f0eadb",
  pointFill: "#151917"
}};

const el = (id) => document.getElementById(id);
const seriesColor = (index) => colors[index % colors.length];
const selectedItems = () => selectedConcepts
  .map((concept) => data.items.find((item) => item.concept_name === concept))
  .filter(Boolean);
const selectedIndexOf = (item) => selectedConcepts.indexOf(item.concept_name);
const scopeText = (scope) => scope === "consolidated" ? "連結" : scope === "non_consolidated" ? "個別" : scope;
const sourceText = (source) => source === "edinet_xbrl" ? "EDINET XBRL" : source;
const unitLabel = () => unit === 1 ? "円" : unit === 1000000 ? "百万円" : "億円";
const scaleValue = (value) => value === null || value === undefined ? null : value / unit;
const formatValue = (value) => {{
  if (value === null || value === undefined || Number.isNaN(value)) return "-";
  const scaled = scaleValue(value);
  const abs = Math.abs(scaled);
  const digits = abs >= 100 ? 0 : abs >= 10 ? 1 : 2;
  return scaled.toLocaleString("ja-JP", {{ maximumFractionDigits: digits }}) + " " + unitLabel();
}};
const compactValue = (value) => {{
  if (value === null || value === undefined || Number.isNaN(value)) return "-";
  const scaled = scaleValue(value);
  return scaled.toLocaleString("ja-JP", {{ notation: "compact", maximumFractionDigits: 2 }});
}};

function toggleItemSelection(conceptName) {{
  if (selectedConcepts.includes(conceptName)) {{
    if (selectedConcepts.length > 1) {{
      selectedConcepts = selectedConcepts.filter((concept) => concept !== conceptName);
    }}
  }} else {{
    selectedConcepts = [...selectedConcepts, conceptName];
  }}
  renderAll();
}}

function initHeader() {{
  const latestReport = data.selected_reports[data.selected_reports.length - 1];
  el("title").textContent = `${{data.ticker}} ${{data.name || ""}} 損益計算書`;
  el("scopeLabel").textContent = scopeText(latestReport.consolidation_scope);
  el("sourceLabel").textContent = sourceText(data.source);
  el("periodCount").textContent = data.periods.length;
  el("itemCount").textContent = data.items.length;
  el("latestDoc").textContent = latestReport.doc_id;
  el("periodStrip").style.setProperty("--period-count", data.periods.length);
  el("periodStrip").innerHTML = data.periods.map((period) => `<div class="period-cell">${{period}}</div>`).join("");
  el("tableHead").innerHTML = `<tr><th>項目</th>${{data.periods.map((period) => `<th>${{period}}</th>`).join("")}}</tr>`;
}}

function sparkline(values) {{
  const width = 92;
  const height = 45;
  const present = values.map((value, index) => [value, index]).filter(([value]) => value !== null && value !== undefined);
  if (present.length < 2) return `<svg class="spark" viewBox="0 0 ${{width}} ${{height}}" aria-hidden="true"></svg>`;
  const ys = present.map(([value]) => scaleValue(value));
  let min = Math.min(...ys);
  let max = Math.max(...ys);
  if (min === max) {{
    min -= 1;
    max += 1;
  }}
  const points = present.map(([value, index]) => {{
    const x = values.length === 1 ? width / 2 : (index / (values.length - 1)) * (width - 8) + 4;
    const y = height - 5 - ((scaleValue(value) - min) / (max - min)) * (height - 10);
    return `${{x.toFixed(2)}},${{y.toFixed(2)}}`;
  }});
  return `<svg class="spark" viewBox="0 0 ${{width}} ${{height}}" aria-hidden="true">
    <polyline points="${{points.join(" ")}}" fill="none" stroke="#55d6c2" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"></polyline>
  </svg>`;
}}

function renderItems() {{
  const query = el("search").value.trim().toLowerCase();
  const filtered = data.items.filter((item) => {{
    const haystack = `${{item.label}} ${{item.concept_name}}`.toLowerCase();
    return haystack.includes(query);
  }});
  if (!filtered.length) {{
    el("items").innerHTML = `<div class="empty">一致する項目がありません</div>`;
    return;
  }}
  el("items").innerHTML = filtered.map((item) => {{
    const selectedIndex = selectedIndexOf(item);
    const isSelected = selectedIndex !== -1;
    const color = isSelected ? seriesColor(selectedIndex) : "#34392f";
    return `
    <button class="item ${{isSelected ? "selected" : ""}}" type="button" data-concept="${{item.concept_name}}" aria-pressed="${{isSelected ? "true" : "false"}}" style="--series-color: ${{color}}">
      <span class="item-toggle" aria-hidden="true"></span>
      <span>
        <span class="item-name">${{item.label}}</span>
        <span class="item-code">${{item.concept_name}}</span>
        <span class="item-value">${{formatValue(item.latest_value)}}</span>
      </span>
      ${{sparkline(item.values)}}
    </button>
  `;
  }}).join("");
  document.querySelectorAll(".item").forEach((button) => {{
    button.addEventListener("click", () => {{
      toggleItemSelection(button.dataset.concept);
    }});
  }});
}}

function renderTable() {{
  const rows = data.items.map((item) => {{
    const isSelected = selectedConcepts.includes(item.concept_name);
    return `
    <tr class="${{isSelected ? "selected-row" : ""}}">
      <td><b>${{item.label}}</b><span>${{item.concept_name}}</span></td>
      ${{item.values.map((value) => `<td>${{compactValue(value)}}</td>`).join("")}}
    </tr>
  `;
  }}).join("");
  el("tableBody").innerHTML = rows;
}}

function setCanvasSize(canvas) {{
  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(rect.width * dpr));
  canvas.height = Math.max(1, Math.round(rect.height * dpr));
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  return {{ ctx, width: rect.width, height: rect.height }};
}}

function drawChart() {{
  const canvas = el("detailChart");
  const {{ ctx, width, height }} = setCanvasSize(canvas);
  ctx.clearRect(0, 0, width, height);
  const series = selectedItems();
  const compact = width < 560;
  const padding = {{
    left: compact ? 60 : 76,
    right: compact ? 14 : 24,
    top: 28,
    bottom: compact ? 46 : 54
  }};
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;
  const scaledSeries = series.map((item) => item.values.map(scaleValue));
  const present = scaledSeries.flat().filter((value) => value !== null && value !== undefined);
  if (!present.length) return;
  let min = Math.min(...present, 0);
  let max = Math.max(...present, 0);
  if (min === max) {{
    min -= 1;
    max += 1;
  }}
  const pad = (max - min) * 0.12;
  min -= pad;
  max += pad;
  const xAt = (index) => padding.left + (data.periods.length === 1 ? chartWidth / 2 : index / (data.periods.length - 1) * chartWidth);
  const yAt = (value) => padding.top + (max - value) / (max - min) * chartHeight;

  ctx.strokeStyle = canvasTheme.grid;
  ctx.lineWidth = 1;
  ctx.fillStyle = canvasTheme.text;
  ctx.font = `${{compact ? 11 : 12}}px Yu Gothic, Hiragino Kaku Gothic ProN, sans-serif`;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let i = 0; i <= 4; i++) {{
    const value = min + (max - min) * i / 4;
    const y = yAt(value);
    ctx.beginPath();
    ctx.moveTo(padding.left, y);
    ctx.lineTo(width - padding.right, y);
    ctx.stroke();
    ctx.fillText(value.toLocaleString("ja-JP", {{ maximumFractionDigits: 1 }}), padding.left - 10, y);
  }}

  const zeroY = yAt(0);
  if (zeroY > padding.top && zeroY < height - padding.bottom) {{
    ctx.strokeStyle = canvasTheme.zero;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(padding.left, zeroY);
    ctx.lineTo(width - padding.right, zeroY);
    ctx.stroke();
  }}

  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const visibleLabels = Math.max(2, Math.floor(chartWidth / (compact ? 76 : 64)));
  const labelStep = Math.max(1, Math.ceil(data.periods.length / visibleLabels));
  data.periods.forEach((period, index) => {{
    if (index !== 0 && index !== data.periods.length - 1 && index % labelStep !== 0) return;
    const x = xAt(index);
    ctx.fillStyle = canvasTheme.text;
    ctx.fillText(period, x, height - padding.bottom + 18);
  }});

  scaledSeries.forEach((values, seriesIndex) => {{
    ctx.strokeStyle = seriesColor(seriesIndex);
    ctx.lineWidth = 3;
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.beginPath();
    let started = false;
    values.forEach((value, index) => {{
      if (value === null || value === undefined) {{
        started = false;
        return;
      }}
      const x = xAt(index);
      const y = yAt(value);
      if (!started) {{
        ctx.moveTo(x, y);
        started = true;
      }} else {{
        ctx.lineTo(x, y);
      }}
    }});
    ctx.stroke();

    values.forEach((value, index) => {{
      if (value === null || value === undefined) return;
      const x = xAt(index);
      const y = yAt(value);
      ctx.fillStyle = canvasTheme.pointFill;
      ctx.strokeStyle = seriesColor(seriesIndex);
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(x, y, 4.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }});
  }});

  canvas.onmousemove = (event) => {{
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const nearest = data.periods
      .map((period, index) => ({{ period, index, dist: Math.abs(xAt(index) - x) }}))
      .sort((a, b) => a.dist - b.dist)[0];
    const entries = series
      .map((item, index) => ({{
        item,
        index,
        rawValue: item.values[nearest.index],
        scaledValue: scaleValue(item.values[nearest.index])
      }}))
      .filter((entry) => entry.rawValue !== null && entry.rawValue !== undefined);
    if (!entries.length) {{
      el("tooltip").style.display = "none";
      return;
    }}
    const tip = el("tooltip");
    const topY = Math.min(...entries.map((entry) => yAt(entry.scaledValue)));
    tip.style.display = "block";
    tip.style.left = `${{Math.max(8, Math.min(rect.width - 240, xAt(nearest.index) + 10))}}px`;
    tip.style.top = `${{Math.max(8, topY - 46)}}px`;
    tip.innerHTML = `<b>${{nearest.period}}</b>${{entries.map((entry) => `
      <div class="tooltip-row" style="--series-color: ${{seriesColor(entry.index)}}">
        <span class="series-dot"></span>
        <span>${{entry.item.label}}</span>
        <strong>${{formatValue(entry.rawValue)}}</strong>
      </div>
    `).join("")}}`;
  }};
  canvas.onmouseleave = () => {{
    el("tooltip").style.display = "none";
  }};
}}

function scheduleChartDraw() {{
  cancelAnimationFrame(chartResizeFrame);
  chartResizeFrame = requestAnimationFrame(drawChart);
}}

function renderSelected() {{
  const series = selectedItems();
  if (!series.length) return;
  el("selectedLabel").textContent = series.length === 1 ? series[0].label : `${{series[0].label}} + ${{series.length - 1}}項目`;
  el("selectedCode").textContent = series.map((item) => item.concept_name).join(" / ");
  el("legend").innerHTML = series.map((item, index) => `
    <span class="legend-chip" style="--series-color: ${{seriesColor(index)}}">
      <span class="series-dot"></span>
      <span>${{item.label}}</span>
    </span>
  `).join("");
  el("latestValues").innerHTML = series.map((item, index) => `
    <div class="latest-row" style="--series-color: ${{seriesColor(index)}}">
      <span class="series-dot"></span>
      <b>${{item.label}}</b>
      <strong>${{formatValue(item.latest_value)}}</strong>
    </div>
  `).join("");
  drawChart();
}}

function renderAll() {{
  renderItems();
  renderTable();
  renderSelected();
}}

document.querySelectorAll("[data-unit]").forEach((button) => {{
  button.addEventListener("click", () => {{
    unit = Number(button.dataset.unit);
    document.querySelectorAll("[data-unit]").forEach((item) => item.classList.toggle("active", item === button));
    renderAll();
  }});
}});
el("search").addEventListener("input", renderItems);
window.addEventListener("resize", scheduleChartDraw);
if ("ResizeObserver" in window) {{
  const chartResizeObserver = new ResizeObserver(scheduleChartDraw);
  chartResizeObserver.observe(el("detailChart"));
}}
initHeader();
renderAll();
</script>
</body>
</html>
"""


def open_html_with_playwright(
    html_path: Path,
    *,
    screenshot_path: Path | None = None,
    headed: bool = False,
    hold_ms: int = 0,
    browser_executable: Path | None = None,
    timeout_ms: int = 10_000,
    viewport_width: int = 1440,
    viewport_height: int = 1000,
) -> PlaywrightOpenResult:
    try:
        from playwright.sync_api import Error as PlaywrightError
        from playwright.sync_api import sync_playwright
    except ImportError as exc:
        raise RuntimeError("playwright is not installed; run `uv sync --group dev`") from exc

    if not html_path.is_file():
        raise RuntimeError(f"HTML not found: {html_path}")
    if browser_executable is not None and not browser_executable.is_file():
        raise RuntimeError(f"Playwright browser executable not found: {browser_executable}")

    launch_options: dict[str, object] = {"headless": not headed}
    if headed:
        launch_options["args"] = ["--start-maximized"]
    if browser_executable is not None:
        launch_options["executable_path"] = str(browser_executable)

    page_errors: list[str] = []
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(**launch_options)
            try:
                page_options: dict[str, object] = {}
                if headed:
                    page_options["no_viewport"] = True
                else:
                    page_options["viewport"] = {"width": viewport_width, "height": viewport_height}
                    page_options["device_scale_factor"] = 1
                page = browser.new_page(**page_options)
                page.on("pageerror", lambda exc: page_errors.append(str(exc)))
                page.goto(html_path.resolve().as_uri(), wait_until="load", timeout=timeout_ms)
                page.wait_for_selector("#detailChart", state="visible", timeout=timeout_ms)
                page.wait_for_function(
                    """
                    () => {
                      const canvas = document.querySelector("#detailChart");
                      const data = window.PL_TREND_DATA;
                      return Boolean(
                        data &&
                        Array.isArray(data.items) &&
                        data.items.length > 0 &&
                        canvas &&
                        canvas.width > 0 &&
                        canvas.height > 0
                      );
                    }
                    """,
                    timeout=timeout_ms,
                )
                if page_errors:
                    raise RuntimeError("Playwright page error: " + "; ".join(page_errors))

                title = page.locator("#title").inner_text(timeout=timeout_ms)
                item_count = page.locator(".item").count()
                canvas_stats = page.locator("#detailChart").evaluate(
                    """
                    (canvas) => {
                      const context = canvas.getContext("2d");
                      const image = context.getImageData(0, 0, canvas.width, canvas.height).data;
                      let drawn = 0;
                      for (let index = 3; index < image.length; index += 4) {
                        if (image[index] > 0) drawn += 1;
                      }
                      return { width: canvas.width, height: canvas.height, drawnPixels: drawn };
                    }
                    """
                )
                if item_count < 1:
                    raise RuntimeError("Playwright opened the HTML, but no PL item rows rendered")
                if int(canvas_stats["drawnPixels"]) < 50:
                    raise RuntimeError("Playwright opened the HTML, but the chart canvas is blank")

                saved_screenshot: str | None = None
                if screenshot_path is not None:
                    screenshot_path.parent.mkdir(parents=True, exist_ok=True)
                    page.screenshot(path=str(screenshot_path), full_page=True)
                    saved_screenshot = str(screenshot_path)

                if hold_ms > 0:
                    page.wait_for_timeout(hold_ms)

                return {
                    "title": title,
                    "item_count": item_count,
                    "canvas_width": int(canvas_stats["width"]),
                    "canvas_height": int(canvas_stats["height"]),
                    "drawn_pixels": int(canvas_stats["drawnPixels"]),
                    "screenshot_path": saved_screenshot,
                }
            finally:
                browser.close()
    except PlaywrightError as exc:
        raise RuntimeError(f"Playwright failed to open {html_path}: {exc}") from exc


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate a static HTML chart for XBRL profit/loss trends",
    )
    parser.add_argument("ticker", help="Ticker code to plot")
    parser.add_argument(
        "--db",
        type=Path,
        default=STOCKS_DB_PATH,
        help=f"Path to sqlite DB (default: {STOCKS_DB_PATH})",
    )
    parser.add_argument(
        "--periods",
        type=int,
        default=10,
        help="Number of latest fiscal periods to include (default: 10)",
    )
    parser.add_argument(
        "--source",
        default="edinet_xbrl",
        help="Data source code to read (default: edinet_xbrl)",
    )
    parser.add_argument(
        "--scope",
        choices=("auto", "consolidated", "non_consolidated"),
        default="auto",
        help="Consolidation scope to plot (default: auto)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="HTML output path (default: var/reports/TICKER_pl_trends.html)",
    )
    parser.add_argument(
        "--open-with-playwright",
        action="store_true",
        help="Open the generated HTML with Playwright and verify that the chart rendered",
    )
    parser.add_argument(
        "--playwright-screenshot",
        type=Path,
        help="Save a Playwright screenshot; implies --open-with-playwright",
    )
    parser.add_argument(
        "--playwright-headed",
        action="store_true",
        help="Run Playwright with a visible browser window",
    )
    parser.add_argument(
        "--playwright-hold-ms",
        type=int,
        default=0,
        help="Keep the Playwright page open this many milliseconds after verification",
    )
    parser.add_argument(
        "--playwright-browser-executable",
        type=Path,
        help="Chromium/Chrome executable path for Playwright",
    )
    parser.add_argument(
        "--playwright-timeout-ms",
        type=int,
        default=10_000,
        help="Playwright open/render timeout in milliseconds (default: 10000)",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.periods < 1:
        print("--periods must be >= 1", file=sys.stderr)
        return 2

    db_path: Path = args.db
    if not db_path.exists():
        print(f"DB not found: {db_path}", file=sys.stderr)
        return 2

    output_path: Path = args.output or Path("var") / "reports" / f"{args.ticker}_pl_trends.html"
    conn = _connect_readonly(db_path)
    try:
        payload = build_pl_trend_payload(
            conn,
            ticker=args.ticker,
            source=args.source,
            n_periods=args.periods,
            scope=args.scope,
        )
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    finally:
        conn.close()

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(render_html(payload), encoding="utf-8")
    print(f"Wrote {output_path}")
    print(
        f"{payload['ticker']} {payload['name']}: "
        f"{len(payload['periods'])} periods, {len(payload['items'])} PL items"
    )
    if args.open_with_playwright or args.playwright_screenshot is not None:
        if args.playwright_timeout_ms < 1:
            print("--playwright-timeout-ms must be >= 1", file=sys.stderr)
            return 2
        if args.playwright_hold_ms < 0:
            print("--playwright-hold-ms must be >= 0", file=sys.stderr)
            return 2
        try:
            playwright_result = open_html_with_playwright(
                output_path,
                screenshot_path=args.playwright_screenshot,
                headed=args.playwright_headed,
                hold_ms=args.playwright_hold_ms,
                browser_executable=args.playwright_browser_executable,
                timeout_ms=args.playwright_timeout_ms,
            )
        except RuntimeError as exc:
            print(str(exc), file=sys.stderr)
            return 1
        print(
            "Playwright opened "
            f"{output_path} ({playwright_result['item_count']} items, "
            f"{playwright_result['drawn_pixels']} drawn canvas pixels)"
        )
        if playwright_result["screenshot_path"] is not None:
            print(f"Playwright screenshot: {playwright_result['screenshot_path']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
