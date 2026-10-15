use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn jpigp(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2020-11-01/jpigp_cor".to_string(),
        local_name.to_string(),
    )
}

fn ifrs(local_name: &str) -> (String, String) {
    (
        "http://www.ifrs.org/taxonomy/2024".to_string(),
        local_name.to_string(),
    )
}

fn us_gaap(local_name: &str) -> (String, String) {
    (
        "http://fasb.org/us-gaap/2024".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> (String, String) {
    (
        "http://example.com/taxonomy/company/2026/custom".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn paid_in_capital_aliases_keep_standard_specific_equity_components() {
    for (concept, expected_field) in [
        (jppfs("CapitalSurplus"), "capital_surplus"),
        (jpigp("SharePremiumIFRS"), "share_premium"),
        (ifrs("SharePremium"), "share_premium"),
        (
            us_gaap("AdditionalPaidInCapital"),
            "additional_paid_in_capital",
        ),
        (
            us_gaap("AdditionalPaidInCapitalUSGAAP"),
            "additional_paid_in_capital",
        ),
    ] {
        let resolved = canonicalize_semantic_concept(&concept)
            .unwrap_or_else(|| panic!("{concept:?} should resolve as a reviewed alias"));
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
}

#[test]
fn share_premium_and_additional_paid_in_capital_are_not_jp_gaap_capital_surplus() {
    let capital_surplus = canonicalize_concept(&jppfs("CapitalSurplus"))
        .unwrap()
        .item_name;

    for concept in [
        jpigp("SharePremiumIFRS"),
        ifrs("SharePremium"),
        us_gaap("AdditionalPaidInCapital"),
        us_gaap("AdditionalPaidInCapitalUSGAAP"),
    ] {
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{concept:?} should resolve as a reviewed alias"));
        assert_ne!(resolved.item_name, capital_surplus);
    }
}

#[test]
fn legal_capital_surplus_keeps_statutory_component_scope() {
    let capital_surplus = canonicalize_concept(&jppfs("CapitalSurplus"))
        .unwrap()
        .item_name;
    let legal_capital_surplus = canonicalize_concept(&jppfs("LegalCapitalSurplus"))
        .unwrap()
        .item_name;

    assert_eq!(capital_surplus, "capital_surplus");
    assert_eq!(legal_capital_surplus, "legal_capital_surplus");
    assert_ne!(legal_capital_surplus, capital_surplus);
}

#[test]
fn extension_namespace_does_not_use_paid_in_capital_aliases() {
    let concept = extension("SharePremium");

    assert!(canonicalize_semantic_concept(&concept).is_none());
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Extension
    ));
}
