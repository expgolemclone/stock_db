use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_semantic_concept, classify_item_name,
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

#[test]
fn active_taxonomy_module_includes_reviewed_alias_layers() {
    for (local_name, expected_field, expected_category, expected_statement) in [
        (
            "CurrentAssetsForIncidentalBusinessesCAGAS",
            "current_assets",
            StatementCategory::BalanceSheet,
            "bs",
        ),
        (
            "AAANOI",
            "non_operating_income",
            StatementCategory::ProfitAndLoss,
            "pl",
        ),
        (
            "OperatingExpensesTransportationOERWY",
            "operating_expenses",
            StatementCategory::ProfitAndLoss,
            "pl",
        ),
        (
            "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA",
            "trade_and_other_current_receivables",
            StatementCategory::BalanceSheet,
            "bs",
        ),
    ] {
        let concept = jppfs(local_name);
        let resolved = canonicalize_semantic_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve in the active taxonomy stack"));

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
    }
}

#[test]
fn active_taxonomy_module_keeps_reviewed_aliases_namespace_gated() {
    for local_name in [
        "CurrentAssetsForIncidentalBusinessesCAGAS",
        "AAANOI",
        "OperatingExpensesTransportationOERWY",
        "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA",
    ] {
        assert!(canonicalize_semantic_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
