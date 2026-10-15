use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_accrued_expenses_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Accrued expenses / accrued liabilities are obligations for expenses
        // already incurred but not yet paid. Keep them in a current-liability
        // accrued-expense bucket rather than collapsing them into trade accounts
        // payable. Do not include OpeCF variants here because those are cash-flow
        // working-capital adjustment concepts, not balance-sheet line items.
        "AccruedExpenses" | "AccruedExpensesCLIFRS" | "AccruedLiabilitiesCLIFRS" => {
            "accrued_expenses_current"
        }

        _ => return None,
    };
    Some(field)
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !matches!(
        base::classify_namespace_uri(&concept.0),
        Some(TaxonomyFamily::EdinetJppfs)
            | Some(TaxonomyFamily::EdinetJpigp)
            | Some(TaxonomyFamily::Ifrs)
    ) {
        return None;
    }

    let field = reviewed_accrued_expenses_alias(&concept.1)?;
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
    reviewed_accrued_expenses_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "accrued_expenses_current" => Some(StatementCategory::BalanceSheet),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "accrued_expenses_current" => Some("bs"),
        _ => base::statement_for_item_name(item_name),
    }
}

