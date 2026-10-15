use crate::taxonomy::{
    StatementCategory, canonicalize_concept, classify_item_name, is_excluded_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn assert_jppfs_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
) {
    let resolved = canonicalize_concept(&jppfs(local_name))
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(
        resolved.item_name, expected_field,
        "{local_name} should preserve its reviewed accounting scope"
    );
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should be removed from excluded_concepts once reviewed"
    );
}

#[test]
fn reviewed_operating_revenue_components_are_pl_components_not_total_revenue() {
    for local_name in [
        "HarborTransportationIncomeRevOA",
        "BrokerageIncomeORCMD",
        "CreditCardRevenueRevOA",
        "RevenueFromOperationalInvestmentSecuritiesRevOA",
        "OtherElectricityRevenueORElectricELE",
    ] {
        assert_jppfs_alias(
            local_name,
            "other_operating_revenue",
            StatementCategory::ProfitAndLoss,
        );
    }
}

#[test]
fn reviewed_balance_sheet_aliases_preserve_asset_and_liability_scope() {
    for (local_name, expected_field) in [
        ("DebenturesLiabilitiesBNK", "bonds_payable"),
        (
            "LongTermDebtToSubsidiariesAndAffiliatesNCLELE",
            "long_term_debt",
        ),
        (
            "ShortTermDebtToSubsidiariesAndAffiliatesCLELE",
            "short_term_debt",
        ),
        ("IntangibleAssetsNCARWY", "intangible_fixed_assets"),
        ("ConstructionMaterialCA", "inventories"),
        ("RawMaterialsCAGAS", "inventories"),
    ] {
        assert_jppfs_alias(local_name, expected_field, StatementCategory::BalanceSheet);
    }
}

#[test]
fn reviewed_receivable_aliases_keep_reviewed_accounting_scope() {
    for local_name in [
        "AccountsReceivableTradeCA",
        "AccountsReceivableFromCompletedConstructionContracts",
        "AccountsReceivable",
        "AccountsReceivableCA",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));
        assert_eq!(
            resolved.item_name, "accounts_receivable_trade",
            "{local_name} is a trade/customer receivable or unconditional contract receivable"
        );
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert_ne!(resolved.item_name, "trade_and_other_current_receivables");
        assert!(!is_excluded_concept(local_name));
    }

    for local_name in [
        "AccountsReceivableReceivablesArisingFromContractsWithCustomersCurrentAssets",
        "AccountsReceivableFromCompletedConstructionContractsAndOther",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));
        assert_eq!(
            resolved.item_name, "trade_and_other_current_receivables",
            "{local_name} includes non-trade, other, or broader current-receivable scope"
        );
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet)
        );
        assert_ne!(resolved.item_name, "accounts_receivable_trade");
        assert!(!is_excluded_concept(local_name));
    }
}

#[test]
fn reviewed_payable_aliases_preserve_trade_or_current_scope() {
    for local_name in [
        "AccountsPayableForConstructionContracts",
        "AccountsPayableTradeAndAccountsPayableForConstructionContracts",
    ] {
        assert_jppfs_alias(
            local_name,
            "accounts_payable_for_construction_contracts_current",
            StatementCategory::BalanceSheet,
        );
    }

    assert_jppfs_alias(
        "AccountsPayableTradeLiabilitiesIFRS",
        "accounts_payable_trade",
        StatementCategory::BalanceSheet,
    );

    for local_name in [
        "AccountsPayable",
        "AccountsPayableCL",
        "AccountsPayableOtherCL",
    ] {
        assert_jppfs_alias(
            local_name,
            "accounts_payable_current",
            StatementCategory::BalanceSheet,
        );
    }

    assert_jppfs_alias(
        "AccountsPayableForEquipmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
        StatementCategory::BalanceSheet,
    );

    assert_jppfs_alias(
        "AccountsPayableRentalAssetsCL",
        "accounts_payable_for_rental_assets_current",
        StatementCategory::BalanceSheet,
    );
}

#[test]
fn reviewed_notes_aliases_do_not_lose_notes_scope() {
    assert_jppfs_alias(
        "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL",
        "accounts_payable_for_construction_contracts_current",
        StatementCategory::BalanceSheet,
    );

    let receivable_notes = "AccountsReceivableFromCompletedConstructionContractsNotesReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS";
    let resolved = canonicalize_concept(&jppfs(receivable_notes))
        .unwrap_or_else(|| panic!("{receivable_notes} should resolve as a reviewed alias"));
    assert_eq!(resolved.item_name, "notes_and_accounts_receivable_trade");
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_ne!(resolved.item_name, "accounts_receivable_trade");
}

#[test]
fn contract_asset_aliases_preserve_receivable_and_asset_scope() {
    for (local_name, expected_field) in [
        (
            "AccountsReceivableAndContractAssets",
            "accounts_receivable_and_contract_assets",
        ),
        (
            "AccountsReceivableTradeAndContractAssetsCA",
            "accounts_receivable_and_contract_assets",
        ),
        (
            "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssets",
            "notes_and_accounts_receivable_trade_and_contract_assets",
        ),
        ("ContractAssets", "contract_assets"),
    ] {
        assert_jppfs_alias(local_name, expected_field, StatementCategory::BalanceSheet);
        let item_name = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item_name, "accounts_receivable_trade");
        if expected_field != "contract_assets" {
            assert_ne!(item_name, "contract_assets");
        }
    }
}
