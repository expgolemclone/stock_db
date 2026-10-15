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

fn jpigp(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2020-11-01/jpigp_cor".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn accrued_expense_aliases_are_current_balance_sheet_liabilities() {
    for concept in [
        jppfs("AccruedExpenses"),
        jppfs("AccruedExpensesCLIFRS"),
        jpigp("AccruedLiabilitiesCLIFRS"),
    ] {
        let local_name = concept.1.clone();
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

        assert_eq!(resolved.item_name, "accrued_expenses_current");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert_eq!(statement_for_concept(&concept), Some("bs"));
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
        assert!(!is_excluded_concept(&local_name));
    }
}

#[test]
fn accrued_expense_cash_flow_adjustments_are_not_balance_sheet_aliases() {
    for concept in [
        jppfs("AccruedExpensesOpeCFIFRS"),
        jpigp("AccruedLiabilitiesOpeCFIFRS"),
    ] {
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{} should remain an identity concept", concept.1));

        assert_ne!(resolved.item_name, "accrued_expenses_current");
        assert_ne!(statement_for_concept(&concept), Some("bs"));
        assert!(!matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(canonical)
                if canonical.item_name == "accrued_expenses_current"
        ));
    }
}
