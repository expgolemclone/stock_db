use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_semantic_concept, classify_item_name,
    concept_status, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> (String, String) {
    (
        "https://filer.example.co.jp/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

const CURRENT_ASSET_ALIASES: &[&str] = &["CurrentAssetsForIncidentalBusinessesCAGAS"];

const CURRENT_LIABILITY_ALIASES: &[&str] = &["CurrentLiabilitiesForIncidentalBusinessesCLGAS"];

const NONCURRENT_ASSET_ALIASES: &[&str] = &[
    "NoncurrentAssetsAutomobileRWY",
    "NoncurrentAssetsRailroadRWY",
    "NoncurrentAssetsRealEstateHotelRWY",
];

const TANGIBLE_FIXED_ASSET_ALIASES: &[&str] = &[
    "PropertyPlantAndEquipmentAutomobileRWY",
    "PropertyPlantAndEquipmentNetOtherRWY",
    "PropertyPlantAndEquipmentNetRailroadRWY",
    "PropertyPlantAndEquipmentOtherRWY",
    "PropertyPlantAndEquipmentRealEstateRWY",
];

const INTANGIBLE_FIXED_ASSET_ALIASES: &[&str] = &[
    "IntangibleAssetsAutomobileRWY",
    "IntangibleAssetsOtherRWY",
    "IntangibleAssetsRailroadRWY",
    "IntangibleAssetsRealEstateRWY",
];

const REVENUE_ALIASES: &[&str] = &[
    "OperatingRevenueTotalRWY",
    "ShippingBusinessRevenueAndOtherOperatingRevenueWAT",
    "WarehouseBusinessRevenueAndOtherOperatingRevenueWAT",
];

const OTHER_OPERATING_REVENUE_ALIASES: &[&str] = &[
    "AcceptForInterestFromSubsidiariesAndAffiliatesRevOA",
    "AffiliatedCompanyBusinessManagementFeeRevOA",
    "AffiliatesBusinessTrustIncomeRevOA",
    "IncomeFromRadioBusinessRevOA",
    "IncomeFromTelevisionBusinessRevOA",
    "MiscellaneousRevenueORRelated",
    "OtherOperatingRevenueCMD",
    "OtherOperatingRevenueORIVT",
    "OtherRevenueOROILTelecommunications",
    "OtherRevenueORSideLine",
    "OtherRevenueShipping",
];

const OPERATING_INCOME_ALIASES: &[&str] = &[
    "OperatingIncomeAutomobileRWY",
    "OperatingIncomeRailroadOERWY",
    "OperatingIncomeRealEstateRWY",
];

const OPERATING_EXPENSE_ALIASES: &[&str] = &[
    "OperatingExpensesTotalRWY",
    "ShippingBusinessExpensesAndOtherOperatingExpensesWAT",
    "WarehouseBusinessExpensesOtherBusinessRevenueWAT",
];

const COST_OF_REVENUE_ALIASES: &[&str] = &[
    "CostOfGoodsSalesOEOther",
    "CostOfSalesOEDevelopment",
    "CostOfSalesOERealEstate",
];

const DEPRECIATION_EXPENSE_ALIASES: &[&str] = &[
    "DepreciationOEAutomobile",
    "DepreciationOEDevelopment",
    "DepreciationOperatingExpensesIFRS",
];

const SGA_ALIASES: &[&str] = &[
    "SellingGeneralAndAdministrativeExpensesOEDevelopment",
    "SellingGeneralAndAdministrativeExpensesOERealEstate",
    "SellingGeneralAndAdministrativeExpensesOESideLine",
];

fn assert_alias(local_name: &str, expected_field: &str, expected_category: StatementCategory) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert_eq!(
        statement_for_concept(&concept),
        Some(statement_for_category(expected_category))
    );
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
}

fn assert_not_collapsed_to(local_name: &str, disallowed_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept);
    assert_ne!(
        resolved
            .as_ref()
            .map(|canonical| canonical.item_name.as_str()),
        Some(disallowed_field),
        "{local_name} is a component or mixed concept and must not collapse to {disallowed_field}"
    );
}

fn statement_for_category(category: StatementCategory) -> &'static str {
    match category {
        StatementCategory::BalanceSheet => "bs",
        StatementCategory::ProfitAndLoss => "pl",
        StatementCategory::CashFlow => "cf",
        _ => panic!("unexpected test category: {category:?}"),
    }
}

#[test]
fn observed_balance_sheet_aliases_resolve_only_when_they_are_same_meaning_totals() {
    for local_name in CURRENT_ASSET_ALIASES {
        assert_alias(
            local_name,
            "current_assets",
            StatementCategory::BalanceSheet,
        );
    }
    for local_name in CURRENT_LIABILITY_ALIASES {
        assert_alias(
            local_name,
            "current_liabilities",
            StatementCategory::BalanceSheet,
        );
    }
    for local_name in NONCURRENT_ASSET_ALIASES {
        assert_alias(
            local_name,
            "non_current_assets",
            StatementCategory::BalanceSheet,
        );
    }
    for local_name in TANGIBLE_FIXED_ASSET_ALIASES {
        assert_alias(
            local_name,
            "tangible_fixed_assets",
            StatementCategory::BalanceSheet,
        );
    }
    for local_name in INTANGIBLE_FIXED_ASSET_ALIASES {
        assert_alias(
            local_name,
            "intangible_fixed_assets",
            StatementCategory::BalanceSheet,
        );
    }
}

#[test]
fn observed_profit_loss_aliases_resolve_only_to_same_meaning_fields() {
    for local_name in REVENUE_ALIASES {
        assert_alias(local_name, "revenue", StatementCategory::ProfitAndLoss);
    }
    for local_name in OTHER_OPERATING_REVENUE_ALIASES {
        assert_alias(
            local_name,
            "other_operating_revenue",
            StatementCategory::ProfitAndLoss,
        );
        assert_not_collapsed_to(local_name, "revenue");
    }
    for local_name in OPERATING_INCOME_ALIASES {
        assert_alias(
            local_name,
            "operating_income",
            StatementCategory::ProfitAndLoss,
        );
    }
    for local_name in OPERATING_EXPENSE_ALIASES {
        assert_alias(
            local_name,
            "operating_expenses",
            StatementCategory::ProfitAndLoss,
        );
        assert_not_collapsed_to(local_name, "cost_of_revenue");
    }
    for local_name in COST_OF_REVENUE_ALIASES {
        assert_alias(
            local_name,
            "cost_of_revenue",
            StatementCategory::ProfitAndLoss,
        );
    }
    for local_name in DEPRECIATION_EXPENSE_ALIASES {
        assert_alias(local_name, "depreciation", StatementCategory::ProfitAndLoss);
    }
    for local_name in SGA_ALIASES {
        assert_alias(
            local_name,
            "selling_general_admin",
            StatementCategory::ProfitAndLoss,
        );
    }
}

#[test]
fn observed_component_tags_do_not_collapse_to_parent_aggregates() {
    for (local_name, disallowed_field) in [
        ("AccountsReceivableOtherBusinessCAWAT", "current_assets"),
        ("GuaranteeDepositsForTransactionCASEC", "current_assets"),
        ("LoansOnMarginTransactionCA", "current_assets"),
        (
            "CurrentPortionOfNoncurrentLiabilitiesCLELC",
            "current_liabilities",
        ),
        (
            "CurrentPortionOfOtherNoncurrentLiabilities",
            "current_liabilities",
        ),
        (
            "RenewablePowerProductionAndRelatedFacilitiesNCAElectricELE",
            "non_current_assets",
        ),
        ("InactiveFacilitiesPPEGAS", "non_current_assets"),
        (
            "PlantAndEquipmentAndIntangibleAssetsELE",
            "tangible_fixed_assets",
        ),
        ("ConstructionInProgressRailroadRWY", "tangible_fixed_assets"),
        ("LandRWY", "tangible_fixed_assets"),
        (
            "LineFacilitiesNetNCATelecommunications",
            "tangible_fixed_assets",
        ),
        ("VehiclesPPECMD", "tangible_fixed_assets"),
        ("GoodwillIAELC", "intangible_fixed_assets"),
        ("SoftwareRWY", "intangible_fixed_assets"),
        ("RevenueFromLNGSalesGAS", "revenue"),
        ("VoiceCommunicationsOROILTelecommunications", "revenue"),
        (
            "TransportationOperatingExpensesOEAutomobile",
            "cost_of_revenue",
        ),
        ("OperatingExpensesTransportationOERWY", "cost_of_revenue"),
        ("ComputerExpensesSGACMD", "selling_general_admin"),
        ("TaxesOESideLine", "selling_general_admin"),
        ("TradingRelatedExpensesSGA", "selling_general_admin"),
    ] {
        assert_not_collapsed_to(local_name, disallowed_field);
    }
}

#[test]
fn observed_same_meaning_aliases_are_namespace_gated() {
    for local_name in [
        CURRENT_ASSET_ALIASES[0],
        TANGIBLE_FIXED_ASSET_ALIASES[0],
        REVENUE_ALIASES[0],
        DEPRECIATION_EXPENSE_ALIASES[0],
        OTHER_OPERATING_REVENUE_ALIASES[0],
        OPERATING_EXPENSE_ALIASES[0],
    ] {
        assert!(canonicalize_semantic_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
