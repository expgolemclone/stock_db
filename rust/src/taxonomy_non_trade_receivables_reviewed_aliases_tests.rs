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

fn assert_non_trade_current_receivable_alias(local_name: &str) {
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "trade_and_other_current_receivables");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&jppfs(local_name)), Some("bs"));
    assert_ne!(resolved.item_name, "accounts_receivable_trade");
    assert!(matches!(
        concept_status(&jppfs(local_name)),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should be removed from excluded_concepts once reviewed"
    );
}

#[test]
fn current_income_receivable_aliases_keep_non_trade_current_scope() {
    for local_name in [
        "AccountsReceivableDividendsCA",
        "AccountReceivableMonetizationOfReceivableCA",
        "AccountsReceivableMonetizationOfReceivableCA",
        "AccruedDividendCA",
        "AccruedRentIncomeCA",
        "AccruedRevenueCA",
    ] {
        assert_non_trade_current_receivable_alias(local_name);
    }
}

#[test]
fn lease_rental_and_other_receivable_aliases_keep_broad_current_scope() {
    for local_name in [
        "AccountReceivableSubcontractorOtherCA",
        "AccountsReceivableForRentCA",
        "AccountsReceivableLeaseCA",
        "AccountsReceivableLeaseCALEA",
        "AccountsReceivableLeasingCA",
        "AccountsReceivableOperatingLoansCASPF",
        "AccountsReceivableOther",
        "AccountsReceivableOtherBusinessCA",
        "AccountsReceivableOtherBusinessCAWAT",
        "AccountsReceivableOtherCA",
        "AccountsReceivableOtherFromFactor",
        "AccountsReceivableOtherFromSubsidiariesAndAffiliates",
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA",
        "AccountsReceivableOtherNetCA",
        "AccountsReceivableOtherOfStorageAgencyCA",
        "AccountsReceivablePaidProvisionCA",
        "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA",
        "AccountsReceivableRentalCA",
        "AccountsReceivableRentalCAAssets",
    ] {
        assert_non_trade_current_receivable_alias(local_name);
    }
}
