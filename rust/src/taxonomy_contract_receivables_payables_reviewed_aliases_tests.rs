use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
    concept_status, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> (String, String) {
    (
        "https://filer.example.co.jp/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

fn assert_bs_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
}

#[test]
fn contract_receivable_aliases_remain_current_receivables_not_contract_assets() {
    for local_name in [
        "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA",
        "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA",
        "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA",
        "AccountsReceivableReceivablesArisingFromContractsWithCustomersCurrentAssets",
    ] {
        assert_bs_alias(local_name, "trade_and_other_current_receivables");
        let item_name = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item_name, "contract_assets");
        assert_ne!(item_name, "accounts_receivable_and_contract_assets");
    }
}

#[test]
fn construction_contract_payable_aliases_keep_current_payable_scope() {
    for local_name in [
        "AccountsPayableForConstructionContracts",
        "AccountsPayableForConstructionContracts2CNS",
        "AccountsPayableForConstructionContractsCL",
        "AccountsPayableForConstructionContractsCNS",
    ] {
        assert_bs_alias(
            local_name,
            "accounts_payable_for_construction_contracts_current",
        );
    }

    for local_name in [
        "AccountPayableForWorksInProgressAndOtherCL",
        "AccountsPayableForConstructionContractsAndOther",
        "AccountsPayableForConstructionContractsAndOtherCL",
        "AccountsPayableForConstructionContractsAndOtherCLCNS",
        "AccountsPayableForConstructionContractsAndOtherCNS",
        "AccountsPayableRealEstateAndOtherCL",
    ] {
        assert_bs_alias(local_name, "accounts_payable_current");
    }

    for local_name in [
        "AccountsPayableTradeAndAccountsPayableForConstructionContracts",
        "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL",
    ] {
        assert_bs_alias(local_name, "accounts_payable_trade");
    }

    for local_name in [
        "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL",
        "NotesPayableAccountsPayableForConstructionContractsAndOtherCL",
    ] {
        assert_bs_alias(local_name, "notes_and_accounts_payable_trade");
    }
}

#[test]
fn combined_advance_obligations_preserve_each_accounting_scope() {
    assert_bs_alias(
        "AdvancesReceivedAndContractLiabilitiesCL",
        "advances_received_and_contract_liabilities_current",
    );
    assert_bs_alias(
        "AdvancesReceivedOnUncompletedConstructionContractsAndOtherContractLiabilitiesCLCNS",
        "advances_received_and_other_contract_liabilities_current",
    );
    assert_bs_alias(
        "AdvancesReceivedAndOperatingDepositsReceivedCL",
        "advances_received_and_operating_deposits_current",
    );
    assert_bs_alias(
        "AdvancesReceivedOperatingDepositsReceivedAndContractLiabilitiesCL",
        "advances_received_operating_deposits_and_contract_liabilities_current",
    );
    assert_bs_alias(
        "AdvancesReceivedOnUncompletedConstructionContractsAndDepositReceivedSellingBusinessCL",
        "advances_received_and_deposits_received_current",
    );
}

#[test]
fn contract_receivable_payable_aliases_are_namespace_gated() {
    for local_name in [
        "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA",
        "AccountsPayableForConstructionContractsAndOther",
        "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL",
    ] {
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
