from __future__ import annotations

import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

from stock_db.sources.edinet.xbrl_bs_parser import (
    _IGNORED_FACT_NAMESPACES,
    _IX_NS,
    _XBRLDI_NS,
    _XBRLI_NS,
    _collect_documents,
    _is_legacy_xbrl_dir,
    _is_nil,
    _legacy_target_file,
    _local_name,
    _namespace_uri,
    _parse_units,
    _parse_xbrl_value,
    _resolve_qname,
    _store_concept_value,
    parse_xbrl_bs,
)

ConceptKey = tuple[str, str]

_BS_ITEM_CANDIDATES: dict[str, tuple[str, ...]] = {
    "cash_and_deposits": ("CashAndDeposits",),
    "current_assets": ("CurrentAssets",),
    "fixed_assets": ("NoncurrentAssets", "FixedAssets"),
    "tangible_fixed_assets": ("PropertyPlantAndEquipment", "TangibleAssets", "TangibleFixedAssets"),
    "intangible_fixed_assets": ("IntangibleAssets", "IntangibleFixedAssets"),
    "investment_securities": ("InvestmentSecurities",),
    "trade_receivables": (
        "NotesAndAccountsReceivableTrade",
        "AccountsReceivableTrade",
        "NotesAndAccountsReceivableTradeAndContractAssets",
        "TradeAndOtherReceivables",
    ),
    "current_liabilities": ("CurrentLiabilities",),
    "trade_payables": (
        "NotesAndAccountsPayableTrade",
        "AccountsPayableTrade",
        "TradeAndOtherPayables",
    ),
    "non_current_liabilities": ("NoncurrentLiabilities",),
    "short_term_debt": ("ShortTermLoansPayable", "ShortTermBorrowings"),
    "long_term_debt": ("LongTermLoansPayable", "LongTermBorrowings"),
    "net_assets": ("NetAssets", "Equity"),
    "stockholders_equity": (
        "ShareholdersEquity",
        "EquityAttributableToOwnersOfParent",
        "OwnersEquity",
    ),
    "total_assets": ("Assets",),
    "total_equity": ("NetAssets", "Equity"),
}

_PL_ITEM_CANDIDATES: dict[str, tuple[str, ...]] = {
    "revenue": ("NetSales", "Revenue", "RevenuesFromExternalCustomers", "NetSalesSummaryOfBusinessResults"),
    "cost_of_revenue": ("CostOfSales", "CostOfRevenue"),
    "operating_income": ("OperatingIncome", "OperatingProfit"),
    "ordinary_income": (
        "OrdinaryIncome",
        "OrdinaryProfit",
        "OrdinaryIncomeLossSummaryOfBusinessResults",
    ),
    "net_income": (
        "ProfitLossAttributableToOwnersOfParent",
        "ProfitLoss",
        "NetIncome",
        "ProfitLossAttributableToOwnersOfParentSummaryOfBusinessResults",
    ),
}

_CF_ITEM_CANDIDATES: dict[str, tuple[str, ...]] = {
    "cash_equivalents": ("CashAndCashEquivalents", "CashAndCashEquivalentsSummaryOfBusinessResults"),
    "operating_cf": (
        "NetCashProvidedByUsedInOperatingActivities",
        "NetCashProvidedByUsedInOperatingActivitiesSummaryOfBusinessResults",
    ),
    "investing_cf": (
        "NetCashProvidedByUsedInInvestmentActivities",
        "NetCashProvidedByUsedInInvestingActivitiesSummaryOfBusinessResults",
    ),
    "financing_cf": (
        "NetCashProvidedByUsedInFinancingActivities",
        "NetCashProvidedByUsedInFinancingActivitiesSummaryOfBusinessResults",
    ),
}

_DIVIDEND_ITEM_CANDIDATES: dict[str, tuple[str, ...]] = {
    "dps": (
        "DividendPaidPerShareSummaryOfBusinessResults",
        "AnnualDividendPerShareSummaryOfBusinessResults",
        "DividendPerShareDividendsOfSurplus",
    ),
    "dividend_payment": ("DividendsFromSurplus", "CashDividendsPaidFinCF"),
}

_FORECAST_ITEM_CANDIDATES: dict[str, tuple[str, ...]] = {
    "revenue": ("ForecastNetSalesSummaryOfBusinessResults", "ForecastRevenueSummaryOfBusinessResults"),
    "operating_income": (
        "ForecastOperatingIncomeSummaryOfBusinessResults",
        "ForecastOperatingProfitSummaryOfBusinessResults",
    ),
    "ordinary_income": (
        "ForecastOrdinaryIncomeSummaryOfBusinessResults",
        "ForecastOrdinaryProfitSummaryOfBusinessResults",
    ),
    "net_income": (
        "ForecastProfitLossAttributableToOwnersOfParentSummaryOfBusinessResults",
        "ForecastProfitLossSummaryOfBusinessResults",
        "ForecastNetIncomeSummaryOfBusinessResults",
    ),
}


@dataclass(frozen=True, slots=True)
class _ContextInfo:
    period: str | None
    is_instant: bool
    has_dimensions: bool
    is_non_consolidated: bool
    dimension_count: int


def _period_from_duration(end_date: str | None) -> str | None:
    if not end_date or len(end_date) < 7:
        return None
    return end_date[:7]


def _parse_contexts(root: ET.Element) -> dict[str, _ContextInfo]:
    contexts: dict[str, _ContextInfo] = {}
    for elem in root.iter():
        if _namespace_uri(elem.tag) != _XBRLI_NS or _local_name(elem.tag) != "context":
            continue
        context_id = elem.attrib.get("id")
        if not context_id:
            continue

        instant_text: str | None = None
        end_date_text: str | None = None
        has_dimensions = False
        is_non_consolidated = False
        dimension_count = 0

        for child in elem.iter():
            namespace = _namespace_uri(child.tag)
            local_name = _local_name(child.tag)
            if namespace == _XBRLI_NS and local_name == "instant":
                instant_text = (child.text or "").strip()
                continue
            if namespace == _XBRLI_NS and local_name == "endDate":
                end_date_text = (child.text or "").strip()
                continue
            if namespace != _XBRLDI_NS:
                continue
            if local_name == "explicitMember":
                has_dimensions = True
                dimension_count += 1
                dimension_name = child.attrib.get("dimension", "").split(":")[-1]
                member_name = (child.text or "").strip().split(":")[-1]
                if (
                    member_name == "NonConsolidatedMember"
                    or dimension_name == "ConsolidatedOrNonConsolidatedAxis"
                    and member_name == "NonConsolidatedMember"
                ):
                    is_non_consolidated = True
            elif local_name == "typedMember":
                has_dimensions = True
                dimension_count += 1

        if instant_text:
            period = instant_text[:7]
            is_instant = True
        else:
            period = _period_from_duration(end_date_text)
            is_instant = False

        contexts[context_id] = _ContextInfo(
            period=period,
            is_instant=is_instant,
            has_dimensions=has_dimensions,
            is_non_consolidated=is_non_consolidated,
            dimension_count=dimension_count,
        )
    return contexts


def _should_use_context(context: _ContextInfo | None) -> bool:
    return bool(
        context is not None
        and context.period is not None
        and not context.has_dimensions
        and not context.is_non_consolidated
    )


def _should_use_non_consolidated_dividend_context(context: _ContextInfo | None) -> bool:
    return bool(
        context is not None
        and context.period is not None
        and context.is_non_consolidated
        and context.dimension_count == 1
    )


def _extract_inline_facts(
    document,
    contexts: dict[str, _ContextInfo],
    units: dict[str, bool],
    facts: dict[str, dict[ConceptKey, float | None]],
    periods_seen: set[str],
    predicate,
) -> None:
    for elem in document.root.iter():
        if _namespace_uri(elem.tag) != _IX_NS or _local_name(elem.tag) != "nonfraction":
            continue
        context_id = elem.attrib.get("contextRef") or elem.attrib.get("contextref")
        context = contexts.get(context_id or "")
        if not predicate(context):
            continue
        unit_id = elem.attrib.get("unitRef") or elem.attrib.get("unitref")
        if unit_id is None or not units.get(unit_id, False):
            continue
        concept = _resolve_qname(elem.attrib.get("name", ""), document.nsmap)
        if concept is None:
            continue
        periods_seen.add(context.period)
        raw_value = "" if _is_nil(elem) else "".join(elem.itertext()).strip()
        value = _parse_xbrl_value(
            raw_value,
            elem.attrib.get("decimals", ""),
            elem.attrib.get("scale", ""),
            elem.attrib.get("sign", ""),
        )
        _store_concept_value(facts, context.period, concept, value)


def _extract_instance_facts(
    document,
    contexts: dict[str, _ContextInfo],
    units: dict[str, bool],
    facts: dict[str, dict[ConceptKey, float | None]],
    periods_seen: set[str],
    predicate,
) -> None:
    for elem in document.root.iter():
        namespace = _namespace_uri(elem.tag)
        if namespace in _IGNORED_FACT_NAMESPACES:
            continue
        context_id = elem.attrib.get("contextRef")
        if context_id is None:
            continue
        context = contexts.get(context_id)
        if not predicate(context):
            continue
        unit_id = elem.attrib.get("unitRef")
        if unit_id is None or not units.get(unit_id, False):
            continue
        concept = _resolve_qname(elem.tag, document.nsmap)
        if concept is None:
            continue
        periods_seen.add(context.period)
        raw_value = "" if _is_nil(elem) else "".join(elem.itertext()).strip()
        value = _parse_xbrl_value(
            raw_value,
            elem.attrib.get("decimals", ""),
            elem.attrib.get("scale", ""),
            elem.attrib.get("sign", ""),
        )
        _store_concept_value(facts, context.period, concept, value)


def _parse_numeric_facts(
    artifact_path: Path,
    *,
    predicate=_should_use_context,
) -> dict[str, dict[ConceptKey, float | None]]:
    if artifact_path.is_file():
        document = Path(artifact_path)
        documents = []
        from stock_db.sources.edinet.xbrl_bs_parser import _parse_xml_document

        parsed = _parse_xml_document(document)
        if parsed is None:
            return {}
        documents = [parsed]
    elif artifact_path.is_dir():
        if _is_legacy_xbrl_dir(artifact_path):
            target = _legacy_target_file(artifact_path)
            if target is None:
                return {}
            return _parse_numeric_facts(target)
        documents = _collect_documents(artifact_path)
    else:
        return {}

    if not documents:
        return {}

    contexts: dict[str, _ContextInfo] = {}
    units: dict[str, bool] = {}
    for document in documents:
        contexts.update(_parse_contexts(document.root))
        units.update(_parse_units(document.root))

    facts: dict[str, dict[ConceptKey, float | None]] = {}
    periods_seen: set[str] = set()
    for document in documents:
        _extract_inline_facts(document, contexts, units, facts, periods_seen, predicate)
        _extract_instance_facts(document, contexts, units, facts, periods_seen, predicate)
    return {period: facts.get(period, {}) for period in periods_seen}


def _first_value(
    period_facts: dict[ConceptKey, float | None],
    candidates: tuple[str, ...],
) -> float | None:
    for name in candidates:
        for concept, value in period_facts.items():
            if concept[1] == name and value is not None:
                return value
    return None


def _build_statement_items(
    period_facts: dict[ConceptKey, float | None],
    candidates: dict[str, tuple[str, ...]],
) -> dict[str, float | None]:
    result: dict[str, float | None] = {}
    for item_name, concept_names in candidates.items():
        value = _first_value(period_facts, concept_names)
        if value is not None:
            result[item_name] = value
    return result


def parse_xbrl_financials(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
    artifact_path = Path(xbrl_path)
    facts_by_period = _parse_numeric_facts(artifact_path)
    if not facts_by_period:
        return {}
    non_consolidated_facts = _parse_numeric_facts(
        artifact_path,
        predicate=_should_use_non_consolidated_dividend_context,
    )

    inventory_by_period = parse_xbrl_bs(xbrl_path)
    result: dict[str, dict[str, dict[str, float | None]]] = {}
    for period in sorted(facts_by_period, reverse=True):
        period_facts = facts_by_period[period]
        bs_items = _build_statement_items(period_facts, _BS_ITEM_CANDIDATES)
        inventories = inventory_by_period.get(period, {}).get("inventories")
        if inventories is not None:
            bs_items["inventories"] = inventories
        if "non_current_liabilities" in bs_items and "non_current_liabilities_total" not in bs_items:
            bs_items["non_current_liabilities_total"] = bs_items["non_current_liabilities"]
        if "net_assets" in bs_items and "total_equity" not in bs_items:
            bs_items["total_equity"] = bs_items["net_assets"]
        if "total_equity" in bs_items and "net_assets" not in bs_items:
            bs_items["net_assets"] = bs_items["total_equity"]
        if "stockholders_equity" not in bs_items and "total_equity" in bs_items:
            bs_items["stockholders_equity"] = bs_items["total_equity"]

        statements: dict[str, dict[str, float | None]] = {}
        if bs_items:
            statements["bs"] = bs_items

        pl_items = _build_statement_items(period_facts, _PL_ITEM_CANDIDATES)
        if pl_items:
            statements["pl"] = pl_items

        cf_items = _build_statement_items(period_facts, _CF_ITEM_CANDIDATES)
        if cf_items:
            statements["cf"] = cf_items

        dividend_items = _build_statement_items(period_facts, _DIVIDEND_ITEM_CANDIDATES)
        if "dps" not in dividend_items:
            dividend_items.update(
                _build_statement_items(
                    non_consolidated_facts.get(period, {}),
                    {"dps": _DIVIDEND_ITEM_CANDIDATES["dps"]},
                )
            )
        if dividend_items:
            statements["dividend"] = dividend_items

        forecast_items = _build_statement_items(period_facts, _FORECAST_ITEM_CANDIDATES)
        if forecast_items:
            statements["forecast"] = forecast_items

        if statements:
            result[period] = statements

    return result
