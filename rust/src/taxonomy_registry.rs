//! Flat taxonomy registry.
//!
//! Reviewed aliases are independent leaf modules. The previous implementation
//! nested every alias module inside the next one, which made rustc type-check a
//! long forwarding chain and accidentally kept unreviewed identity candidates
//! on the production canonicalization path.

#[path = "taxonomy.rs"]
mod base;

mod taxonomy_edinet_jgaap_reviewed_aliases;
mod taxonomy_eps_reviewed_aliases;
mod taxonomy_jppfs_high_frequency_batch3_reviewed_aliases;
mod taxonomy_jppfs_high_frequency_batch2_reviewed_aliases;
mod taxonomy_high_frequency_reviewed_aliases;
mod taxonomy_working_capital_and_accumulated_assets_reviewed_aliases;
mod taxonomy_factchecked_working_capital_reviewed_aliases;
mod taxonomy_factchecked_receivable_payable_100_reviewed_aliases;
mod taxonomy_working_capital_100_reviewed_aliases;
mod taxonomy_bulk_current_liability_reviewed_aliases;
mod taxonomy_bulk_same_meaning_reviewed_aliases_20260617;
mod taxonomy_active_reviewed_aliases;
mod taxonomy_jppfs_statement_role_suffix_reviewed_aliases;
mod taxonomy_debt_securities_reviewed_aliases;
mod taxonomy_same_meaning_reviewed_aliases;
mod taxonomy_accumulated_depreciation_reviewed_aliases;
mod taxonomy_non_trade_receivables_reviewed_aliases;
mod taxonomy_paid_in_capital_reviewed_aliases;
mod taxonomy_industry_reviewed_aliases;
mod taxonomy_accrued_expenses_reviewed_aliases;
mod taxonomy_affiliate_payables_reviewed_aliases;
mod taxonomy_us_gaap_cash_flow_reviewed_aliases;
mod taxonomy_jppfs_amortization_cash_flow_reviewed_aliases;
mod taxonomy_amortization_reviewed_aliases;
mod taxonomy_operating_expenses_reviewed_aliases;
mod taxonomy_contract_receivables_payables_reviewed_aliases;
mod taxonomy_reviewed_aliases;

use crate::types::ConceptKey;

pub use base::{
    CanonicalConcept, MatchKind, StatementCategory, TaxonomyConceptStatus, TaxonomyFamily,
    canonicalize_synthetic, classify_namespace_uri,
};

#[cfg(test)]
pub use taxonomy_factchecked_working_capital_reviewed_aliases::{
    REVIEWED_WORKING_CAPITAL_ALIAS_COUNT, reviewed_working_capital_aliases_for_tests,
};

type AliasResolver = fn(&ConceptKey) -> Option<CanonicalConcept>;
type StatementResolver = fn(&str) -> Option<&'static str>;

const REVIEWED_EDINET_JGAAP_ALIAS_NAMESPACE: &str =
    "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/0000-00-00/jppfs_cor";

const ALIAS_RESOLVERS: &[AliasResolver] = &[
    taxonomy_eps_reviewed_aliases::reviewed_alias,
    taxonomy_edinet_jgaap_reviewed_aliases::reviewed_accounting_alias,
    taxonomy_jppfs_high_frequency_batch3_reviewed_aliases::reviewed_alias,
    taxonomy_jppfs_high_frequency_batch2_reviewed_aliases::reviewed_alias,
    taxonomy_high_frequency_reviewed_aliases::reviewed_alias,
    taxonomy_working_capital_and_accumulated_assets_reviewed_aliases::reviewed_alias,
    taxonomy_factchecked_working_capital_reviewed_aliases::reviewed_alias,
    taxonomy_factchecked_receivable_payable_100_reviewed_aliases::reviewed_alias,
    taxonomy_working_capital_100_reviewed_aliases::reviewed_alias,
    taxonomy_bulk_current_liability_reviewed_aliases::reviewed_alias,
    taxonomy_bulk_same_meaning_reviewed_aliases_20260617::reviewed_alias,
    taxonomy_active_reviewed_aliases::reviewed_alias,
    taxonomy_jppfs_statement_role_suffix_reviewed_aliases::reviewed_alias,
    taxonomy_debt_securities_reviewed_aliases::reviewed_alias,
    taxonomy_same_meaning_reviewed_aliases::reviewed_alias,
    taxonomy_accumulated_depreciation_reviewed_aliases::reviewed_alias,
    taxonomy_non_trade_receivables_reviewed_aliases::reviewed_alias,
    taxonomy_paid_in_capital_reviewed_aliases::reviewed_alias,
    taxonomy_industry_reviewed_aliases::reviewed_alias,
    taxonomy_accrued_expenses_reviewed_aliases::reviewed_alias,
    taxonomy_affiliate_payables_reviewed_aliases::reviewed_alias,
    taxonomy_us_gaap_cash_flow_reviewed_aliases::reviewed_alias,
    taxonomy_jppfs_amortization_cash_flow_reviewed_aliases::reviewed_alias,
    taxonomy_amortization_reviewed_aliases::reviewed_alias,
    taxonomy_operating_expenses_reviewed_aliases::reviewed_alias,
    taxonomy_contract_receivables_payables_reviewed_aliases::reviewed_alias,
    taxonomy_reviewed_aliases::reviewed_alias,
];

const STATEMENT_RESOLVERS: &[StatementResolver] = &[
    taxonomy_eps_reviewed_aliases::statement_for_item_name,
    taxonomy_edinet_jgaap_reviewed_aliases::statement_for_item_name,
    taxonomy_jppfs_high_frequency_batch3_reviewed_aliases::statement_for_item_name,
    taxonomy_jppfs_high_frequency_batch2_reviewed_aliases::statement_for_item_name,
    taxonomy_high_frequency_reviewed_aliases::statement_for_item_name,
    taxonomy_working_capital_and_accumulated_assets_reviewed_aliases::statement_for_item_name,
    taxonomy_factchecked_working_capital_reviewed_aliases::statement_for_item_name,
    taxonomy_factchecked_receivable_payable_100_reviewed_aliases::statement_for_item_name,
    taxonomy_working_capital_100_reviewed_aliases::statement_for_item_name,
    taxonomy_bulk_current_liability_reviewed_aliases::statement_for_item_name,
    taxonomy_bulk_same_meaning_reviewed_aliases_20260617::statement_for_item_name,
    taxonomy_active_reviewed_aliases::statement_for_item_name,
    taxonomy_jppfs_statement_role_suffix_reviewed_aliases::statement_for_item_name,
    taxonomy_debt_securities_reviewed_aliases::statement_for_item_name,
    taxonomy_same_meaning_reviewed_aliases::statement_for_item_name,
    taxonomy_accumulated_depreciation_reviewed_aliases::statement_for_item_name,
    taxonomy_non_trade_receivables_reviewed_aliases::statement_for_item_name,
    taxonomy_paid_in_capital_reviewed_aliases::statement_for_item_name,
    taxonomy_industry_reviewed_aliases::statement_for_item_name,
    taxonomy_accrued_expenses_reviewed_aliases::statement_for_item_name,
    taxonomy_affiliate_payables_reviewed_aliases::statement_for_item_name,
    taxonomy_us_gaap_cash_flow_reviewed_aliases::statement_for_item_name,
    taxonomy_jppfs_amortization_cash_flow_reviewed_aliases::statement_for_item_name,
    taxonomy_amortization_reviewed_aliases::statement_for_item_name,
    taxonomy_operating_expenses_reviewed_aliases::statement_for_item_name,
    taxonomy_contract_receivables_payables_reviewed_aliases::statement_for_item_name,
    taxonomy_reviewed_aliases::statement_for_item_name,
];

fn is_edinet_jgaap_sibling_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jplvh/")
        || uri.contains("/taxonomy/jpsps/")
        || uri.contains("/taxonomy/jpctl/")
}

fn normalized_edinet_jgaap_concept(concept: &ConceptKey) -> Option<ConceptKey> {
    is_edinet_jgaap_sibling_namespace(&concept.0).then(|| {
        (
            REVIEWED_EDINET_JGAAP_ALIAS_NAMESPACE.to_string(),
            concept.1.clone(),
        )
    })
}

fn resolve_reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    ALIAS_RESOLVERS
        .iter()
        .find_map(|resolve| resolve(concept))
        .or_else(|| base::canonicalize_semantic_concept(concept))
}

/// Resolve only accounting-reviewed concepts.
///
/// Unreviewed official concepts no longer receive fabricated snake_case field
/// names. They remain Unknown until an explicit, namespace-gated alias is added.
pub fn canonicalize_semantic_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    resolve_reviewed_alias(concept).or_else(|| {
        normalized_edinet_jgaap_concept(concept)
            .as_ref()
            .and_then(resolve_reviewed_alias)
    })
}

pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    canonicalize_semantic_concept(concept)
}

pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(canonical) = canonicalize_semantic_concept(concept) {
        return TaxonomyConceptStatus::Semantic(canonical);
    }

    if classify_namespace_uri(&concept.0).is_none()
        && !is_edinet_jgaap_sibling_namespace(&concept.0)
    {
        return TaxonomyConceptStatus::Extension;
    }

    if base::is_excluded_concept(&concept.1) {
        return TaxonomyConceptStatus::Excluded;
    }

    TaxonomyConceptStatus::Unknown
}

pub fn is_excluded_concept(local_name: &str) -> bool {
    const REVIEWED_NAMESPACES: &[&str] = &[
        REVIEWED_EDINET_JGAAP_ALIAS_NAMESPACE,
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/0000-00-00/jpigp_cor",
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/0000-00-00/jpcrp_cor",
        "https://fasb.org/us-gaap/2024",
        "https://xbrl.ifrs.org/taxonomy/2024-03-27/ifrs-full",
    ];

    let is_reviewed = REVIEWED_NAMESPACES.iter().any(|namespace| {
        let representative = ((*namespace).to_string(), local_name.to_string());
        resolve_reviewed_alias(&representative).is_some()
    });

    !is_reviewed && base::is_excluded_concept(local_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    let baseline = base::statement_for_item_name(item_name);

    STATEMENT_RESOLVERS
        .iter()
        .map(|resolve| resolve(item_name))
        .find(|candidate| candidate.is_some() && candidate != &baseline)
        .flatten()
        .or(baseline)
}

pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match statement_for_item_name(item_name) {
        Some("bs") => Some(StatementCategory::BalanceSheet),
        Some("pl") => Some(StatementCategory::ProfitAndLoss),
        Some("cf") => Some(StatementCategory::CashFlow),
        Some("changes_in_equity") => Some(StatementCategory::ChangesInEquity),
        Some("oci") => Some(StatementCategory::OtherComprehensiveIncome),
        Some("dividend") => Some(StatementCategory::PerShare),
        Some("forecast") => Some(StatementCategory::Forecast),
        Some("disclosure") => Some(StatementCategory::Disclosure),
        _ => base::classify_item_name(item_name),
    }
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

    fn concept(namespace: &str, local_name: &str) -> ConceptKey {
        (namespace.to_string(), local_name.to_string())
    }

    #[test]
    fn unreviewed_official_concepts_remain_unknown() {
        let candidate = concept(
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor",
            "AccountsPayableOther",
        );

        assert!(canonicalize_concept(&candidate).is_none());
        assert!(matches!(
            concept_status(&candidate),
            TaxonomyConceptStatus::Unknown
        ));
    }

    #[test]
    fn company_extensions_remain_extension_gated() {
        let candidate = concept("https://example.com/issuer/2026", "Revenue");

        assert!(canonicalize_concept(&candidate).is_none());
        assert!(matches!(
            concept_status(&candidate),
            TaxonomyConceptStatus::Extension
        ));
    }

    #[test]
    fn legacy_edinet_jgaap_namespaces_use_reviewed_aliases() {
        let candidate = concept(
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jplvh/2014-03-31/jplvh_cor",
            "NetSalesCNS",
        );

        assert_eq!(
            canonicalize_semantic_concept(&candidate).map(|entry| entry.item_name),
            Some("revenue".to_string())
        );
    }
}
