use crate::taxonomy::{
    self, StatementCategory, TaxonomyConceptStatus, canonicalize_concept,
    canonicalize_semantic_concept, classify_item_name, concept_status, statement_for_concept,
};
use crate::types::ConceptKey;

fn jppfs(local_name: &str) -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn jplvh(local_name: &str) -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jplvh/2016-03-31/jplvh_cor".to_string(),
        local_name.to_string(),
    )
}

fn jpctl(local_name: &str) -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jpctl/2014-03-31/jpctl_cor".to_string(),
        local_name.to_string(),
    )
}

fn ifrs(local_name: &str) -> ConceptKey {
    (
        "https://xbrl.ifrs.org/taxonomy/2024-03-27/ifrs-full".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> ConceptKey {
    (
        "https://issuer.example.jp/taxonomy/2026/custom".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn contract_liabilities_are_reviewed_as_balance_sheet_liabilities() {
    let concept = jppfs("ContractLiabilities");
    let resolved = canonicalize_semantic_concept(&concept)
        .expect("ContractLiabilities should resolve as a reviewed semantic alias");

    assert_eq!(resolved.item_name, "contract_liabilities");
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(concept_status(&concept), TaxonomyConceptStatus::Semantic(_)));
}

#[test]
fn contract_liabilities_alias_covers_edinet_jgaap_variant_namespaces() {
    for concept in [jplvh("ContractLiabilities"), jpctl("ContractLiabilities")] {
        let resolved = canonicalize_semantic_concept(&concept)
            .expect("J-GAAP variant namespace should resolve the reviewed liability alias");
        assert_eq!(resolved.item_name, "contract_liabilities");
        assert_eq!(statement_for_concept(&concept), Some("bs"));
    }
}

#[test]
fn issuer_extensions_do_not_use_contract_liabilities_local_name_fallback() {
    let concept = extension("ContractLiabilities");

    assert!(canonicalize_semantic_concept(&concept).is_none());
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Extension | TaxonomyConceptStatus::Unknown
    ));
}

#[test]
fn insurance_corporate_bonds_assets_do_not_resolve_to_bonds_payable() {
    let concept = jppfs("CorporateBondsAssetsINS");
    let resolved = canonicalize_semantic_concept(&concept)
        .expect("insurance corporate bond assets should resolve as a reviewed semantic alias");

    assert_eq!(
        resolved.item_name,
        "corporate_bonds_investment_securities_assets"
    );
    assert_ne!(resolved.item_name, "bonds_payable");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(concept_status(&concept), TaxonomyConceptStatus::Semantic(_)));
}

#[test]
fn insurance_corporate_bonds_asset_alias_is_namespace_gated() {
    let concept = extension("CorporateBondsAssetsINS");

    assert!(canonicalize_concept(&concept).is_none());
    assert!(matches!(concept_status(&concept), TaxonomyConceptStatus::Extension));
}

#[test]
fn identity_candidates_do_not_become_semantic_mappings() {
    let concept = jppfs("AccountsPayableOther");

    assert!(
        taxonomy::canonicalize_semantic_concept(&concept).is_none(),
        "identity candidates must not enter reviewed semantic mappings"
    );

    assert!(matches!(
        taxonomy::concept_status(&concept),
        TaxonomyConceptStatus::Unknown
    ));
}

#[test]
fn extension_namespaces_do_not_use_local_name_fallbacks() {
    let concept = extension("Revenue");

    assert!(
        taxonomy::canonicalize_concept(&concept).is_none(),
        "company-extension local names must not be collapsed onto standard aliases"
    );
    assert!(matches!(taxonomy::concept_status(&concept), TaxonomyConceptStatus::Extension));
}

#[test]
fn cross_standard_intangible_assets_scope_stays_unreviewed() {
    let concept = ifrs("IntangibleAssets");

    assert!(
        taxonomy::canonicalize_semantic_concept(&concept).is_none(),
        "IFRS IntangibleAssets must not be merged into the J-GAAP intangible fixed asset bucket"
    );

    assert!(matches!(
        taxonomy::concept_status(&concept),
        TaxonomyConceptStatus::Unknown
    ));
}

#[test]
fn reviewed_jppfs_aliases_keep_statement_side_and_context() {
    for (local, expected_field, expected_statement) in [
        ("OtherCA", "other_current_assets", "bs"),
        ("OtherCL", "other_current_liabilities", "bs"),
        ("TreasuryStock", "treasury_stock", "bs"),
        ("CommissionFeeNOE", "commission_fee_non_operating_expenses", "pl"),
        ("SubtotalOpeCF", "subtotal_operating_cf", "cf"),
    ] {
        let concept = jppfs(local);
        let canonical = taxonomy::canonicalize_semantic_concept(&concept)
            .unwrap_or_else(|| panic!("{local} must be a reviewed semantic alias"));

        assert_eq!(canonical.item_name, expected_field, "{local}");
        assert_eq!(
            taxonomy::statement_for_item_name(&canonical.item_name),
            Some(expected_statement),
            "{local} must stay on its reviewed statement side"
        );
    }
}

#[test]
fn cf_role_suffix_aliases_do_not_collapse_to_related_balance_sheet_accounts() {
    let cf_concept = jppfs("IncreaseDecreaseInOtherCurrentLiabilitiesOpeCF");
    let bs_concept = jppfs("OtherCL");

    let cf = taxonomy::canonicalize_semantic_concept(&cf_concept).unwrap();
    let bs = taxonomy::canonicalize_semantic_concept(&bs_concept).unwrap();

    assert_eq!(
        cf.item_name,
        "increase_decrease_in_other_current_liabilities_operating_cf"
    );
    assert_eq!(taxonomy::statement_for_item_name(&cf.item_name), Some("cf"));
    assert_eq!(bs.item_name, "other_current_liabilities");
    assert_eq!(taxonomy::statement_for_item_name(&bs.item_name), Some("bs"));
    assert_ne!(
        cf.item_name, bs.item_name,
        "operating-CF movement tags must not collapse into the related BS account"
    );
}

