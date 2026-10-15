use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;

use crate::types::ConceptKey;

const CURRENT_PAYABLE_ALIASES: &[&str] = &[
    "AccountsPayable",
    "AccountsPayableAffiliatedCompanies",
    "AccountsPayableConsignment",
    "AccountsPayableDevelopmentBusinessAndOther",
    "AccountsPayableDueToMemberMerchants",
    "AccountsPayableFacilities",
    "AccountsPayableFacility",
    "AccountsPayableForEquipmentLiabilities",
    "AccountsPayableForFacilities",
    "AccountsPayableForFranchisee",
    "AccountsPayableFromSubsidiariesAndAffiliatesOther",
    "AccountsPayableFromSubsidiariesAndAssociatesOther",
    "AccountsPayableInstallmentPurchase",
    "AccountsPayableInstallmentPurchaseCL",
    "AccountsPayableOtherAndAccruedExpenses",
    "AccountsPayableOtherFromSubsidiariesAndAssociates",
    "AccountsPayableOtherToSubsidiariesAndAffiliates",
    "AccountsPayablePurchaseOfCredit",
    "AccountsPayableRealEstate",
    "AccountsPayableSurveyingSpecific",
    "AccountsPayableToAffiliatedCompanies",
    "AccountsPayableToSubsidiariesAndAssociates",
    "AgencySecurityCL",
    "AdvancesNotesPayableFacilitiesInSubsidiariesAndAffiliatesCL",
    "AdvancesObligationsFromSubsidiariesAndAffiliatesCL",
];

const ADVANCES_RECEIVED_ALIASES: &[&str] = &[
    "AdvanceDepositsReceivedCL",
    "AdvanceReceivedInterestInstalmentCL",
    "AdvanceReceivedSalesOfMaintenanceServiceCL",
    "AdvancesFromContractsInProgress",
    "AdvancesReceived",
    "AdvancesReceivedOnConstructionConstractsInprogress",
    "AdvancesReceivedOnConstructionContractsInProgress",
    "AdvancesReceivedOnDevelopmentProjectsCNS",
    "AdvancesReceivedOnUmcompletedConstructionContractsAndOtherCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsAndOtherCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsAndOthersCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsCNS",
    "AdvancesReceivedOnUncompletedContracts",
    "AdvancesReceivedRental",
];

const CURRENT_PROVISION_ALIASES: &[&str] = &[
    "AllowanceForAfterCareOfProductsCL",
    "AllowanceForBusinessRestructuringLossesCL",
    "AllowanceForCancellationOfContractCL",
    "AllowanceForCapitalLossOfRelatedCompaniesCL",
    "AllowanceForCoinUsageCL",
    "AllowanceForCompanyBuildingReplacementLossCL",
    "AllowanceForConstructionLossCL",
    "AllowanceForConstructionRepairCL",
    "AllowanceForDemolitionCostsCL",
    "AllowanceForDirectorsRetirementBenefitsCL",
    "AllowanceForDirectorsRetirementCL",
    "AllowanceForEarlyRetirementProgramCL",
    "AllowanceForEarlySubscriptionCancellationsCurrentLiabilitiesLiabilities",
    "AllowanceForEnvironmentalMeasuresCL",
    "AllowanceForEnvironmentalRemediationExpensesCL",
    "AllowanceForExpenseRelatedToRetrospectiveAdjustmentCL",
    "AllowanceForForeignIndirectTaxCL",
    "AllowanceForInvestmentLossCL",
    "AllowanceForInvestmentLossOfSubsidiariesAndAffiliatesCL",
    "AllowanceForLiquidationLossOfAffiliatedCompaniesCL",
    "AllowanceForLossOnCollectingGiftTicketsCurrentLiabilities",
    "AllowanceForLossOnDisposalOfReturnedGoodsCL",
    "AllowanceForLossOnSaleOfRelatedCompanySharesCL",
    "AllowanceForLossOnShortTermCancellationCL",
    "AllowanceForLossOnTransferOfSharesOfAffiliatedCompaniesCL",
    "AllowanceForLossOnVoluntaryRecallOfProductsCL",
    "AllowanceForLossRelatedToAntiMonopolyActCL",
    "AllowanceForLossesFromBusinessTransferCL",
    "AllowanceForLossesOnBusinessCL",
    "AllowanceForLossesOnBusinessTransferCL",
    "AllowanceForLossesOnCollecctingGiftTicketsCL",
    "AllowanceForLossesOnEmployeePensionFundWithdrawalCL",
    "AllowanceForLossesRelatedToTrustTypeStockOptionsCL",
    "AllowanceForOfficeTransferExpensesCL",
    "AllowanceForProductReturnedCL",
    "AllowanceForProductWarrantyCL",
    "AllowanceForProgramProductWarrantyCL",
    "AllowanceForQualityRelatedLossCL",
    "AllowanceForRecallCL",
    "AllowanceForRefundCL",
    "AllowanceForRelocationCostCL",
    "AllowanceForRepairProductCostsCL",
    "AllowanceForRepaymentCL",
    "AllowanceForResourceRecyclingExpensesCL",
    "AllowanceForRestructuringCL",
    "AllowanceForReturnsCL",
    "AllowanceForSalesDiscountCL",
    "AllowanceForShareholdersSpecialBenefitsCL",
    "AllowanceForShortTermCancellationLossCL",
    "AllowanceForSpecialInvestigationCL",
    "AllowanceForStockBasedCompensationCL",
    "AllowanceForSubscriptionCancellationsCL",
    "AllowanceForTransferOfSharesOfSubsidiariesAndAffiliatesCL",
    "AllowanceForWithdrawalOfBusinessCL",
    "AllowanceOfficerRetirementSpecialServiceMoneyCL",
    "AllowancesForLossOnPurchaseAgreementsCL",
    "AllowancesForSalesReturnsCL",
    "AloowanceForInvestmentLossCL",
    "AmendsForProductWarrantiesCL",
];

fn reviewed_field(local_name: &str) -> Option<&'static str> {
    if CURRENT_PAYABLE_ALIASES.contains(&local_name) {
        return Some("accounts_payable_current");
    }
    if ADVANCES_RECEIVED_ALIASES.contains(&local_name) {
        return Some("advances_received_current");
    }
    if CURRENT_PROVISION_ALIASES.contains(&local_name) {
        return Some("current_provisions");
    }
    None
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }
    Some(CanonicalConcept {
        item_name: reviewed_field(&concept.1)?.to_string(),
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
    reviewed_field(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "current_provisions" => Some(StatementCategory::BalanceSheet),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "current_provisions" => Some("bs"),
        _ => base::statement_for_item_name(item_name),
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
