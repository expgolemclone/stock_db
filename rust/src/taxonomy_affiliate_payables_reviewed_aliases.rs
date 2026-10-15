use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_affiliate_payables_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Trade payables to subsidiaries / associates retain the trade-payable
        // scope. The `TradeCL` suffix indicates an operating payable presented
        // within current liabilities, not a generic other payable.
        "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL" => "accounts_payable_trade",

        // Affiliate / subsidiary payables without a trade indicator are current
        // liability payables, but should not be collapsed into the narrower
        // trade-payables bucket.
        "AccountsPayableToAffiliatedCompanies"
        | "AccountsPayableOtherFromSubsidiariesAndAssociates" => "accounts_payable_current",

        _ => return None,
    };
    Some(field)
}

fn reviewed_jppfs_composite_payables_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // This composite label includes other accounts payable and accrued
        // expenses. Accrued expenses are accrued liabilities, not trade accounts
        // payable, so retain the combined current-liability scope instead of
        // folding the tag into a pure trade or generic accounts-payable bucket.
        "AccountsPayableOtherAndAccruedExpenses" => "accounts_payable_other_and_accrued_expenses",

        _ => return None,
    };
    Some(field)
}

fn reviewed_jppfs_combined_receivable_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // These labels explicitly contain both accounts receivable and contract
        // assets. A contract asset is conditional, while a receivable is an
        // unconditional right to consideration, so the combined presentation
        // must not be collapsed into the pure contract_assets bucket.
        "AccountReceivableTradeAndContractAssetsCA"
        | "AccountsReccivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableAndContractAssets"
        | "AccountsReceivableAndContractAssetsCA"
        | "AccountsReceivableAndContractAssetsTrade"
        | "AccountsReceivableCompletedOperationAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssets"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsContractAssetsCACNS"
        | "AccountsReceivableOtherAndContractAssetCA"
        | "AccountsReceivableTradeAndContractAssets"
        | "AccountsReceivableTradeAndContractAssetsAccountsReceivableTrade"
        | "AccountsReceivableTradeAndContractAssetsCA"
        | "AccountsReceivableTradeAndContractAssetsNet"
        | "AccountsReceivableTradeAndContractCA"
        | "AccountsReceivablesAndContractAssetsCA"
        | "AccountsreceivabletradeandcontractassetsCA" => "accounts_receivable_and_contract_assets",

        // These combined labels include notes receivable as well as accounts
        // receivable and contract assets. Keep the notes-and-accounts scope
        // rather than folding into accounts_receivable_and_contract_assets.
        "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssets"
        | "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssetsCA" => {
            "notes_and_accounts_receivable_trade_and_contract_assets"
        }

        _ => return None,
    };
    Some(field)
}

fn reviewed_jppfs_other_current_receivable_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Other current accounts receivable variants are still current
        // receivable assets, but they do not consistently carry a trade/customer
        // marker. Use the broad trade-and-other-current-receivables bucket
        // instead of forcing them into pure trade receivables.
        "AccountReceivableSubcontractorOtherCA"
        | "AccountsReceivableDevelopmentProjectsCACNS"
        | "AccountsReceivableForRentCA"
        | "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA"
        | "AccountsReceivableLeaseCA"
        | "AccountsReceivableLeaseCALEA"
        | "AccountsReceivableLeasingCA"
        | "AccountsReceivableOtherBusinessCA"
        | "AccountsReceivableOtherCA"
        | "AccountsReceivableOtherFromFactor"
        | "AccountsReceivableOtherMaterialsCA"
        | "AccountsReceivableOtherNetCA"
        | "AccountsReceivableOtherOfStorageAgencyCA"
        | "AccountsReceivablePaidProvisionCA"
        | "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA" => {
            "trade_and_other_current_receivables"
        }

        _ => return None,
    };
    Some(field)
}

fn reviewed_jppfs_alias(local_name: &str) -> Option<&'static str> {
    reviewed_jppfs_affiliate_payables_alias(local_name)
        .or_else(|| reviewed_jppfs_composite_payables_alias(local_name))
        .or_else(|| reviewed_jppfs_combined_receivable_alias(local_name))
        .or_else(|| reviewed_jppfs_other_current_receivable_alias(local_name))
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !matches!(
        base::classify_namespace_uri(&concept.0),
        Some(TaxonomyFamily::EdinetJppfs)
    ) {
        return None;
    }
    let field = reviewed_jppfs_alias(&concept.1)?;
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
    reviewed_jppfs_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "accounts_payable_other_and_accrued_expenses"
        | "accounts_receivable_and_contract_assets"
        | "notes_and_accounts_receivable_trade_and_contract_assets" => {
            Some(StatementCategory::BalanceSheet)
        }
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "accounts_payable_other_and_accrued_expenses"
        | "accounts_receivable_and_contract_assets"
        | "notes_and_accounts_receivable_trade_and_contract_assets" => Some("bs"),
        _ => base::statement_for_item_name(item_name),
    }
}

