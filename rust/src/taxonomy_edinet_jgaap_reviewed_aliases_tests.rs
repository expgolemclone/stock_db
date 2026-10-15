use crate::taxonomy::{
    TaxonomyConceptStatus, canonicalize_semantic_concept, concept_status, statement_for_concept,
};

fn edinet_jgaap_sibling(token: &str, local_name: &str) -> (String, String) {
    (
        format!("http://disclosure.edinet-fsa.go.jp/taxonomy/{token}/2024-11-01/{token}_cor"),
        local_name.to_string(),
    )
}

fn issuer_extension(local_name: &str) -> (String, String) {
    (
        "https://issuer.example.jp/taxonomy/2026/custom".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn edinet_jgaap_sibling_namespaces_use_reviewed_aliases() {
    for token in ["jplvh", "jpsps", "jpctl"] {
        for (local_name, expected_field, expected_statement) in [
            ("OtherCA", "other_current_assets", "bs"),
            ("InterestExpensesNOE", "interest_expenses", "pl"),
            ("SubtotalOpeCF", "subtotal_operating_cf", "cf"),
        ] {
            let concept = edinet_jgaap_sibling(token, local_name);
            let resolved = canonicalize_semantic_concept(&concept)
                .unwrap_or_else(|| panic!("{token}:{local_name} should resolve as reviewed"));

            assert_eq!(resolved.item_name, expected_field);
            assert_eq!(statement_for_concept(&concept), Some(expected_statement));
            assert!(matches!(
                concept_status(&concept),
                TaxonomyConceptStatus::Semantic(_)
            ));
        }
    }
}

#[test]
fn issuer_extensions_do_not_use_edinet_jgaap_reviewed_alias_fallback() {
    let concept = issuer_extension("OtherCA");

    assert!(canonicalize_semantic_concept(&concept).is_none());
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Extension
    ));
}
