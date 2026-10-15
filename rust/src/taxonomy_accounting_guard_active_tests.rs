use crate::taxonomy::{
    TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept, concept_status,
    statement_for_concept,
};
use crate::types::ConceptKey;

fn jppfs(local: &str) -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2026-11-01/jppfs_cor".to_string(),
        local.to_string(),
    )
}

fn ifrs(local: &str) -> ConceptKey {
    (
        "https://xbrl.ifrs.org/taxonomy/2024-03-27/ifrs-full".to_string(),
        local.to_string(),
    )
}

fn us_gaap(local: &str) -> ConceptKey {
    (
        "https://fasb.org/us-gaap/2024".to_string(),
        local.to_string(),
    )
}

fn extension(local: &str) -> ConceptKey {
    (
        "https://issuer.example.jp/taxonomy/2026".to_string(),
        local.to_string(),
    )
}

#[test]
fn active_taxonomy_keeps_confirmed_semantic_mappings_namespace_gated() {
    let concept = extension("Revenue");

    assert!(canonicalize_semantic_concept(&concept).is_none());
    assert!(canonicalize_concept(&concept).is_none());
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Extension
    ));
    assert_eq!(statement_for_concept(&concept), None);
}

#[test]
fn active_taxonomy_does_not_merge_ifrs_or_us_gaap_intangibles_into_j_gaap_fixed_assets() {
    for concept in [ifrs("IntangibleAssets"), us_gaap("IntangibleAssets")] {
        assert!(canonicalize_semantic_concept(&concept).is_none());
        assert_ne!(
            canonicalize_concept(&concept)
                .as_ref()
                .map(|canonical| canonical.item_name.as_str()),
            Some("intangible_fixed_assets"),
            "IntangibleAssets outside J-GAAP must not be collapsed into the J-GAAP fixed-asset bucket"
        );
        assert!(!matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
    }
}

#[test]
fn active_taxonomy_preserves_specific_pl_accounts_before_generic_section_buckets() {
    for (local_name, expected_field) in [
        ("InterestIncomeNOI", "interest_income"),
        ("InterestExpensesNOE", "interest_expenses"),
        ("ImpairmentLossEL", "impairment_loss"),
    ] {
        let concept = jppfs(local_name);
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed PL account"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(statement_for_concept(&concept), Some("pl"));
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
    }
}

#[test]
fn active_taxonomy_uses_j_gaap_pl_suffix_section_only_for_reviewed_generic_aliases() {
    let concept = jppfs("AdjustmentReceivedNOI");
    let resolved = canonicalize_concept(&concept)
        .expect("reviewed NOI alias should resolve to the non-operating income bucket");

    assert_eq!(resolved.item_name, "non_operating_income");
    assert_eq!(statement_for_concept(&concept), Some("pl"));
    assert!(canonicalize_concept(&extension("AdjustmentReceivedNOI")).is_none());
}
