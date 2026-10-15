use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed 2026-06-17 JPPFS aliases observed in historical EDINET filings.
///
/// This batch groups 100 old or issuer/industry-spelled tags only when their
/// local names carry the same accounting meaning as an existing canonical
/// bucket. Component tags are kept in component buckets, not widened to assets,
/// liabilities, revenue, or expense totals.
const REVIEWED_ALIASES_20260617: &[(&str, &str)] = &[
    (
        "ACcountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    ("AccountsPayable", "accounts_payable_current"),
    (
        "AccountsPayableAffiliatedCompanies",
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
    ("AccountsPayableFacilities", "accounts_payable_current"),
    ("AccountsPayableFacility", "accounts_payable_current"),
    ("AccountsPayableForFacilities", "accounts_payable_current"),
    ("AccountsPayableForFranchisee", "accounts_payable_current"),
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
        "AccountsPayableOtherFromSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    (
        "AccountsPayablePurchaseOfCredit",
        "accounts_payable_current",
    ),
    ("AccountsPayableRealEstate", "accounts_payable_current"),
    (
        "AccountsPayableSurveyingSpecific",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableToSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableForSideLineBusinessCNS",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOther",
        "accounts_payable_current",
    ),
    ("AccountsReceivable", "trade_and_other_current_receivables"),
    (
        "AccountsReceivableCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableAssetsIFRS",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableAssetsINS",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableCareBusiness",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableCareBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableCareBusinessCAAssets",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableCommissionCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableDevelopmentBusiness",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableDevelopmentBusinessAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableDueFromFranchisedStores",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromFranchisee",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromSalesOnSideLineBusiness",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromSideBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromSideLineBusinessCNS",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableInstallment",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableRealEstateBusiness",
        "trade_and_other_current_receivables",
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
        "AccountsReceivableRealEstateBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableSideLineBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableTenantsCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AgencyAccountsReceivableAssetsINS",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableAdvisoryFeeCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableMedicalIncomeCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccounAccountsReceivableFromCompletedConstructionContractstsReceivableFromCompletedConstructionContracts",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableAccountsReceivableFromCompletedConstructionContractsAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
        "trade_and_other_current_receivables",
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
        "AccountsReceivableFromCompletedConstructionContracts",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAReceivableFromContractsWithCustomersCACNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAccountsReceivableRealEstateBusinessAndAccountsReceivableTradeCA",
        "accounts_receivable_trade",
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
        "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS",
        "trade_and_other_current_receivables",
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
        "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsNotesReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
        "notes_and_accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSubsidiariesAndAffiliatesTrade",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTrade1CA", "accounts_receivable_trade"),
    (
        "AccountsReceivableTradeAccountsReceivableTrade",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableTradeAndAccountsReceivableFromCompletedConstructionContracts",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableTradeAndAccountsReceivableFromCompletedConstructionContractsCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTradeCA", "accounts_receivable_trade"),
    ("AccountsReceivableTradeCAINV", "accounts_receivable_trade"),
    ("AccountsReceivableTradeNet", "accounts_receivable_trade"),
    (
        "AccountsReceivableTradeReceivablesFromContractsWithCustomers",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivablerealEstateBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableAndContractAssets",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableAndContractAssetsCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableAndContractAssetsTrade",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableCompletedOperationAndContractAssetsCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssets",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableOtherAndContractAssetCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableTradeAndContractAssets",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivableTradeAndContractAssetsCA",
        "accounts_receivable_and_contract_assets",
    ),
    (
        "AccountsReceivablesAndContractAssetsCA",
        "accounts_receivable_and_contract_assets",
    ),
    ("AdvancesReceived", "advances_received_current"),
    (
        "AdvancesReceivedAndContractLiabilities",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedAndOperatingDepositsReceivedCL",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOfContractLiabilities",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnConstructionConstractsInprogress",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnConstructionContractsInProgress",
        "advances_received_current",
    ),
    (
        "AdvancesReceivedOnConstructionContractsInProgressAndContractLiabilities",
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
        "AdvancesReceivedOnUncompletedContracts",
        "advances_received_current",
    ),
];

#[cfg(test)]
pub fn reviewed_alias_count_20260617() -> usize {
    REVIEWED_ALIASES_20260617.len()
}

fn reviewed_20260617_alias(local_name: &str) -> Option<&'static str> {
    REVIEWED_ALIASES_20260617
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

    let field = reviewed_20260617_alias(&concept.1)?;
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

fn is_reviewed_20260617_item_name(item_name: &str) -> bool {
    matches!(
        item_name,
        "accounts_receivable_and_contract_assets" | "advances_received_current"
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
    reviewed_20260617_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    if is_reviewed_20260617_item_name(item_name) {
        Some(StatementCategory::BalanceSheet)
    } else {
        base::classify_item_name(item_name)
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    if is_reviewed_20260617_item_name(item_name) {
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

