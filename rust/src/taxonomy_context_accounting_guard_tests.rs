use crate::taxonomy::{
    TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept, concept_status,
    statement_for_concept,
};
use crate::types::ConceptKey;

fn jppfs(local_name: &str) -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn ifrs(local_name: &str) -> ConceptKey {
    (
        "https://xbrl.ifrs.org/taxonomy/2024-03-27/ifrs-full".to_string(),
        local_name.to_string(),
    )
}

fn issuer_extension(local_name: &str) -> ConceptKey {
    (
        "https://example.com/company-extension/2026".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn issuer_extension_must_not_fallback_to_standard_revenue() {
    let concept = issuer_extension("Revenue");

    assert!(canonicalize_semantic_concept(&concept).is_none());
    assert!(canonicalize_concept(&concept).is_none());
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Extension
    ));
}

#[test]
fn industry_revenue_tags_keep_accounting_meaning() {
    assert_eq!(
        canonicalize_semantic_concept(&jppfs("OrdinaryIncomeBNK")).map(|entry| entry.item_name),
        Some("revenue".to_string())
    );
    assert_eq!(
        statement_for_concept(&jppfs("OrdinaryIncomeBNK")),
        Some("pl")
    );

    assert_eq!(
        canonicalize_semantic_concept(&jppfs("OperatingIncomeINS")).map(|entry| entry.item_name),
        Some("revenue".to_string())
    );
    assert_eq!(
        statement_for_concept(&jppfs("OperatingIncomeINS")),
        Some("pl")
    );
}

#[test]
fn industry_suffixes_are_not_blanket_stripped() {
    assert_ne!(
        canonicalize_semantic_concept(&jppfs("OperatingRevenueBNK")).map(|entry| entry.item_name),
        Some("revenue".to_string())
    );
}

#[test]
fn ifrs_intangible_assets_scope_is_not_j_gaap_fixed_assets() {
    assert_eq!(
        canonicalize_semantic_concept(&jppfs("IntangibleAssets")).map(|entry| entry.item_name),
        Some("intangible_fixed_assets".to_string())
    );
    assert!(canonicalize_semantic_concept(&ifrs("IntangibleAssets")).is_none());
    let ifrs_concept = ifrs("IntangibleAssets");
    assert!(canonicalize_concept(&ifrs_concept).is_none());
    assert!(matches!(
        concept_status(&ifrs_concept),
        TaxonomyConceptStatus::Unknown
    ));
}

