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

fn assert_semantic_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
    expected_statement: &str,
) {
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert_eq!(
        statement_for_concept(&jppfs(local_name)),
        Some(expected_statement)
    );
    assert!(matches!(
        concept_status(&jppfs(local_name)),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should be removed from excluded_concepts once reviewed"
    );
}

fn assert_balance_sheet_semantic_alias(local_name: &str, expected_field: &str) {
    assert_semantic_alias(
        local_name,
        expected_field,
        StatementCategory::BalanceSheet,
        "bs",
    );
}

#[test]
fn industry_current_customer_receivable_aliases_fold_into_trade_receivables() {
    for local_name in [
        "AccountsReceivableCARWY",
        "AccountsReceivableCustomerCACMD",
        "AccountsReceivableFormAgentsCAWAT",
        "AccountsReceivableFromSubsidiariesAndAffiliatesTradeCAGAS",
        "AccountsReceivableInstallmentSalesCALEA",
        "AccountsReceivableInstallmentSalesCreditGuaranteeCA",
        "AccountsReceivableShippingCAWAT",
        "RailwayFaresReceivablesCARWY",
    ] {
        assert_balance_sheet_semantic_alias(local_name, "accounts_receivable_trade");
    }
}

#[test]
fn industry_current_non_trade_receivable_aliases_keep_broad_current_scope() {
    for local_name in [
        "AccountsReceivableOperatingLoansCALEA",
        "AccountsReceivableOtherLoansToCustomersCALEA",
        "OtherAccountsReceivableCAELE",
        "ShortTermReceivablesFromSubsidiariesAndAffiliatesCAELE",
        "ShortTermReceivablesFromSubsidiariesAndAffiliatesCAGAS",
    ] {
        assert_balance_sheet_semantic_alias(local_name, "trade_and_other_current_receivables");
        assert_ne!(
            canonicalize_concept(&jppfs(local_name)).unwrap().item_name,
            "accounts_receivable_trade"
        );
    }
}

#[test]
fn industry_current_payable_aliases_fold_into_current_payables() {
    for local_name in [
        "AccountsPayableCreditGuaranteeCL",
        "AccountsPayableShippingCLWAT",
        "CurrentPortionOfLongTermAccountsPayableRailwayEquipmentCLRWY",
        "CurrentPortionOfLongTermPayablesUnderFluidityLeaseReceivablesCLLEA",
    ] {
        assert_balance_sheet_semantic_alias(local_name, "accounts_payable_current");
    }
}

#[test]
fn industry_consideration_received_in_advance_is_current_liability_not_payable() {
    for local_name in [
        "AdvancesReceivedLeaseCLLEA",
        "InterLineFaresReceivedCLRWY",
        "OtherAdvancesCLELE",
        "PrepaidFaresReceivedCLRWY",
    ] {
        assert_balance_sheet_semantic_alias(local_name, "advances_received_current");
        assert_ne!(
            canonicalize_concept(&jppfs(local_name)).unwrap().item_name,
            "accounts_payable_current"
        );
    }
}

#[test]
fn industry_income_statement_aliases_keep_statement_subtotal_semantics() {
    assert_semantic_alias(
        "OperatingGrossProfitWAT",
        "gross_profit",
        StatementCategory::ProfitAndLoss,
        "pl",
    );
    assert_semantic_alias(
        "OperatingIncomeRailwayRWY",
        "operating_income",
        StatementCategory::ProfitAndLoss,
        "pl",
    );

    for local_name in [
        "GeneralAndAdministrativeExpensesGAS",
        "GeneralAndAdministrativeExpensesOEElectricELE",
        "GeneralAndAdministrativeExpensesOERWY",
        "GeneralAndAdministrativeExpensesWAT",
        "SellingGeneralAndAdministrativeExpensesGAS",
    ] {
        assert_semantic_alias(
            local_name,
            "selling_general_admin",
            StatementCategory::ProfitAndLoss,
            "pl",
        );
    }
}

#[test]
fn industry_investing_cash_flow_asset_purchase_alias_maps_to_capex() {
    assert_semantic_alias(
        "PurchaseOfOwnUsedAssetsInvCFLEA",
        "capex",
        StatementCategory::CashFlow,
        "cf",
    );
}

#[test]
fn cost_of_sales_role_suffix_maps_to_cost_of_revenue() {
    assert_semantic_alias(
        "CostOfSalesCOS",
        "cost_of_revenue",
        StatementCategory::ProfitAndLoss,
        "pl",
    );
    assert_ne!(
        canonicalize_concept(&jppfs("CostOfSalesCOS"))
            .unwrap()
            .item_name,
        "cost_of_sales_cos",
        "CostOfSalesCOS is an explicit cost-of-sales concept, not an identity-only role tag"
    );
}

#[test]
fn industry_operating_expense_aliases_fold_into_operating_expenses() {
    for local_name in [
        "ElectricUtilityOperatingExpensesELE",
        "OperatingExpensesELE",
        "OperatingExpensesGasSupplyBusinessOEElectricELE",
        "OperatingExpensesIncidentalELC",
        "OperatingExpensesOILTelecommunications",
        "OperatingExpensesRailwayRWY",
        "OperatingExpensesSPF",
        "OperatingExpensesTotalRWY",
        "OtherBusinessOperatingExpensesELE",
        "OtherOperatingExpensesSPF",
        "ShippingBusinessExpensesAndOtherOperatingExpensesWAT",
        "TransportationOperatingExpensesOERWY",
        "WarehouseBusinessExpensesOtherBusinessRevenueWAT",
    ] {
        assert_semantic_alias(
            local_name,
            "operating_expenses",
            StatementCategory::ProfitAndLoss,
            "pl",
        );
        let item_name = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item_name, "cost_of_revenue");
        assert_ne!(item_name, "operating_expenses_ins");
        assert_ne!(item_name, "selling_general_admin");
    }
}
