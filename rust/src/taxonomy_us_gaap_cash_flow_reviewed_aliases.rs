use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_us_gaap_cash_flow_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // US-GAAP statement-of-cash-flows investing activities. These tags are
        // standard US-GAAP cash-flow elements and should not remain identity-only
        // snake_case candidates.
        "PaymentsToAcquirePropertyPlantAndEquipment" | "PaymentsToAcquireProductiveAssets" => {
            "capex"
        }
        "PaymentsToAcquireIntangibleAssets" => "intangible_asset_purchases",
        "ProceedsFromSaleOfPropertyPlantAndEquipment" => "ppe_sold",

        // US-GAAP investing cash-flow aliases for investment securities. These
        // are purchases and sale proceeds of securities, so they share the same
        // accounting scope as the existing investment-securities CF buckets.
        "PaymentsToAcquireAvailableForSaleSecurities" | "PaymentsToAcquireMarketableSecurities" => {
            "investment_securities_purchased"
        }
        "ProceedsFromSaleOfAvailableForSaleSecurities"
        | "ProceedsFromSaleOfMarketableSecurities" => "investment_securities_sold",

        // US-GAAP financing cash-flow aliases. Dividends paid are a financing
        // cash outflow under ASC 230, so the role-specific field is safer than
        // the generic IFRS/J-GAAP cash_dividends_paid bucket.
        "PaymentsOfDividends" => "cash_dividends_paid_financing_cf",
        "RepaymentsOfLongTermDebt" => "lt_debt_repayment",
        "ProceedsFromIssuanceOfLongTermDebt" => "lt_debt_proceeds",
        "PaymentsForRepurchaseOfCommonStock" | "PaymentsForRepurchaseOfEquity" => {
            "treasury_stock_purchase"
        }

        _ => return None,
    };
    Some(field)
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    let field = match base::classify_namespace_uri(&concept.0) {
        Some(TaxonomyFamily::UsGaap) => reviewed_us_gaap_cash_flow_alias(&concept.1)?,
        _ => return None,
    };

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
    reviewed_us_gaap_cash_flow_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

