use std::collections::HashSet;

use crate::taxonomy::{
    REVIEWED_WORKING_CAPITAL_ALIAS_COUNT, StatementCategory, TaxonomyConceptStatus,
    canonicalize_semantic_concept, classify_item_name, concept_status,
    reviewed_working_capital_aliases_for_tests, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn company_extension(local_name: &str) -> (String, String) {
    (
        "http://example.com/taxonomy/company/2026/custom".to_string(),
        local_name.to_string(),
    )
}

fn assert_balance_sheet_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
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
fn working_capital_review_covers_exactly_75_unambiguous_aliases() {
    let aliases = reviewed_working_capital_aliases_for_tests();
    assert_eq!(REVIEWED_WORKING_CAPITAL_ALIAS_COUNT, 75);
    assert_eq!(aliases.len(), 75);

    let unique_local_names = aliases
        .iter()
        .map(|(local_name, _field)| *local_name)
        .collect::<HashSet<_>>();
    assert_eq!(unique_local_names.len(), 75);
}

#[test]
fn working_capital_aliases_preserve_reviewed_balance_sheet_scope() {
    for (local_name, expected_field) in reviewed_working_capital_aliases_for_tests() {
        assert_balance_sheet_alias(local_name, expected_field);
    }
}

#[test]
fn narrower_payable_and_advance_scopes_do_not_collapse_to_generic_accounts_payable() {
    for (local_name, expected_field) in reviewed_working_capital_aliases_for_tests() {
        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();

        match *expected_field {
            "accounts_payable_current" | "accounts_payable_trade" => {
                assert_eq!(resolved.item_name, *expected_field);
            }
            _ => {
                assert_ne!(resolved.item_name, "accounts_payable_current");
                assert_ne!(resolved.item_name, "accounts_payable_trade");
            }
        }
    }
}

#[test]
fn trade_payable_aliases_do_not_collapse_to_broad_current_payables() {
    for local_name in [
        "AccountPayableTradeForFranchisedStoreCL",
        "AccountsPayableTrade",
        "AccountsPayableTradeCL",
        "AccountsPayableTradeLiabilitiesIFRS",
    ] {
        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        assert_eq!(resolved.item_name, "accounts_payable_trade");
        assert_ne!(resolved.item_name, "accounts_payable_current");
    }
}

#[test]
fn working_capital_aliases_are_namespace_gated() {
    for (local_name, _expected_field) in reviewed_working_capital_aliases_for_tests().iter().take(8)
    {
        let concept = company_extension(local_name);
        assert!(canonicalize_semantic_concept(&concept).is_none());
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Extension
        ));
    }
}
