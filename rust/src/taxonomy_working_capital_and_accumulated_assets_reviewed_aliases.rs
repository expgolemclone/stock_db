use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP working-capital and accumulated-asset aliases added on
/// 2026-06-17.
///
/// The aliases below intentionally group exactly 100 formerly identity-only
/// historical tags into reviewed canonical fields. The groups are narrow:
/// current receivables stay in a broad current-receivables bucket unless the
/// wording proves a pure trade receivable; accrued expenses/payables stay on
/// the liability side; and accumulated depreciation / impairment stays on the
/// balance-sheet contra-asset side rather than profit-or-loss depreciation or
/// impairment expense.
const REVIEWED_JPPFS_ALIASES_20260617: &[(&str, &str)] = &[
    (
        "AccountsReceivableDevelopmentProjectsCACNS",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableInstallment",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherNotesConsolidatedBalanceSheet",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableRealEstateBusinessAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableTenantsCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccruedConsumptionTaxRefundCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccruedCorporationTaxCA",
        "trade_and_other_current_receivables",
    ),
    ("AccruedIncome", "trade_and_other_current_receivables"),
    (
        "AccruedIncomeOtherFromSubsidiariesAndAffiliates",
        "trade_and_other_current_receivables",
    ),
    (
        "AccruedIncomeOtherFromSubsidiariesAndAffiliatesCA",
        "trade_and_other_current_receivables",
    ),
    ("AccruedIncomeTaxCA", "trade_and_other_current_receivables"),
    (
        "AccruedInterestFromSubsidiariesAndAssociatesOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccruedInvestmentTrustManagementFeeCAIVT",
        "trade_and_other_current_receivables",
    ),
    (
        "AccruedPremiumsAssetsINS",
        "trade_and_other_current_receivables",
    ),
    ("AccruedRentalIncome", "trade_and_other_current_receivables"),
    ("AccruedTaxCA", "trade_and_other_current_receivables"),
    ("AccruedBonuses", "accrued_expenses_current"),
    ("AccruedBonusesCLIFRS", "accrued_expenses_current"),
    ("AccruedBusinessOfficeTaxes", "accounts_payable_current"),
    ("AccruedCompensationCLIFRS", "accrued_expenses_current"),
    ("AccruedConsumptionTaxes", "accounts_payable_current"),
    ("AccruedCorporateInhabitantTax", "accounts_payable_current"),
    ("AccruedCorporateTax", "accounts_payable_current"),
    ("AccruedDirectorSBonusCL", "accrued_expenses_current"),
    ("AccruedDirectorsBonusCL", "accrued_expenses_current"),
    (
        "AccruedExpenseForRepeatingCorsesCL",
        "accrued_expenses_current",
    ),
    (
        "AccruedExpensesForOverhaulingMajorSmeltingEquipmentCL",
        "accrued_expenses_current",
    ),
    (
        "AccruedExpensesToSubsidiariesAndAffiliates",
        "accrued_expenses_current",
    ),
    (
        "AccruedExpensesToSubsidiariesAndAffiliatesCL",
        "accrued_expenses_current",
    ),
    ("AccruedGasolineTax", "accounts_payable_current"),
    ("AccruedIncomeTaxes", "accounts_payable_current"),
    ("AccruedInterestCL", "accounts_payable_current"),
    ("AccruedLaborExpensesCLIFRS", "accrued_expenses_current"),
    ("AccruedLiabilityForFactoring", "factoring_accounts_payable"),
    (
        "AccruedLiabilityForFactoringCL",
        "factoring_accounts_payable",
    ),
    ("AccruedRestructuringChargesCL", "accrued_expenses_current"),
    ("AccruedSalesReturnsCL", "accounts_payable_current"),
    (
        "AccruedVolatileOilAndOtherPetroleumTaxesCL",
        "accounts_payable_current",
    ),
    (
        "AccumulatedImpairmentLossBuildings",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossBuildingsAndAccompanyingFacilities",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossBuildingsAndStructures",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossConstructionInProgress",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossLandUsedForMiningOperations",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossLeaseAssetsPPE",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossMachineryAndEquipment",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossMachineryEquipmentAndVehicles",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossOtherPPE",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossPPEByGroup",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossRentalAssetsPPE",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossRightOfUseAssets",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossStructures",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossToolsFurnitureAndFixtures",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentLossVehicles",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedImpairmentRightOfUseAssets",
        "accumulated_depreciation_and_impairment_losses",
    ),
    (
        "AccumulatedDepreciationToolsFurnitureAndFixtures",
        "accumulated_depreciation",
    ),
    (
        "AccumulatedDepreciationToolsFurnitureAndFixturesInTrustINV",
        "accumulated_depreciation",
    ),
    (
        "AccumulatedDepreciationToolsFurnitureAndFixturesNCATelecommunications",
        "accumulated_depreciation",
    ),
    (
        "AccumulatedDepreciationUseOfRightAssetsPPE",
        "accumulated_depreciation",
    ),
    (
        "AccumulatedDepreciationUseRightsAssets",
        "accumulated_depreciation",
    ),
    (
        "ACcountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    ("AccountPayableFactoring", "factoring_accounts_payable"),
    (
        "AccountPayableForWorksInProgressAndOtherCL",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    ("AccountsPayableConsignment", "accounts_payable_current"),
    (
        "AccountsPayableDevelopmentBusinessAndOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableDueToMemberMerchants",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFacilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableFacility",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForEquipmentLiabilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AccountsPayableForFactoring", "factoring_accounts_payable"),
    ("AccountsPayableForFranchisee", "accounts_payable_current"),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAssociatesOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOtherToSubsidiariesAndAffiliates",
        "accounts_payable_current",
    ),
    (
        "AccountsPayablePurchaseOfCredit",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableRealEstate",
        "accounts_payable_real_estate_current",
    ),
    (
        "AccountsPayableSurveyingSpecific",
        "accounts_payable_surveying_current",
    ),
    (
        "AccountsPayableToSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableRealEstateAndOtherCL",
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
    (
        "AccountsPayableDevelopmentProjectsInProgressCNS",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountsPayableForConstructionContracts2CNS",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountsPayableForSideLineBusinessCNS",
        "accounts_payable_trade",
    ),
    (
        "AccruedRetirementLumpSumPlanForDirectorsCL",
        "accrued_expenses_current",
    ),
    (
        "AccountsReceivableDueFromFranchisedStores",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromSideBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAssociatesCL",
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
];

fn reviewed_jppfs_alias_20260617(local_name: &str) -> Option<&'static str> {
    REVIEWED_JPPFS_ALIASES_20260617
        .iter()
        .find_map(|(name, field)| (*name == local_name).then_some(*field))
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }
    let field = reviewed_jppfs_alias_20260617(&concept.1)?;
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
    reviewed_jppfs_alias_20260617(local_name).is_none() && base::is_excluded_concept(local_name)
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

