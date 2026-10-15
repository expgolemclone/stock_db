use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Active reviewed EDINET J-GAAP same-meaning aliases added on 2026-06-17.
///
/// This layer folds 100 legacy, industry-suffixed, or statement-role-suffixed
/// local names only when the wording preserves the same balance-sheet,
/// profit-loss, or cash-flow subtotal meaning as an existing canonical field.
/// Component, extension, and issuer-specific names still need explicit review
/// before being collapsed.
fn reviewed_jppfs_alias_active_20260617(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        "CashAndDepositsBNK" | "CashAndDepositsINS" | "CashAndDepositsSEC"
        | "CashAndDepositsGAS" | "CashAndDepositsELE" | "CashAndDepositsRWY"
        | "CashAndDepositsWAT" | "CashAndDepositsCMD" => "cash_and_deposits",

        "CashAndCashEquivalentsBNK"
        | "CashAndCashEquivalentsINS"
        | "CashAndCashEquivalentsGAS"
        | "CashAndCashEquivalentsELE"
        | "CashAndCashEquivalentsRWY"
        | "CashAndCashEquivalentsSPF" => "cash_and_cash_equivalents",

        "CurrentAssetsBNK" | "CurrentAssetsINS" | "CurrentAssetsSEC" | "CurrentAssetsELE"
        | "CurrentAssetsGAS" => "current_assets",

        "NoncurrentAssetsBNK"
        | "NoncurrentAssetsINS"
        | "NoncurrentAssetsSEC"
        | "NoncurrentAssetsELE"
        | "NoncurrentAssetsGAS" => "non_current_assets",

        "CurrentLiabilitiesBNK"
        | "CurrentLiabilitiesINS"
        | "CurrentLiabilitiesSEC"
        | "CurrentLiabilitiesELE"
        | "CurrentLiabilitiesGAS" => "current_liabilities",

        "NoncurrentLiabilitiesBNK"
        | "NoncurrentLiabilitiesINS"
        | "NoncurrentLiabilitiesSEC"
        | "NoncurrentLiabilitiesELE"
        | "NoncurrentLiabilitiesGAS" => "non_current_liabilities",

        "LiabilitiesAndNetAssetsBNK" | "LiabilitiesAndNetAssetsINS" => {
            "total_liabilities_and_net_assets"
        }

        "EquityBNK" | "EquityINS" => "total_equity",
        "CapitalStockBNK" => "capital_stock",
        "RetainedEarningsBNK" => "retained_earnings",

        "NetSalesCNS"
        | "NetSalesGAS"
        | "NetSalesELE"
        | "NetSalesRWY"
        | "NetSalesWAT"
        | "NetSalesCMD"
        | "NetSalesSEC"
        | "NetSalesSPF"
        | "OperatingRevenueGAS"
        | "OperatingRevenueSEC"
        | "OperatingRevenueINS"
        | "OperatingRevenueSPF" => "revenue",

        "CostOfSalesCNS" | "CostOfSalesGAS" | "CostOfSalesELE" | "CostOfSalesRWY"
        | "CostOfSalesWAT" | "CostOfSalesCMD" | "CostOfSalesSEC" | "CostOfSalesSPF" => {
            "cost_of_revenue"
        }

        "GrossProfitCNS" | "GrossProfitGAS" | "GrossProfitELE" | "GrossProfitRWY"
        | "GrossProfitWAT" => "gross_profit",

        "SellingGeneralAndAdministrativeExpensesCNS"
        | "SellingGeneralAndAdministrativeExpensesCMD"
        | "SellingGeneralAndAdministrativeExpensesSEC"
        | "SellingGeneralAndAdministrativeExpensesSPF" => "selling_general_admin",

        "OperatingIncomeBNK" | "OperatingIncomeCNS" | "OperatingIncomeGAS"
        | "OperatingIncomeELE" | "OperatingIncomeWAT" => "operating_income",

        // EDINET industry tags use OrdinaryIncome{BNK,GAS,ELE} for 経常収益
        // (ordinary income/revenues), whereas generic OrdinaryIncome is 経常利益.
        // Keep these top-line industry tags in revenue instead of ordinary_income.
        "OrdinaryIncomeBNK" | "OrdinaryIncomeGAS" | "OrdinaryIncomeELE" => "revenue",

        "ProfitLossAttributableToOwnersOfParentBNK"
        | "ProfitLossAttributableToOwnersOfParentGAS" => "net_income_parent",

        "NetIncomeLossBNK" => "net_income",

        "NetCashProvidedByUsedInOperatingActivitiesOpeCF"
        | "NetCashProvidedByUsedInOperatingActivitiesBNK"
        | "NetCashProvidedByUsedInOperatingActivitiesGAS"
        | "CashFlowsFromUsedInOperatingActivitiesBNK" => "operating_cf",

        "NetCashProvidedByUsedInInvestmentActivitiesBNK"
        | "NetCashProvidedByUsedInInvestmentActivitiesGAS"
        | "NetCashProvidedByUsedInInvestingActivitiesSEC"
        | "CashFlowsFromUsedInInvestingActivitiesBNK" => "investing_cf",

        "NetCashProvidedByUsedInFinancingActivitiesBNK"
        | "NetCashProvidedByUsedInFinancingActivitiesGAS"
        | "CashFlowsFromUsedInFinancingActivitiesBNK"
        | "CashFlowsFromUsedInFinancingActivitiesSEC" => "financing_cf",

        "PurchaseOfPropertyPlantAndEquipmentBNK" | "PurchaseOfPropertyPlantAndEquipmentGAS" => {
            "capex"
        }

        "PurchaseOfIntangibleAssetsBNK" | "PurchaseOfIntangibleAssetsGAS" => {
            "intangible_asset_purchases"
        }

        "ProceedsFromSalesOfPropertyPlantAndEquipmentBNK"
        | "ProceedsFromSalesOfPropertyPlantAndEquipmentGAS" => "ppe_sold",

        "ProceedsFromSalesOfInvestmentSecuritiesBNK" => "investment_securities_sold",
        "PurchaseOfInvestmentSecuritiesBNK" => "investment_securities_purchased",

        _ => return None,
    };
    Some(field)
}

fn is_edinet_jppfs_primary_statement_namespace(uri: &str) -> bool {
    [
        "/taxonomy/jppfs/",
        "/taxonomy/jplvh/",
        "/taxonomy/jpsps/",
        "/taxonomy/jpctl/",
    ]
    .iter()
    .any(|family| uri.contains(family))
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_edinet_jppfs_primary_statement_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_alias_active_20260617(&concept.1)?;
    Some(CanonicalConcept {
        item_name: field.to_string(),
        priority: reviewed_priority(&concept.1),
        match_kind: MatchKind::TaxonomyFamilyAlias,
    })
}

fn reviewed_priority(local_name: &str) -> u8 {
    if local_name.contains("SummaryOfBusinessResults")
        || local_name.contains("SummaryOfBusinessResult")
    {
        5
    } else if local_name.contains("IFRS") {
        20
    } else {
        10
    }
}

pub fn canonicalize_semantic_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    reviewed_alias(concept).or_else(|| base::canonicalize_semantic_concept(concept))
}

pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    reviewed_alias(concept).or_else(|| base::canonicalize_concept(concept))
}

pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(alias) = reviewed_alias(concept) {
        return TaxonomyConceptStatus::Semantic(alias);
    }
    base::concept_status(concept)
}

#[cfg(test)]
pub fn is_excluded_concept(local_name: &str) -> bool {
    reviewed_jppfs_alias_active_20260617(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

pub fn statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    canonicalize_concept(concept)
        .and_then(|canonical| statement_for_item_name(&canonical.item_name))
}

pub fn raw_statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    match statement_for_concept(concept)? {
        "bs" => Some("bs"),
        "pl" => Some("pl"),
        "cf" => Some("cf"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_jppfs(family: &str, local_name: &str) -> ConceptKey {
        (
            format!("http://disclosure.edinet-fsa.go.jp/taxonomy/{family}/2014-03-31/{family}_cor"),
            local_name.to_string(),
        )
    }

    #[test]
    fn active_aliases_cover_legacy_edinet_jgaap_primary_statement_uris() {
        for family in ["jplvh", "jpsps", "jpctl"] {
            assert_eq!(
                canonicalize_semantic_concept(&legacy_jppfs(family, "NetSalesCNS"))
                    .map(|entry| entry.item_name),
                Some("revenue".to_string()),
                "active reviewed aliases must cover historical EDINET J-GAAP primary-statement family {family}",
            );
        }
    }
}

