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

#[test]
fn reviewed_bank_operating_revenue_is_top_line_revenue_not_component() {
    let local_name = "OperatingRevenueBNK";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "revenue");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::ProfitAndLoss)
    );
    assert_ne!(resolved.item_name, "other_operating_revenue");
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn reviewed_current_payable_aliases_preserve_current_liability_scope() {
    for local_name in [
        "ACcountsPayableInstallmentPurchase",
        "AccountsPayableCLINV",
        "AccountsPayableInstallmentPurchase",
        "AccountsPayableInstallmentPurchaseCL",
        "AccountsPayableOnMarginCL",
        "AccountsPayableOtherBusinessCL",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

        assert_eq!(resolved.item_name, "accounts_payable_current");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_trade_payable_aliases_are_trade_payables() {
    for local_name in ["AccountsPayableForSideLineBusinessCNS"] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

        assert_eq!(resolved.item_name, "accounts_payable_trade");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_operating_specific_payable_uses_broad_current_payable_scope() {
    let local_name = "AccountsPayableOperatingSpecific";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "accounts_payable_current");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_ne!(resolved.item_name, "accounts_payable_trade");
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn reviewed_completed_construction_receivable_typo_alias_uses_trade_receivables() {
    let local_name = "AccounAccountsReceivableFromCompletedConstructionContractstsReceivableFromCompletedConstructionContracts";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "accounts_receivable_trade");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_ne!(resolved.item_name, "trade_and_other_current_receivables");
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn reviewed_singular_account_receivable_contract_asset_alias_preserves_combined_scope() {
    let local_name = "AccountReceivableTradeAndContractAssetsCA";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(
        resolved.item_name,
        "accounts_receivable_and_contract_assets"
    );
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_ne!(resolved.item_name, "accounts_receivable_trade");
    assert_ne!(resolved.item_name, "contract_assets");
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn composite_payables_with_accrued_expenses_preserve_composite_liability_scope() {
    let local_name = "AccountsPayableOtherAndAccruedExpenses";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(
        resolved.item_name,
        "accounts_payable_other_and_accrued_expenses"
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
    assert_ne!(resolved.item_name, "accounts_payable_trade");
    assert_ne!(resolved.item_name, "accounts_payable_current");
    assert!(!is_excluded_concept(local_name));
}
