use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
    concept_status, is_excluded_concept, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn assert_balance_sheet_semantic_alias(local_name: &str, expected_field: &str) {
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(
        resolved.item_name, expected_field,
        "{local_name} must preserve its presentation scope"
    );
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&jppfs(local_name)), Some("bs"));
    assert!(matches!(
        concept_status(&jppfs(local_name)),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should be removed from excluded_concepts once reviewed"
    );
}

#[test]
fn combined_receivable_contract_asset_aliases_preserve_combined_scope() {
    for local_name in [
        "AccountReceivableTradeAndContractAssetsCA",
        "AccountsReccivableFromCompletedConstructionContractsAndContractAssetsCA",
        "AccountsReceivableAndContractAssets",
        "AccountsReceivableAndContractAssetsCA",
        "AccountsReceivableAndContractAssetsTrade",
        "AccountsReceivableCompletedOperationAndContractAssetsCA",
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA",
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssets",
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA",
        "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS",
        "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA",
        "AccountsReceivableFromCompletedConstructionContractsContractAssetsCACNS",
        "AccountsReceivableOtherAndContractAssetCA",
        "AccountsReceivableTradeAndContractAssets",
        "AccountsReceivableTradeAndContractAssetsAccountsReceivableTrade",
        "AccountsReceivableTradeAndContractAssetsCA",
        "AccountsReceivableTradeAndContractAssetsNet",
        "AccountsReceivableTradeAndContractCA",
        "AccountsReceivablesAndContractAssetsCA",
        "AccountsreceivabletradeandcontractassetsCA",
    ] {
        assert_balance_sheet_semantic_alias(local_name, "accounts_receivable_and_contract_assets");
        let item_name = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item_name, "accounts_receivable_trade");
        assert_ne!(item_name, "contract_assets");
    }
}

#[test]
fn combined_notes_and_accounts_receivable_contract_asset_aliases_keep_notes_scope() {
    for local_name in [
        "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssets",
        "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssetsCA",
    ] {
        assert_balance_sheet_semantic_alias(
            local_name,
            "notes_and_accounts_receivable_trade_and_contract_assets",
        );
        let item_name = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item_name, "accounts_receivable_and_contract_assets");
        assert_ne!(item_name, "contract_assets");
    }
}

#[test]
fn pure_contract_asset_aliases_remain_contract_assets() {
    for local_name in ["ContractAssets", "ContractAssetsCA"] {
        assert_balance_sheet_semantic_alias(local_name, "contract_assets");
        assert_ne!(
            canonicalize_concept(&jppfs(local_name)).unwrap().item_name,
            "accounts_receivable_and_contract_assets"
        );
    }
}
