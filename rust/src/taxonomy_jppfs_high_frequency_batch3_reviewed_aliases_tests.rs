use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_semantic_concept, classify_item_name,
    concept_status, is_excluded_concept, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn assert_semantic_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
    expected_statement: &str,
) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed semantic alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert_eq!(statement_for_concept(&concept), Some(expected_statement));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should no longer be excluded"
    );
}

#[test]
fn high_frequency_batch3_balance_sheet_aliases_preserve_scope() {
    use StatementCategory::BalanceSheet;

    for (local_name, expected_field) in [
        ("ProductionFacilitiesPPEGAS", "production_facilities_gas"),
        (
            "DistributionFacilitiesPPEGAS",
            "distribution_facilities_gas",
        ),
        (
            "ServiceAndMaintenanceFacilitiesPPEGAS",
            "service_and_maintenance_facilities_gas",
        ),
        ("OtherFacilitiesPPEGAS", "other_facilities_gas"),
        (
            "FacilitiesForIncidentalBusinessesPPEGAS",
            "facilities_for_incidental_businesses_gas",
        ),
        (
            "ProvisionForReprocessingOfIrradiatedNuclearFuelNCLELE",
            "provision_for_reprocessing_of_irradiated_nuclear_fuel_non_current",
        ),
        (
            "ProvisionForPreparationOfTheReprocessingOfIrradiatedNuclearFuelNCLELE",
            "provision_for_preparation_of_reprocessing_of_irradiated_nuclear_fuel_non_current",
        ),
        (
            "ReserveForFluctuationInWaterLevelsReservesUnderTheSpecialLawsELE",
            "reserve_for_fluctuation_in_water_levels",
        ),
        (
            "ProvisionForSpecialRepairsNCLWAT",
            "provision_for_special_repairs_non_current_water_transport",
        ),
        (
            "LandAndBuildingsForSaleCARWY",
            "land_and_buildings_for_sale_current_railway",
        ),
        ("EquipmentNetPPEWAT", "equipment_net_water_transport"),
        (
            "DeferredAndPrepaidExpensesCAWAT",
            "deferred_and_prepaid_expenses_current_water_transport",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, BalanceSheet, "bs");
    }
}

#[test]
fn high_frequency_batch3_profit_loss_aliases_preserve_scope() {
    use StatementCategory::ProfitAndLoss;

    for (local_name, expected_field) in [
        ("MiscellaneousIncomeNOIRWY", "miscellaneous_income"),
        (
            "MiscellaneousExpensesNOERWY",
            "miscellaneous_expenses_non_operating",
        ),
        (
            "ProvisionOfReserveForFluctuationInWaterLevelsELE",
            "provision_of_reserve_for_fluctuation_in_water_levels",
        ),
        (
            "ReversalOfReserveForFluctuationInWaterLevelsELE",
            "reversal_of_reserve_for_fluctuation_in_water_levels",
        ),
        (
            "ProvisionOrReversalOfReserveForFluctuationInWaterLevelsELE",
            "provision_or_reversal_of_reserve_for_fluctuation_in_water_levels",
        ),
        (
            "ShippingBusinessIncomeWAT",
            "shipping_business_income_water_transport",
        ),
        (
            "ShippingBusinessExpensesWAT",
            "shipping_business_expenses_water_transport",
        ),
        (
            "OtherShippingBusinessExpensesWAT",
            "other_shipping_business_expenses_water_transport",
        ),
        ("TaxesOERWY", "taxes_railway_operating_expenses"),
        (
            "TaxesAndDuesOEOILTelecommunications",
            "taxes_and_dues_telecommunications_operating_expenses",
        ),
        ("SubsidyEIRWY", "subsidy_income_extraordinary"),
    ] {
        assert_semantic_alias(local_name, expected_field, ProfitAndLoss, "pl");
    }
}

#[test]
fn high_frequency_batch3_rejects_extensions() {
    let extension = (
        "https://issuer.example.jp/taxonomy/2026/custom".to_string(),
        "MiscellaneousIncomeNOIRWY".to_string(),
    );

    assert!(canonicalize_semantic_concept(&extension).is_none());
}
