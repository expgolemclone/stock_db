use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP working-capital aliases.
///
/// These aliases are intentionally scoped to the JPPFS taxonomy family. The
/// group normalizes legacy spelling variants and industry-specific local names
/// only when the account wording preserves a balance-sheet receivable or payable
/// meaning. "And other" receivables stay in the broader current-receivables
/// bucket, construction/equipment/factoring payables stay in their specific
/// current-liability buckets, and trade-marked payables stay in trade payables.
const REVIEWED_WORKING_CAPITAL_100: &[(&str, &str)] = &[
    (
        "AccountReceivableMedicalIncomeCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivable", "accounts_receivable_trade"),
    (
        "AccountsReceivableAdvisoryFeeCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableAssetsIFRS", "accounts_receivable_trade"),
    ("AccountsReceivableAssetsINS", "accounts_receivable_trade"),
    ("AccountsReceivableCA", "accounts_receivable_trade"),
    (
        "AccountsReceivableCareBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCareBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCareBusinessCAAssets",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCommissionCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedConstructionCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedOperation",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedOperationCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedSurveyCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDevelopmentBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDevelopmentBusinessAndOther",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDueFromFranchisedStores",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContracts",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsCNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromFranchisee",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSalesOnSideLineBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSideBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSideLineBusinessCNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSubsidiariesAndAffiliatesTrade",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableSideLineBusinessCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTenantsCA", "accounts_receivable_trade"),
    ("AccountsReceivableTrade1CA", "accounts_receivable_trade"),
    (
        "AccountsReceivableTradeAccountsReceivableTrade",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTradeCA", "accounts_receivable_trade"),
    ("AccountsReceivableTradeCAINV", "accounts_receivable_trade"),
    ("AccountsReceivableTradeNet", "accounts_receivable_trade"),
    (
        "AccountsReceivablerealEstateBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusinessAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableRealEstateBusinessAndOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
        "trade_and_other_current_receivables",
    ),
    (
        "ACcountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    ("AFactoringAccountPayable", "factoring_accounts_payable"),
    ("AFactoringAccountPayableCL", "factoring_accounts_payable"),
    ("AccountPayableFactoring", "factoring_accounts_payable"),
    (
        "AccountPayableForWorksInProgressAndOtherCL",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    ("AccountsPayable", "accounts_payable_current"),
    (
        "AccountsPayableAffiliatedCompanies",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableAffiliatedCompaniesCL",
        "accounts_payable_current",
    ),
    ("AccountsPayableCL", "accounts_payable_current"),
    ("AccountsPayableCLIFRS", "accounts_payable_current"),
    ("AccountsPayableCLINV", "accounts_payable_current"),
    ("AccountsPayableConsignment", "accounts_payable_current"),
    (
        "AccountsPayableDevelopmentBusinessAndOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableDevelopmentProjectsInProgressCNS",
        "accounts_payable_for_construction_contracts_current",
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
        "AccountsPayableForAffiliatedCompanyCL",
        "accounts_payable_current",
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
        "AccountsPayableForEquipmentLiabilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilitiesvCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AccountsPayableForFactoring", "factoring_accounts_payable"),
    (
        "AccountsPayableForFactoringCL",
        "factoring_accounts_payable",
    ),
    ("AccountsPayableForFranchisee", "accounts_payable_current"),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOtherCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesOther",
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
    (
        "AccountsPayableFromSubsidiariesAndAssociatesOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableInstallmentPurchaseCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOfBuyNoncurrentAssetsCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AccountsPayableOfCollectionCL", "accounts_payable_current"),
    ("AccountsPayableOnMarginCL", "accounts_payable_current"),
    ("AccountsPayableOtherBusinessCL", "accounts_payable_current"),
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
        "AccountsReceivableOtherBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherBusinessCAWAT",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromFactor",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromSubsidiariesAndAffiliates",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherMaterialsCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherNetCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherOfStorageAgencyCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivablePaidProvisionCA",
        "trade_and_other_current_receivables",
    ),
];

fn reviewed_jppfs_working_capital_100_alias(local_name: &str) -> Option<&'static str> {
    REVIEWED_WORKING_CAPITAL_100
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

    let field = reviewed_jppfs_working_capital_100_alias(&concept.1)?;
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
    reviewed_jppfs_working_capital_100_alias(local_name).is_none()
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

