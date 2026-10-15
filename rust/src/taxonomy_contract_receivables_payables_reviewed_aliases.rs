use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP contract-receivable and construction-payable aliases.
///
/// These aliases keep IFRS 15 / ASC 606 presentation semantics intact:
/// - receivables from contracts with customers are unconditional current claims,
///   not contract assets;
/// - construction-contract payable names preserve whether they are pure
///   construction payables, trade payables, or notes-and-accounts payables;
/// - advances received are not collapsed with contract liabilities or operating
///   deposits when the tag explicitly combines those obligations.
fn reviewed_jppfs_contract_receivable_payable_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Completed-construction receivables and receivables from contracts with
        // customers are current receivable claims. Do not collapse them into
        // `contract_assets`, because the wording indicates a receivable rather
        // than a conditional contract asset. "And other" names intentionally stay
        // in the broad current-receivables bucket, not pure trade receivables.
        "AccountsReceivableCompletedConstructionContractsCA"
        | "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsAReceivableFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS"
        | "AccountsReceivableReceivablesArisingFromContractsWithCustomersCurrentAssets" => {
            "trade_and_other_current_receivables"
        }

        // Pure construction-contract payables retain their narrower operating
        // liability scope.
        "AccountsPayableForConstructionContracts"
        | "AccountsPayableForConstructionContracts2CNS"
        | "AccountsPayableForConstructionContractsCL"
        | "AccountsPayableForConstructionContractsCNS" => {
            "accounts_payable_for_construction_contracts_current"
        }

        // These names explicitly include "other" payables. They cannot be
        // represented as pure construction or real-estate payables.
        "AccountPayableForWorksInProgressAndOtherCL"
        | "AccountsPayableForConstructionContractsAndOther"
        | "AccountsPayableForConstructionContractsAndOtherCL"
        | "AccountsPayableForConstructionContractsAndOtherCLCNS"
        | "AccountsPayableForConstructionContractsAndOtherCNS"
        | "AccountsPayableRealEstateAndOtherCL" => "accounts_payable_current",

        // Both components are operating trade payables.
        "AccountsPayableTradeAndAccountsPayableForConstructionContracts"
        | "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL" => {
            "accounts_payable_trade"
        }

        // Notes payable are present in the tag, so preserve the broader
        // notes-and-accounts payable scope.
        "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL"
        | "NotesPayableAccountsPayableForConstructionContractsAndOtherCL" => {
            "notes_and_accounts_payable_trade"
        }

        "AdvancesReceivedAndContractLiabilities"
        | "AdvancesReceivedAndContractLiabilitiesCL"
        | "AdvancesReceivedContractLiabilitiesCL"
        | "AdvancesReceivedOfContractLiabilities"
        | "AdvancesReceivedOnConstructionContractsInProgressAndContractLiabilities"
        | "AdvancesReceivedOnConstructionContractsInProgressContractLiabilitiesCL"
        | "AdvancesReceivedOnConstructionContractsInProgressContractLiabilitiesCLCNS" => {
            "advances_received_and_contract_liabilities_current"
        }

        "AdvancesReceivedOnUncompletedConstructionContractsAndOtherContractLiabilitiesCLCNS" => {
            "advances_received_and_other_contract_liabilities_current"
        }

        "AdvancesReceivedAndOperatingDepositsReceivedCL" => {
            "advances_received_and_operating_deposits_current"
        }

        "AdvancesReceivedOperatingDepositsReceivedAndContractLiabilitiesCL" => {
            "advances_received_operating_deposits_and_contract_liabilities_current"
        }

        "AdvancesReceivedOnUncompletedConstructionContractsAndDepositReceivedSellingBusinessCL" => {
            "advances_received_and_deposits_received_current"
        }

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

    let field = reviewed_jppfs_contract_receivable_payable_alias(&concept.1)?;
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
    reviewed_jppfs_contract_receivable_payable_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "trade_and_other_current_receivables"
        | "accounts_payable_for_construction_contracts_current"
        | "advances_received_and_contract_liabilities_current"
        | "advances_received_and_other_contract_liabilities_current"
        | "advances_received_and_operating_deposits_current"
        | "advances_received_operating_deposits_and_contract_liabilities_current"
        | "advances_received_and_deposits_received_current" => Some("bs"),
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
