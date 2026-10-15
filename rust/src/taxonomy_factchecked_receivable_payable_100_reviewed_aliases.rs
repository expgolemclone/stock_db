use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP receivable/payable aliases whose local names carry
/// equivalent working-capital meanings but had remained as separate identity
/// tags.
///
/// This layer intentionally keeps trade/customer receivables, broad current
/// receivables, and current payables in distinct canonical buckets.  It avoids
/// widening specific construction/equipment/factoring payables that lower
/// fact-checked layers already classify more narrowly.
fn reviewed_jppfs_receivable_payable_100_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        "AccountReceivableMedicalIncomeCA"
        | "AccountsReceivableAdvisoryFeeCA"
        | "AccountsReceivableCareBusiness"
        | "AccountsReceivableCareBusinessCA"
        | "AccountsReceivableCareBusinessCAAssets"
        | "AccountsReceivableCommissionCA"
        | "AccountsReceivableCompletedConstructionCA"
        | "AccountsReceivableCompletedOperation"
        | "AccountsReceivableCompletedOperationCA"
        | "AccountsReceivableCompletedSurveyCA"
        | "AccountsReceivableDevelopmentBusiness"
        | "AccountsReceivableDueFromFranchisedStores"
        | "AccountsReceivableFromFranchisee"
        | "AccountsReceivableFromSalesOnSideLineBusiness"
        | "AccountsReceivableFromSideBusinessCA"
        | "AccountsReceivableFromSideLineBusinessCNS"
        | "AccountsReceivableFromSubsidiariesAndAffiliatesTrade"
        | "AccountsReceivableInstallment"
        | "AccountsReceivableRealEstateBusiness"
        | "AccountsReceivableRealEstateBusinessCA"
        | "AccountsReceivableSideLineBusinessCA"
        | "AccountsReceivableTenantsCA"
        | "AccountsReceivableTrade1CA"
        | "AccountsReceivableTradeAccountsReceivableTrade"
        | "AccountsReceivableTradeCAINV"
        | "AccountsReceivableTradeNet"
        | "AccountsReceivableTradeReceivablesFromContractsWithCustomers"
        | "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivablerealEstateBusiness"
        | "AccountsReceivableCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCNS"
        | "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS"
        | "AccounAccountsReceivableFromCompletedConstructionContractstsReceivableFromCompletedConstructionContracts" => {
            "accounts_receivable_trade"
        }

        "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssets"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS"
        | "AccountsReceivableOtherAndContractAssetCA" => "accounts_receivable_and_contract_assets",

        "AccountsReceivableDevelopmentBusinessAndOther"
        | "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA"
        | "AccountsReceivableOtherNotesConsolidatedBalanceSheet"
        | "AccountsReceivableRealEstateBusinessAndOther"
        | "AccountsReceivableFromCompletedConstructionContractsAndOther"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS"
        | "AccruedConsumptionTaxRefundCA"
        | "AccruedIncome"
        | "AccruedIncomeOtherFromSubsidiariesAndAffiliates"
        | "AccruedIncomeOtherFromSubsidiariesAndAffiliatesCA"
        | "AccruedInterest"
        | "AccruedInterestFromSubsidiariesAndAssociatesOther"
        | "AccruedInvestmentTrustManagementFeeCAIVT"
        | "AccruedPremiumsAssetsINS"
        | "AccruedRentalIncome"
        | "AccruedTaxCA"
        | "AccountsReceivableOtherFromSubsidiariesAndAffiliates"
        | "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA"
        | "AccountsReceivableOtherFromFactor"
        | "AccountsReceivableOtherNetCA"
        | "AccountsReceivableOtherOfStorageAgencyCA"
        | "AccountsReceivablePaidProvisionCA"
        | "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA"
        | "AccountsReceivableRentalCA"
        | "AccountsReceivableRentalCAAssets"
        | "AccountsReceivableOtherBusinessCAWAT" => "trade_and_other_current_receivables",

        "AccountsPayableFacilities"
        | "AccountsPayableFacility"
        | "AccountsPayableForFacilities" => {
            "accounts_payable_for_property_plant_and_equipment_current"
        }

        "AccountsPayableRealEstate" => "accounts_payable_real_estate_current",
        "AccountsPayableSurveyingSpecific" => "accounts_payable_surveying_current",
        "AccountsPayableForSideLineBusinessCNS" => "accounts_payable_trade",
        "AccountsPayableOtherAndAccruedExpenses" => "accounts_payable_other_and_accrued_expenses",

        "AccountPaylableSubsidiariesAndAssociates"
        | "AccountsPayableAffiliatedCompanies"
        | "AccountsPayableConsignment"
        | "AccountsPayableDevelopmentBusinessAndOther"
        | "AccountsPayableDueToMemberMerchants"
        | "AccountsPayableForFranchisee"
        | "AccountsPayableFormSubsideiariesAndAssociateOther"
        | "AccountsPayableFromSubsidiariesAndAffiliatesOther"
        | "AccountsPayableFromSubsidiariesAndAssociatesOther"
        | "AccountsPayableInstallmentPurchase"
        | "AccountsPayableInstallmentPurchaseCL"
        | "AccountsPayableOtherFromSubsidiariesAndAssociates"
        | "AccountsPayableOtherToSubsidiariesAndAffiliates"
        | "AccountsPayablePurchaseOfCredit"
        | "AccountsPayableToAffiliatedCompanies"
        | "AccountsPayableToSubsidiariesAndAssociates"
        | "AccountsPayableOtherToSubsidiariesAndAffiliatesCLCL"
        | "AccountsPayableFromSubsidiariesAndAssociatesCL"
        | "AccountsPayableOfCollectionCL"
        | "AccountsPayableOnMarginCL"
        | "AccountsPayableOtherToAffiliatesCL"
        | "AccountsPayableOtherFromSubsidiariesAndAffiliatesCL"
        | "AccountsPayableToSubsidiariesAndAffiliatesCL" => "accounts_payable_current",

        _ => return None,
    };
    Some(field)
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_receivable_payable_100_alias(&concept.1)?;
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

fn is_reviewed_receivable_payable_100_item_name(item_name: &str) -> bool {
    matches!(
        item_name,
        "accounts_receivable_and_contract_assets"
            | "accounts_payable_for_property_plant_and_equipment_current"
            | "accounts_payable_other_and_accrued_expenses"
            | "accounts_payable_real_estate_current"
            | "accounts_payable_surveying_current"
    )
}

#[cfg(test)]
pub fn is_excluded_concept(local_name: &str) -> bool {
    reviewed_jppfs_receivable_payable_100_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    if is_reviewed_receivable_payable_100_item_name(item_name) {
        Some(StatementCategory::BalanceSheet)
    } else {
        base::classify_item_name(item_name)
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    if is_reviewed_receivable_payable_100_item_name(item_name) {
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

