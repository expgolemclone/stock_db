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

fn extension(local_name: &str) -> (String, String) {
    (
        "https://filer.example.co.jp/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

fn assert_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
    expected_statement: &str,
) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve in the active 20260617 stack"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert_eq!(statement_for_concept(&concept), Some(expected_statement));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn active_20260617_stack_preserves_bs_receivable_payable_scope() {
    for (local_name, expected_field) in [
        (
            "AccountsReceivableAdvisoryFeeCA",
            "accounts_receivable_trade",
        ),
        (
            "AccountsReceivableDevelopmentBusinessAndOther",
            "trade_and_other_current_receivables",
        ),
        ("AccountsPayableConsignment", "accounts_payable_current"),
    ] {
        assert_alias(
            local_name,
            expected_field,
            StatementCategory::BalanceSheet,
            "bs",
        );
    }
}

#[test]
fn active_20260617_stack_preserves_pl_suffix_scope() {
    for (local_name, expected_field) in [
        ("CashBackIncomeNOI", "non_operating_income"),
        ("CashOverAndShortNOE", "non_operating_expenses"),
    ] {
        assert_alias(
            local_name,
            expected_field,
            StatementCategory::ProfitAndLoss,
            "pl",
        );
    }
}

#[test]
fn active_20260617_aliases_are_namespace_gated() {
    for local_name in [
        "AccountsReceivableAdvisoryFeeCA",
        "AccountsPayableConsignment",
        "CashBackIncomeNOI",
    ] {
        assert_ne!(
            canonicalize_concept(&extension(local_name)).map(|entry| entry.item_name),
            canonicalize_concept(&jppfs(local_name)).map(|entry| entry.item_name)
        );
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
