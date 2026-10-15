use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed asset-side debt security aliases that must stay separate from
/// issuer-side commercial paper liabilities.
fn reviewed_debt_securities_alias(local_name: &str) -> Option<&'static str> {
    match local_name {
        // Commercial papers held as current financial assets. Do not collapse
        // this into issuer-side `commercial_paper` liabilities.
        "CommercialPapersCAFND" => Some("debt_securities"),
        _ => None,
    }
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_debt_securities_alias(&concept.1)?;
    Some(CanonicalConcept {
        item_name: field.to_string(),
        priority: 10,
        match_kind: MatchKind::TaxonomyFamilyAlias,
    })
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
    reviewed_debt_securities_alias(local_name).is_none() && base::is_excluded_concept(local_name)
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

