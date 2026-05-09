from __future__ import annotations

from stock_db.sources.edinet.xbrl_bs_parser import (
    load_xbrl_artifact,
    parse_xbrl_bs_loaded,
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
    artifact = load_xbrl_artifact(xbrl_path)
    if artifact is None:
        return {}

    inventory_by_period = parse_xbrl_bs_loaded(artifact)
    result: dict[str, dict[str, dict[str, float | None]]] = {}
    for period in sorted(artifact.financial_facts, reverse=True):
        period_facts = artifact.financial_facts[period]
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
                    artifact.non_consolidated_facts.get(period, {}),
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
