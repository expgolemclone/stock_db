use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

// The high-frequency reviewed overlay is local-name based but is intentionally
// namespace-gated. EDINET J-GAAP financial-statement taxonomies include sibling
// families such as jplvh, jpsps, and jpctl; taxonomy.rs already classifies their
// field-map prefixes as the JPPFS family. Normalize only those official EDINET
// namespaces to a synthetic JPPFS namespace before delegating to the reviewed
// overlay so issuer extensions still remain extension-gated.
const REVIEWED_EDINET_JGAAP_ALIAS_NAMESPACE: &str =
    "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/0000-00-00/jppfs_cor";

fn is_edinet_jgaap_sibling_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jplvh/")
        || uri.contains("/taxonomy/jpsps/")
        || uri.contains("/taxonomy/jpctl/")
}

fn is_edinet_jgaap_primary_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/") || is_edinet_jgaap_sibling_namespace(uri)
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

fn normalized_edinet_jgaap_review_concept(concept: &ConceptKey) -> Option<ConceptKey> {
    if is_edinet_jgaap_sibling_namespace(&concept.0) {
        Some((
            REVIEWED_EDINET_JGAAP_ALIAS_NAMESPACE.to_string(),
            concept.1.clone(),
        ))
    } else {
        None
    }
}

fn with_edinet_jgaap_review_fallback<F>(concept: &ConceptKey, f: F) -> Option<CanonicalConcept>
where
    F: Fn(&ConceptKey) -> Option<CanonicalConcept>,
{
    f(concept).or_else(|| normalized_edinet_jgaap_review_concept(concept).and_then(|c| f(&c)))
}

/// Small accounting-review layer for aliases whose local names are easy to
/// misclassify when the broader high-frequency layer is extended.
pub(super) fn reviewed_accounting_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    let field = match concept.1.as_str() {
        // Obligations arising from customer contracts. This keeps the existing
        // canonical output stable while making the liability-side decision
        // explicit and namespace-gated.
        "ContractLiabilities" if is_edinet_jgaap_primary_namespace(&concept.0) => {
            "contract_liabilities"
        }

        // Insurance-industry presentation for corporate bonds held as assets.
        // Do not collapse this into `bonds_payable`, which is a liability for
        // bonds issued by the filer.
        "CorporateBondsAssetsINS" if is_jppfs_namespace(&concept.0) => {
            "corporate_bonds_investment_securities_assets"
        }

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
    reviewed_accounting_alias(concept)
        .or_else(|| with_edinet_jgaap_review_fallback(concept, base::canonicalize_semantic_concept))
}

pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    reviewed_accounting_alias(concept)
        .or_else(|| with_edinet_jgaap_review_fallback(concept, base::canonicalize_concept))
}

pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(alias) = reviewed_accounting_alias(concept) {
        return TaxonomyConceptStatus::Semantic(alias);
    }

    match base::concept_status(concept) {
        TaxonomyConceptStatus::Extension if is_edinet_jgaap_sibling_namespace(&concept.0) => {
            let normalized = normalized_edinet_jgaap_review_concept(concept)
                .expect("sibling namespace should normalize");
            base::concept_status(&normalized)
        }
        status => status,
    }
}

#[cfg(test)]
pub fn is_excluded_concept(local_name: &str) -> bool {
    !matches!(local_name, "ContractLiabilities" | "CorporateBondsAssetsINS")
        && base::is_excluded_concept(local_name)
}

fn reviewed_accounting_statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "contract_liabilities" | "corporate_bonds_investment_securities_assets" => Some("bs"),
        _ => None,
    }
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match reviewed_accounting_statement_for_item_name(item_name) {
        Some("bs") => Some(StatementCategory::BalanceSheet),
        Some("pl") => Some(StatementCategory::ProfitAndLoss),
        Some("cf") => Some(StatementCategory::CashFlow),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    reviewed_accounting_statement_for_item_name(item_name)
        .or_else(|| base::statement_for_item_name(item_name))
}

pub fn statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    canonicalize_concept(concept).and_then(|canonical| statement_for_item_name(&canonical.item_name))
}

pub fn raw_statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    match statement_for_concept(concept)? {
        "bs" => Some("bs"),
        "pl" => Some("pl"),
        "cf" => Some("cf"),
        _ => None,
    }
}

