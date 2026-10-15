use crate::taxonomy::{
    StatementCategory, canonicalize_concept, classify_item_name, is_excluded_concept,
    statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn reviewed_amortization_aliases_preserve_profit_or_cash_flow_context() {
    for local_name in [
        "AmoritisationOfIntangibleAssetsIFRS",
        "AmortizationIFRS",
        "AmortizationOfIdentifiableAssetsIFRS",
        "AmortizationOfIntangibleAssets",
        "AmortizationOfIntangibleAssetsAssociatedWithProductsIFRS",
        "AmortizationOnIntangibleAssetsAssociatedWithProductsIFRS",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed PL alias"));

        assert_eq!(resolved.item_name, "amortization");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::ProfitAndLoss)
        );
        assert_eq!(statement_for_concept(&jppfs(local_name)), Some("pl"));
        assert!(!is_excluded_concept(local_name));
    }

    for local_name in [
        "AmoitizationOfDefferedAssetsOpeCF",
        "AmortizationExpensesOfServicingAssetsOpeCFIFRS",
        "AmortizationLongTermPrepaidExpensesOpeCF",
        "AmortizationOfContactCostsOpeCFIFRS",
        "AmortizationOfContractRelatedIntangibleAssetsOpeCF",
        "AmortizationOfCustomerDataOpeCF",
        "AmortizationOfCustomerRelatedAssetsOpeCF",
        "AmortizationOfCustomerRelationshipCF",
        "AmortizationOfCustomerRelationshipOpeCF",
        "AmortizationOfCustomerReletionshipOpeCF",
        "AmortizationOfDeferredAssetsOpeCF",
        "AmortizationOfIntangibleAssetOpeCFIFRS",
        "AmortizationOfIntangibleAssetsOpeCFIFRS",
        "AmortizationOfOtherIntangibleAssetsOpeCF",
        "AmortizationOfRightOfUseAssetsOpeCF",
        "AmortizationOfRightOfUseAssetsOpeCFIFRS",
        "AmortizationOpeCFIFRS",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed CF alias"));

        assert_eq!(resolved.item_name, "amortization_cf_adjustment");
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::CashFlow)
        );
        assert_eq!(statement_for_concept(&jppfs(local_name)), Some("cf"));
        assert!(!is_excluded_concept(local_name));
    }

    assert_ne!(
        canonicalize_concept(&jppfs("AmortizationOfIntangibleAssets"))
            .unwrap()
            .item_name,
        canonicalize_concept(&jppfs("AmortizationOfIntangibleAssetsOpeCFIFRS"))
            .unwrap()
            .item_name,
        "PL amortization expense and operating-CF add-back must remain separate"
    );
}
