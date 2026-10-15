use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_amortization_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Profit-and-loss amortization aliases. These describe period expenses
        // and must stay separate from operating cash-flow add-back concepts.
        "AmoritisationOfIntangibleAssetsIFRS"
        | "AmortizationIFRS"
        | "AmortizationOfIdentifiableAssetsIFRS"
        | "AmortizationOfIntangibleAssets"
        | "AmortizationOfIntangibleAssetsAssociatedWithProductsIFRS"
        | "AmortizationOnIntangibleAssetsAssociatedWithProductsIFRS" => "amortization",

        // Operating cash-flow amortization aliases. These are noncash PL add-backs
        // in the indirect-method cash-flow statement, not PL amortization expense.
        "AmoitizationOfDefferedAssetsOpeCF"
        | "AmortizationExpensesOfServicingAssetsOpeCFIFRS"
        | "AmortizationLongTermPrepaidExpensesOpeCF"
        | "AmortizationOfContactCostsOpeCFIFRS"
        | "AmortizationOfContractRelatedIntangibleAssetsOpeCF"
        | "AmortizationOfCustomerDataOpeCF"
        | "AmortizationOfCustomerRelatedAssetsOpeCF"
        | "AmortizationOfCustomerRelationshipCF"
        | "AmortizationOfCustomerRelationshipOpeCF"
        | "AmortizationOfCustomerReletionshipOpeCF"
        | "AmortizationOfDeferredAssetsOpeCF"
        | "AmortizationOfIntangibleAssetOpeCFIFRS"
        | "AmortizationOfIntangibleAssetsOpeCFIFRS"
        | "AmortizationOfOtherIntangibleAssetsOpeCF"
        | "AmortizationOfRightOfUseAssetsOpeCF"
        | "AmortizationOfRightOfUseAssetsOpeCFIFRS"
        | "AmortizationOpeCFIFRS" => "amortization_cf_adjustment",

        _ => return None,
    };
    Some(field)
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !matches!(
        base::classify_namespace_uri(&concept.0),
        Some(TaxonomyFamily::EdinetJppfs)
    ) {
        return None;
    }
    let field = reviewed_jppfs_amortization_alias(&concept.1)?;
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
    reviewed_jppfs_amortization_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

