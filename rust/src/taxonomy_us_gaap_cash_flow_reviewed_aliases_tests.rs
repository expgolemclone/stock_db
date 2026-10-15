use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
    concept_status, is_excluded_concept, statement_for_concept,
};

fn us_gaap(local_name: &str) -> (String, String) {
    (
        "https://fasb.org/us-gaap/2024".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn reviewed_us_gaap_investing_cash_flow_aliases_are_semantic_cash_flow_items() {
    for (local_name, expected_field) in [
        ("PaymentsToAcquirePropertyPlantAndEquipment", "capex"),
        ("PaymentsToAcquireProductiveAssets", "capex"),
        (
            "PaymentsToAcquireIntangibleAssets",
            "intangible_asset_purchases",
        ),
        ("ProceedsFromSaleOfPropertyPlantAndEquipment", "ppe_sold"),
    ] {
        let resolved = canonicalize_concept(&us_gaap(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed US-GAAP alias"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::CashFlow)
        );
        assert_eq!(statement_for_concept(&us_gaap(local_name)), Some("cf"));
        assert!(matches!(
            concept_status(&us_gaap(local_name)),
            TaxonomyConceptStatus::Semantic(_)
        ));
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_us_gaap_investment_security_cash_flow_aliases_are_semantic_cash_flow_items() {
    for (local_name, expected_field) in [
        (
            "PaymentsToAcquireAvailableForSaleSecurities",
            "investment_securities_purchased",
        ),
        (
            "PaymentsToAcquireMarketableSecurities",
            "investment_securities_purchased",
        ),
        (
            "ProceedsFromSaleOfAvailableForSaleSecurities",
            "investment_securities_sold",
        ),
        (
            "ProceedsFromSaleOfMarketableSecurities",
            "investment_securities_sold",
        ),
    ] {
        let resolved = canonicalize_concept(&us_gaap(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed US-GAAP alias"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::CashFlow)
        );
        assert_eq!(statement_for_concept(&us_gaap(local_name)), Some("cf"));
        assert!(matches!(
            concept_status(&us_gaap(local_name)),
            TaxonomyConceptStatus::Semantic(_)
        ));
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_us_gaap_financing_cash_flow_aliases_preserve_financing_context() {
    for (local_name, expected_field) in [
        ("PaymentsOfDividends", "cash_dividends_paid_financing_cf"),
        ("RepaymentsOfLongTermDebt", "lt_debt_repayment"),
        ("ProceedsFromIssuanceOfLongTermDebt", "lt_debt_proceeds"),
        (
            "PaymentsForRepurchaseOfCommonStock",
            "treasury_stock_purchase",
        ),
        ("PaymentsForRepurchaseOfEquity", "treasury_stock_purchase"),
    ] {
        let resolved = canonicalize_concept(&us_gaap(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed US-GAAP alias"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::CashFlow)
        );
        assert_eq!(statement_for_concept(&us_gaap(local_name)), Some("cf"));
        assert!(matches!(
            concept_status(&us_gaap(local_name)),
            TaxonomyConceptStatus::Semantic(_)
        ));
        assert!(!is_excluded_concept(local_name));
    }

    assert_ne!(
        canonicalize_concept(&us_gaap("PaymentsOfDividends"))
            .unwrap()
            .item_name,
        "cash_dividends_paid",
        "US-GAAP dividends paid should use the financing-CF field, not the generic dividend cash-flow field"
    );
}
