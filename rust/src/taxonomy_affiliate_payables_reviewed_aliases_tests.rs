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

#[test]
fn reviewed_affiliate_trade_payable_alias_keeps_trade_scope() {
    let local_name = "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL";
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, "accounts_payable_trade");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_ne!(resolved.item_name, "accounts_payable_current");
    assert!(matches!(
        concept_status(&jppfs(local_name)),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(!is_excluded_concept(local_name));
}

#[test]
fn reviewed_affiliate_non_trade_payable_aliases_are_current_liabilities() {
    for local_name in [
        "AccountsPayableToAffiliatedCompanies",
        "AccountsPayableOtherFromSubsidiariesAndAssociates",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

        assert_eq!(resolved.item_name, "accounts_payable_current");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert_ne!(resolved.item_name, "accounts_payable_trade");
        assert!(matches!(
            concept_status(&jppfs(local_name)),
            TaxonomyConceptStatus::Semantic(_)
        ));
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_other_current_receivable_aliases_keep_non_trade_current_scope() {
    for local_name in [
        "AccountReceivableSubcontractorOtherCA",
        "AccountsReceivableDevelopmentProjectsCACNS",
        "AccountsReceivableForRentCA",
        "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA",
        "AccountsReceivableLeaseCA",
        "AccountsReceivableLeaseCALEA",
        "AccountsReceivableLeasingCA",
        "AccountsReceivableOtherBusinessCA",
        "AccountsReceivableOtherCA",
        "AccountsReceivableOtherFromFactor",
        "AccountsReceivableOtherMaterialsCA",
        "AccountsReceivableOtherNetCA",
        "AccountsReceivableOtherOfStorageAgencyCA",
        "AccountsReceivablePaidProvisionCA",
        "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA",
    ] {
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
        assert!(!is_excluded_concept(local_name));
    }
}
