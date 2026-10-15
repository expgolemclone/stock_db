use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP statement-role suffix aliases.
///
/// This layer is intentionally a fallback after the specific reviewed alias
/// stack. More specific fields such as `other_non_operating_income` must win
/// before this classifier groups remaining JPPFS scalar concepts by their
/// NOI / NOE / EI / EL statement-role suffixes.
///
/// Most extraordinary-income names are recognizable from their wording. The
/// exception table contains historically observed EI tags whose labels were
/// reviewed but do not carry an income/gain keyword in the local name.
const EXTRAORDINARY_INCOME_EXCEPTIONS: &[&str] = &[
    "AffiliatedCompanyConsolidationProfitEI",
    "AmortizationOfGoodwillEI",
    "AmortizationOfPriorServiceCostEI",
    "AnIntroductionFeeEI",
    "AntiDumpingDutiesEI",
    "AssetRetirementObligationEI",
    "BuildingDemolitionCostRestitutionProfitEI",
    "BurdenChargeForDevelopmentEI",
    "BusinessCompensationEI",
    "BusinessTaxRefundEI",
    "CashBeneficiaryEI",
    "ChangeProfitEI",
    "ChangesInEquityInterestEI",
    "CityGasCooperationFeeEI",
    "CollectionOfDamagesCompensationEI",
    "CollectionOfReceivableFromSubsidiariesAndAffiliatesEI",
    "CompensationExpensesEI",
    "CompensationForDamageCommisionEI",
    "CompensationForDamageEI",
    "CompensationForEvictionEI",
    "CompensationForHeadOfficeTransferEI",
    "CompensationForLossEI",
    "CompensationForMovingExpensesEI",
    "CompensationForRemovalEI",
    "CompensationForStoreClosingEI",
    "CompensationForTechnicalServicesEI",
    "CompensationForTheDamageEI",
    "CompensationForTransferEI",
    "CompensationForVacationEI",
    "CompensationOfRevenueEI",
    "ConsumptionTaxesDifferentialEI",
    "ConsumptionTaxesForPriorPeriodsEI",
    "ContractCancellationRevenueEI",
    "ContractTerminationRevenueEI",
    "ContributionForConstructionEI",
    "ContributionForConstructionRefundEI",
    "ContributionForDonationEI",
    "ContributionForRestorationEI",
    "ContributionFromRecapitalizationEI",
];

fn reviewed_jppfs_statement_role_suffix_alias(local_name: &str) -> Option<&'static str> {
    if is_non_scalar_concept(local_name) {
        return None;
    }

    if local_name.ends_with("NOI") {
        return Some("non_operating_income");
    }
    if local_name.ends_with("NOE") {
        return Some("non_operating_expenses");
    }
    if local_name.ends_with("EL") {
        return Some("extraordinary_loss");
    }
    if is_extraordinary_income_name(local_name) {
        return Some("extraordinary_income");
    }

    None
}

fn is_extraordinary_income_name(local_name: &str) -> bool {
    local_name.ends_with("EI")
        && (local_name.contains("Income")
            || local_name.contains("Gain")
            || local_name.contains("Subsidy")
            || local_name.contains("Adjustment")
            || local_name.contains("Received")
            || EXTRAORDINARY_INCOME_EXCEPTIONS.contains(&local_name))
}

fn is_non_scalar_concept(local_name: &str) -> bool {
    local_name.contains("TextBlock")
        || local_name.ends_with("Abstract")
        || local_name.ends_with("Table")
        || local_name.ends_with("LineItems")
        || local_name.ends_with("Axis")
        || local_name.ends_with("Domain")
        || local_name.ends_with("Member")
        || local_name.ends_with("Heading")
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_statement_role_suffix_alias(&concept.1)?;
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
    base::canonicalize_semantic_concept(concept).or_else(|| reviewed_alias(concept))
}

pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    canonicalize_semantic_concept(concept).or_else(|| base::canonicalize_concept(concept))
}

pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(alias) = canonicalize_semantic_concept(concept) {
        return TaxonomyConceptStatus::Semantic(alias);
    }
    base::concept_status(concept)
}

#[cfg(test)]
pub fn is_excluded_concept(local_name: &str) -> bool {
    reviewed_jppfs_statement_role_suffix_alias(local_name).is_none()
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

