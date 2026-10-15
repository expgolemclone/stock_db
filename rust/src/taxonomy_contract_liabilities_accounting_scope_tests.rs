use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, is_excluded_concept, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn contract_liabilities_are_liability_aliases_not_contract_assets() {
    let concept = jppfs("ContractLiabilities");
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("ContractLiabilities should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "contract_liabilities");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(!is_excluded_concept("ContractLiabilities"));

    for asset_like in [
        "ContractAssets",
        "ContractAssetsCA",
        "AccountsReceivableAndContractAssets",
        "AccountsReceivableAndContractAssetsCA",
    ] {
        let asset_field = canonicalize_concept(&jppfs(asset_like))
            .unwrap_or_else(|| panic!("{asset_like} should resolve as a reviewed asset alias"))
            .item_name;
        assert_ne!(
            resolved.item_name, asset_field,
            "contract liabilities must not collapse into receivable/contract-asset fields"
        );
    }

    assert_ne!(
        resolved.item_name, "advance_payments_trade",
        "contract liabilities are obligations to transfer goods or services, not an advance-payment asset"
    );
}
