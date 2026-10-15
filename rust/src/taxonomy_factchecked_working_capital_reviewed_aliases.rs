use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

#[cfg(test)]
pub const REVIEWED_WORKING_CAPITAL_ALIAS_COUNT: usize = 75;

const REVIEWED_WORKING_CAPITAL_ALIASES: &[(&str, &str)] = &[
    ("AccountsPayableCL", "accounts_payable_current"),
    ("AccountsPayableCLIFRS", "accounts_payable_current"),
    ("AccountsPayableCLINV", "accounts_payable_current"),
    ("AccountsPayableOtherCL", "accounts_payable_current"),
    ("AccountsPayableOtherBusinessCL", "accounts_payable_current"),
    (
        "AccountsPayableDirectorsBonuses",
        "accounts_payable_current",
    ),
    ("AccountsPayableGasolineTax", "accounts_payable_current"),
    (
        "AccountsForNonReceivedSecuritiesAndOthersCLSEC",
        "accounts_payable_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableAffiliatedCompaniesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableForAffiliatedCompanyCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOtherCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesOtherCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    ("AccountsPayableOfCollectionCL", "accounts_payable_current"),
    ("AccountsPayableOnMarginCL", "accounts_payable_current"),
    (
        "AccountsPayableOperatingSpecific",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOtherFromSubsidiariesAndAffiliatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOtherToAffiliatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOtherToSubsidiariesAndAffiliatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOtherToSubsidiariesAndAffiliatesCLCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableToSubsidiariesAndAffiliatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableToSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountPayableTradeForFranchisedStoreCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesTradeCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL",
        "accounts_payable_trade",
    ),
    ("AccountsPayableTrade", "accounts_payable_trade"),
    ("AccountsPayableTradeCL", "accounts_payable_trade"),
    (
        "AccountsPayableTradeForFranchisedStores",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeForFranchisedStoresCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeFromSubsidiariesAndAffiliates",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeLiabilitiesIFRS",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeToSubsidiariesAndAffiliatesCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableDevelopmentProjectsInProgressCNS",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountsPayableForEquipmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForEquipmentInvestmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilitiesvCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableOfBuyNoncurrentAssetsCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayablePropertyPlantAndEquipmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AFactoringAccountPayable", "factoring_accounts_payable"),
    ("AFactoringAccountPayableCL", "factoring_accounts_payable"),
    ("AccountPayableFactoring", "factoring_accounts_payable"),
    ("AccountsPayableForFactoring", "factoring_accounts_payable"),
    (
        "AccountsPayableForFactoringCL",
        "factoring_accounts_payable",
    ),
    (
        "AccountsPayableRealEstateCL",
        "accounts_payable_real_estate_current",
    ),
    (
        "AccountsPayableRentalAssetsCL",
        "accounts_payable_for_rental_assets_current",
    ),
    (
        "AccountsPayableRentalAssetsCLLiabilities",
        "accounts_payable_for_rental_assets_current",
    ),
    (
        "AccountsPayableSurveyingCL",
        "accounts_payable_surveying_current",
    ),
    (
        "AcceptanceMoneySuchAsDevelopmentWorkCL",
        "advances_received_current",
    ),
    (
        "AcceptanceMoneySuchAsDevelopmentWork2CL",
        "advances_received_current",
    ),
    ("AdvancesReceivedCLIFRS", "advances_received_current"),
    (
        "AdvancesReceivedFromSubsidiariesAndAffiliatesCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnDevelopementProjectsCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnDevelopmentProjectsCLCNS",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnOtherBusinessCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsAndAdvancesReceivedCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsAndOtherCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedContractsCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedSideLineBusinessCL",
        "advances_received_current",
    ),
    ("AdvancesRentIncomeCL", "advances_received_current"),
    (
        "AdvancesFromContractsInProgress",
        "advances_received_current",
    ),
    ("AdvancesReceived", "advances_received_current"),
    (
        "AdvancesReceivedOnConstructionConstractsInprogress",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnConstructionContractsInProgress",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnDevelopmentProjectsCNS",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUmcompletedConstructionContractsAndOtherCNS",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsAndOtherCNS",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsAndOthersCNS",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnUncompletedConstructionContractsCNS",
        "advances_received_on_uncompleted_construction_contracts",
    ),
    (
        "AdvancesReceivedOnUncompletedContracts",
        "advances_received_current",
    ),
    ("AdvancesReceivedRental", "advances_received_current"),
    (
        "AccountPayableAndAllowanceForEvaluationLossOfDerivativesCL",
        "accounts_payable_and_derivative_valuation_allowance_current",
    ),
    (
        "AccountsPayableDirectorSRetirementBenefitsNCL",
        "accounts_payable_directors_retirement_benefits_non_current",
    ),
    (
        "AccountsPayableDirectorsRetirementBenefitsNCL",
        "accounts_payable_directors_retirement_benefits_non_current",
    ),
];

fn reviewed_working_capital_alias(local_name: &str) -> Option<&'static str> {
    REVIEWED_WORKING_CAPITAL_ALIASES
        .iter()
        .find_map(|(alias, field)| (*alias == local_name).then_some(*field))
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }
    let field = reviewed_working_capital_alias(&concept.1)?;
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

fn is_reviewed_balance_sheet_item_name(item_name: &str) -> bool {
    matches!(
        item_name,
        "accounts_payable_and_derivative_valuation_allowance_current"
            | "accounts_payable_directors_retirement_benefits_non_current"
            | "accounts_payable_for_construction_contracts_current"
            | "accounts_payable_for_property_plant_and_equipment_current"
            | "accounts_payable_for_rental_assets_current"
            | "accounts_payable_real_estate_current"
            | "accounts_payable_surveying_current"
            | "advances_received_current"
            | "advances_received_on_uncompleted_construction_contracts"
            | "factoring_accounts_payable"
            | "accumulated_depreciation"
            | "accumulated_depreciation_and_impairment_losses"
    )
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
    reviewed_working_capital_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    if is_reviewed_balance_sheet_item_name(item_name) {
        Some(StatementCategory::BalanceSheet)
    } else {
        base::classify_item_name(item_name)
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    if is_reviewed_balance_sheet_item_name(item_name) {
        Some("bs")
    } else {
        base::statement_for_item_name(item_name)
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
pub fn reviewed_working_capital_aliases_for_tests() -> &'static [(&'static str, &'static str)] {
    REVIEWED_WORKING_CAPITAL_ALIASES
}
