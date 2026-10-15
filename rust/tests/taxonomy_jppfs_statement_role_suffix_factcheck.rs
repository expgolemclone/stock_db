#![allow(dead_code, unused_imports)]

#[path = "../src/types.rs"]
mod types;

#[path = "../src/taxonomy_jppfs_statement_role_suffix_reviewed_aliases.rs"]
mod taxonomy;

use taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, statement_for_concept,
};

fn jppfs(local: &str) -> types::ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2026-11-01/jppfs_cor".to_string(),
        local.to_string(),
    )
}

fn extension(local: &str) -> types::ConceptKey {
    (
        "https://issuer.example.jp/taxonomy/2026".to_string(),
        local.to_string(),
    )
}

#[test]
fn consolidated_statement_role_suffixes_are_available_from_integration_tests() {
    for (local_name, expected_field) in [
        ("CashBackIncomeNOI", "non_operating_income"),
        ("CashOverAndShortNOE", "non_operating_expenses"),
        ("CashBeneficiaryEI", "extraordinary_income"),
        ("CasualtyLossEL", "extraordinary_loss"),
    ] {
        let concept = jppfs(local_name);
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed PL suffix alias"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::ProfitAndLoss)
        );
        assert_eq!(statement_for_concept(&concept), Some("pl"));
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
    }
}

#[test]
fn consolidated_statement_role_suffixes_are_namespace_gated() {
    for local_name in [
        "CashBackIncomeNOI",
        "CashOverAndShortNOE",
        "CashBeneficiaryEI",
        "CasualtyLossEL",
    ] {
        assert!(canonicalize_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}

#[test]
fn cash_flow_suffixes_are_not_classified_as_pl_suffix_aliases() {
    for (local_name, rejected_field) in [
        ("CashlessReturnIncomeOpeCF", "non_operating_income"),
        (
            "ConstructionRelatedCompensationForDamagesOpeCF",
            "extraordinary_loss",
        ),
        ("ContractCancellationRevenueOpeCF", "extraordinary_income"),
    ] {
        let resolved = canonicalize_semantic_concept(&jppfs(local_name));
        assert_ne!(
            resolved
                .as_ref()
                .map(|canonical| canonical.item_name.as_str()),
            Some(rejected_field),
            "{local_name} should not resolve to PL-only alias {rejected_field}"
        );
    }
}
