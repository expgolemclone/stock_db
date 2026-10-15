use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn is_ifrs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jpigp/") || uri.contains("ifrs.org") || uri.contains("xbrl.ifrs")
}

fn is_us_gaap_namespace(uri: &str) -> bool {
    uri.contains("fasb.org/us-gaap")
}

fn reviewed_paid_in_capital_alias(concept: &ConceptKey) -> Option<&'static str> {
    let field = match concept.1.as_str() {
        // IFRS/EDINET-IFRS share premium is a paid-in equity component, but it is
        // not the Japanese statutory capital-surplus bucket. Preserve the
        // existing canonical SharePremium field instead of widening it to
        // capital_surplus.
        "SharePremium" | "SharePremiumIFRS" if is_ifrs_namespace(&concept.0) => "share_premium",

        // US GAAP APIC is likewise a distinct paid-in capital component. Keep it
        // in the AdditionalPaidInCapital bucket instead of merging it into the
        // Japanese capital_surplus field.
        "AdditionalPaidInCapital" | "AdditionalPaidInCapitalUSGAAP"
            if is_us_gaap_namespace(&concept.0) =>
        {
            "additional_paid_in_capital"
        }
        _ => return None,
    };
    Some(field)
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    let field = reviewed_paid_in_capital_alias(concept)?;
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
    } else if local_name.contains("IFRS") || local_name.contains("USGAAP") {
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
    reviewed_paid_in_capital_alias(&(
        "http://www.ifrs.org/taxonomy/2024".to_string(),
        local_name.to_string(),
    ))
    .is_none()
        && reviewed_paid_in_capital_alias(&(
            "http://fasb.org/us-gaap/2024".to_string(),
            local_name.to_string(),
        ))
        .is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

