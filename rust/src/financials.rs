use std::collections::HashMap;

use crate::InventoriesTagMismatchError;
use crate::inventory;
use crate::share_classes;
use crate::types::{ConceptKey, LoadedXbrlArtifact};
use crate::xml_util;

type ItemCandidates = &'static [(&'static str, &'static [&'static str])];

const BS_ITEM_CANDIDATES: ItemCandidates = &[
    ("cash_and_deposits", &["CashAndDeposits"]),
    ("current_assets", &["CurrentAssets"]),
    ("fixed_assets", &["NoncurrentAssets", "FixedAssets"]),
    (
        "tangible_fixed_assets",
        &[
            "PropertyPlantAndEquipment",
            "TangibleAssets",
            "TangibleFixedAssets",
        ],
    ),
    (
        "intangible_fixed_assets",
        &["IntangibleAssets", "IntangibleFixedAssets"],
    ),
    ("investment_securities", &["InvestmentSecurities"]),
    (
        "trade_receivables",
        &[
            "NotesAndAccountsReceivableTrade",
            "AccountsReceivableTrade",
            "NotesAndAccountsReceivableTradeAndContractAssets",
            "TradeAndOtherReceivables",
        ],
    ),
    ("current_liabilities", &["CurrentLiabilities"]),
    (
        "trade_payables",
        &[
            "NotesAndAccountsPayableTrade",
            "AccountsPayableTrade",
            "TradeAndOtherPayables",
        ],
    ),
    ("non_current_liabilities", &["NoncurrentLiabilities"]),
    (
        "short_term_debt",
        &[
            "ShortTermLoansPayable",
            "ShortTermBorrowings",
            "CurrentPortionOfLongTermLoansPayable",
            "CurrentPortionOfLongTermBorrowingsCLIFRS",
            "BorrowingsCLIFRS",
            "BondsAndBorrowingsCLIFRS",
        ],
    ),
    (
        "long_term_debt",
        &[
            "LongTermLoansPayable",
            "LongTermBorrowings",
            "BorrowingsNCLIFRS",
            "BondsAndBorrowingsNCLIFRS",
        ],
    ),
    ("net_assets", &["NetAssets", "Equity"]),
    (
        "stockholders_equity",
        &[
            "ShareholdersEquity",
            "EquityAttributableToOwnersOfParent",
            "OwnersEquity",
        ],
    ),
    ("total_assets", &["Assets"]),
    ("total_equity", &["NetAssets", "Equity"]),
];

const PL_ITEM_CANDIDATES: ItemCandidates = &[
    (
        "revenue",
        &[
            "NetSales",
            "Revenue",
            "RevenuesFromExternalCustomers",
            "NetSalesSummaryOfBusinessResults",
        ],
    ),
    ("cost_of_revenue", &["CostOfSales", "CostOfRevenue"]),
    ("operating_income", &["OperatingIncome", "OperatingProfit"]),
    (
        "ordinary_income",
        &[
            "OrdinaryIncome",
            "OrdinaryProfit",
            "OrdinaryIncomeLossSummaryOfBusinessResults",
        ],
    ),
    (
        "net_income",
        &[
            "ProfitLossAttributableToOwnersOfParent",
            "ProfitLoss",
            "NetIncome",
            "ProfitLossAttributableToOwnersOfParentSummaryOfBusinessResults",
        ],
    ),
];

const CF_ITEM_CANDIDATES: ItemCandidates = &[
    (
        "cash_equivalents",
        &[
            "CashAndCashEquivalents",
            "CashAndCashEquivalentsSummaryOfBusinessResults",
            "CashAndCashEquivalentsIFRS",
            "CashAndCashEquivalentsIFRSSummaryOfBusinessResults",
        ],
    ),
    (
        "operating_cf",
        &[
            "NetCashProvidedByUsedInOperatingActivities",
            "NetCashProvidedByUsedInOperatingActivitiesSummaryOfBusinessResults",
            "NetCashProvidedByUsedInOperatingActivitiesIFRS",
            "CashFlowsFromUsedInOperatingActivitiesIFRSSummaryOfBusinessResults",
        ],
    ),
    (
        "investing_cf",
        &[
            "NetCashProvidedByUsedInInvestmentActivities",
            "NetCashProvidedByUsedInInvestmentActivitiesSummaryOfBusinessResults",
            "NetCashProvidedByUsedInInvestingActivitiesIFRS",
            "CashFlowsFromUsedInInvestingActivitiesIFRSSummaryOfBusinessResults",
        ],
    ),
    (
        "financing_cf",
        &[
            "NetCashProvidedByUsedInFinancingActivities",
            "NetCashProvidedByUsedInFinancingActivitiesSummaryOfBusinessResults",
            "NetCashProvidedByUsedInFinancingActivitiesIFRS",
            "CashFlowsFromUsedInFinancingActivitiesIFRSSummaryOfBusinessResults",
        ],
    ),
];

const DIVIDEND_ITEM_CANDIDATES: ItemCandidates = &[
    (
        "dps",
        &[
            "DividendPaidPerShareSummaryOfBusinessResults",
            "AnnualDividendPerShareSummaryOfBusinessResults",
            "DividendPerShareDividendsOfSurplus",
        ],
    ),
    (
        "dividend_payment",
        &["DividendsFromSurplus", "CashDividendsPaidFinCF"],
    ),
];

const SHARES_ITEM_CANDIDATES: ItemCandidates =
    &[("shares_outstanding", xml_util::SHARES_OUTSTANDING_TAGS)];

const FORECAST_ITEM_CANDIDATES: ItemCandidates = &[
    (
        "revenue",
        &[
            "ForecastNetSalesSummaryOfBusinessResults",
            "ForecastRevenueSummaryOfBusinessResults",
        ],
    ),
    (
        "operating_income",
        &[
            "ForecastOperatingIncomeSummaryOfBusinessResults",
            "ForecastOperatingProfitSummaryOfBusinessResults",
        ],
    ),
    (
        "ordinary_income",
        &[
            "ForecastOrdinaryIncomeSummaryOfBusinessResults",
            "ForecastOrdinaryProfitSummaryOfBusinessResults",
        ],
    ),
    (
        "net_income",
        &[
            "ForecastProfitLossAttributableToOwnersOfParentSummaryOfBusinessResults",
            "ForecastProfitLossSummaryOfBusinessResults",
            "ForecastNetIncomeSummaryOfBusinessResults",
        ],
    ),
];

fn first_value(
    period_facts: &HashMap<ConceptKey, Option<f64>>,
    candidates: &[&str],
) -> Option<f64> {
    for name in candidates {
        for (concept, value) in period_facts {
            if concept.1 == *name {
                if let Some(v) = value {
                    return Some(*v);
                }
            }
        }
    }
    None
}

fn build_statement_items(
    period_facts: &HashMap<ConceptKey, Option<f64>>,
    candidates: ItemCandidates,
) -> HashMap<String, Option<f64>> {
    let mut result: HashMap<String, Option<f64>> = HashMap::new();
    for (item_name, concept_names) in candidates {
        if let Some(value) = first_value(period_facts, concept_names) {
            result.insert(item_name.to_string(), Some(value));
        }
    }
    result
}

fn build_statement_items_with_fallback(
    primary_facts: Option<&HashMap<ConceptKey, Option<f64>>>,
    fallback_facts: Option<&HashMap<ConceptKey, Option<f64>>>,
    candidates: ItemCandidates,
) -> HashMap<String, Option<f64>> {
    let primary_items = primary_facts
        .map(|facts| build_statement_items(facts, candidates))
        .unwrap_or_default();
    if !primary_items.is_empty() {
        return primary_items;
    }
    fallback_facts
        .map(|facts| build_statement_items(facts, candidates))
        .unwrap_or_default()
}

/// Parse financial statements from a loaded XBRL artifact.
///
/// Mirrors the Python `parse_xbrl_financials` function.
pub fn parse_financials_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Result<
    HashMap<String, HashMap<String, HashMap<String, Option<f64>>>>,
    InventoriesTagMismatchError,
> {
    let inventory_by_period = inventory::parse_inventories_from_artifact(artifact)?;
    let share_class_rows = share_classes::parse_share_classes_from_artifact(artifact);
    let preferred_flags = share_classes::preferred_flags_by_period(&share_class_rows);

    let mut periods: Vec<String> = artifact
        .financial_facts
        .keys()
        .chain(artifact.non_consolidated_financial_facts.keys())
        .cloned()
        .collect();
    periods.sort();
    periods.dedup();
    periods.reverse();

    let mut result: HashMap<String, HashMap<String, HashMap<String, Option<f64>>>> = HashMap::new();

    for period in &periods {
        let period_facts = artifact.financial_facts.get(period);
        let non_consolidated_facts = artifact.non_consolidated_financial_facts.get(period);

        // BS items
        let mut bs_items = build_statement_items_with_fallback(
            period_facts,
            non_consolidated_facts,
            BS_ITEM_CANDIDATES,
        );
        if let Some(inv) = inventory_by_period
            .get(period)
            .and_then(|m| m.get("inventories"))
            .copied()
            .flatten()
        {
            bs_items.insert("inventories".to_string(), Some(inv));
        }
        if let Some(ncl) = bs_items.get("non_current_liabilities").copied().flatten() {
            if !bs_items.contains_key("non_current_liabilities_total") {
                bs_items.insert("non_current_liabilities_total".to_string(), Some(ncl));
            }
        }
        if let Some(na) = bs_items.get("net_assets").copied().flatten() {
            if !bs_items.contains_key("total_equity") {
                bs_items.insert("total_equity".to_string(), Some(na));
            }
        }
        if let Some(te) = bs_items.get("total_equity").copied().flatten() {
            if !bs_items.contains_key("net_assets") {
                bs_items.insert("net_assets".to_string(), Some(te));
            }
        }
        if !bs_items.contains_key("stockholders_equity") && bs_items.contains_key("total_equity") {
            if let Some(te) = bs_items.get("total_equity").copied().flatten() {
                bs_items.insert("stockholders_equity".to_string(), Some(te));
            }
        }

        // Shares outstanding from shares-denominated facts.
        // Shares facts use FilingDateInstant context (e.g. 2025-09) while
        // financial periods use fiscal-year-end (e.g. 2025-06), so we search
        // ALL shares_facts periods rather than doing an exact key lookup.
        let shares_period_facts = artifact.shares_facts.values().find(|facts| {
            facts.keys().any(|(_, local)| {
                SHARES_ITEM_CANDIDATES
                    .iter()
                    .any(|(_, candidates)| candidates.contains(&local.as_str()))
            })
        });
        let shares_items =
            build_statement_items_with_fallback(shares_period_facts, None, SHARES_ITEM_CANDIDATES);
        for (name, value) in shares_items {
            bs_items.insert(name, value);
        }
        let has_preferred = preferred_flags.get(period).copied().unwrap_or(false);
        bs_items.insert(
            "has_preferred_shares".to_string(),
            Some(if has_preferred { 1.0 } else { 0.0 }),
        );

        let mut statements: HashMap<String, HashMap<String, Option<f64>>> = HashMap::new();
        if !bs_items.is_empty() {
            statements.insert("bs".to_string(), bs_items);
        }

        let pl_items = build_statement_items_with_fallback(
            period_facts,
            non_consolidated_facts,
            PL_ITEM_CANDIDATES,
        );
        if !pl_items.is_empty() {
            statements.insert("pl".to_string(), pl_items);
        }

        let cf_items = build_statement_items_with_fallback(
            period_facts,
            non_consolidated_facts,
            CF_ITEM_CANDIDATES,
        );
        if !cf_items.is_empty() {
            statements.insert("cf".to_string(), cf_items);
        }

        // Dividend — try consolidated first, then non-consolidated for dps
        let mut dividend_items = period_facts
            .map(|facts| build_statement_items(facts, DIVIDEND_ITEM_CANDIDATES))
            .unwrap_or_default();
        if !dividend_items.contains_key("dps") {
            if let Some(nc_facts) = artifact.non_consolidated_facts.get(period) {
                if let Some(v) = first_value(nc_facts, DIVIDEND_ITEM_CANDIDATES[0].1) {
                    dividend_items.insert("dps".to_string(), Some(v));
                }
            }
        }
        if !dividend_items.is_empty() {
            statements.insert("dividend".to_string(), dividend_items);
        }

        // Forecast
        let forecast_items = build_statement_items_with_fallback(
            period_facts,
            non_consolidated_facts,
            FORECAST_ITEM_CANDIDATES,
        );
        if !forecast_items.is_empty() {
            statements.insert("forecast".to_string(), forecast_items);
        }

        if !statements.is_empty() {
            result.insert(period.clone(), statements);
        }
    }

    Ok(result)
}
