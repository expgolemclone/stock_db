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
fn high_frequency_batch2_balance_sheet_aliases_preserve_scope() {
    use StatementCategory::BalanceSheet;

    for (local_name, expected_field) in [
        ("AccruedTaxes", "accrued_taxes_current"),
        (
            "CurrentPortionOfNoncurrentLiabilitiesCLELE",
            "current_portion_of_non_current_liabilities_electric_utility",
        ),
        (
            "CurrentPortionOfNoncurrentLiabilitiesCLGAS",
            "current_portion_of_non_current_liabilities_gas",
        ),
        (
            "ElectricUtilityPlantAndEquipmentAssetsELE",
            "electric_utility_plant_and_equipment_assets",
        ),
        (
            "ConstructionInProgressELE",
            "construction_in_progress_electric_utility",
        ),
        (
            "ConstructionInProgressNCAInProgressELE",
            "construction_in_progress_electric_utility",
        ),
        (
            "ConstructionAndRetirementInProgressNCAInProcessELE",
            "construction_and_retirement_in_progress_electric_utility",
        ),
        (
            "RetirementInProgressNCAInProgressELE",
            "retirement_in_progress_electric_utility",
        ),
        (
            "TransmissionFacilitiesNCAElectricELE",
            "transmission_facilities_electric_utility",
        ),
        (
            "TransformationFacilitiesNCAElectricELE",
            "transformation_facilities_electric_utility",
        ),
        (
            "DistributionFacilitiesNCAElectricELE",
            "distribution_facilities_electric_utility",
        ),
        (
            "GeneralFacilitiesNCAElectricELE",
            "general_facilities_electric_utility",
        ),
        (
            "HydroelectricPowerProductionFacilitiesNCAElectricELE",
            "hydroelectric_power_production_facilities_electric_utility",
        ),
        (
            "ThermalPowerProductionFacilitiesNCAElectricELE",
            "thermal_power_production_facilities_electric_utility",
        ),
        (
            "NuclearPowerProductionFacilitiesNCAElectricELE",
            "nuclear_power_production_facilities_electric_utility",
        ),
        (
            "RenewablePowerProductionFacilitiesNCAElectricELE",
            "renewable_power_production_facilities_electric_utility",
        ),
        (
            "InternalCombustionEnginePowerProductionFacilitiesNCAElectricELE",
            "internal_combustion_engine_power_production_facilities_electric_utility",
        ),
        (
            "OtherElectricUtilityPlantAndEquipmentNCAElectricELE",
            "other_electric_utility_plant_and_equipment",
        ),
        (
            "FacilitiesLoanedNCAElectricELE",
            "facilities_loaned_electric_utility",
        ),
        (
            "InactiveFacilitiesNCAElectricELE",
            "inactive_facilities_electric_utility",
        ),
        (
            "NonOperatingFacilitiesELE",
            "non_operating_facilities_electric_utility",
        ),
        (
            "IncidentalBusinessFacilitiesELE",
            "incidental_business_facilities_electric_utility",
        ),
        ("NuclearFuelNCAELE", "nuclear_fuel"),
        ("LoadedNuclearFuelNCAELE", "loaded_nuclear_fuel"),
        (
            "NuclearFuelInProcessingNCAELE",
            "nuclear_fuel_in_processing",
        ),
        ("LongTermInvestmentsELE", "long_term_investments"),
        (
            "LongTermInvestmentForSubsidiariesAndAffiliatesELE",
            "long_term_investments_in_subsidiaries_and_affiliates_electric_utility",
        ),
        (
            "ReserveFundForReprocessingOfIrradiatedNuclearFuelIOAELE",
            "reserve_fund_for_reprocessing_of_irradiated_nuclear_fuel",
        ),
        (
            "SpecialAccountRelatedToReprocessingOfSpentNuclearFuelELE",
            "special_account_related_to_reprocessing_of_spent_nuclear_fuel",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, BalanceSheet, "bs");
    }
}

#[test]
fn high_frequency_batch2_profit_loss_aliases_preserve_scope() {
    use StatementCategory::ProfitAndLoss;

    for (local_name, expected_field) in [
        ("OrdinaryRevenueELE", "revenue"),
        ("OrdinaryExpensesELE", "ordinary_expenses_electric_utility"),
        (
            "HydroelectricPowerProductionExpensesELE",
            "hydroelectric_power_production_expenses_electric_utility",
        ),
        (
            "ThermalPowerProductionExpensesOEElectricELE",
            "thermal_power_production_expenses_electric_utility",
        ),
        (
            "NuclearPowerProductionExpensesELE",
            "nuclear_power_production_expenses_electric_utility",
        ),
        (
            "RenewablePowerProductionExpensesOEElectricELE",
            "renewable_power_production_expenses_electric_utility",
        ),
        (
            "InternalCombustionEnginePowerProductionExpensesOEElectricELE",
            "internal_combustion_engine_power_production_expenses_electric_utility",
        ),
        (
            "TransmissionExpensesOEElectricELE",
            "transmission_expenses_electric_utility",
        ),
        (
            "TransformationExpensesOEElectricELE",
            "transformation_expenses_electric_utility",
        ),
        (
            "DistributionExpensesOEElectricELE",
            "distribution_expenses_electric_utility",
        ),
        (
            "SellingExpensesOEElectricELE",
            "selling_expenses_electric_utility",
        ),
        (
            "GeneralAndAdministrativeExpensesOEElectricELE",
            "general_and_administrative_expenses_electric_utility",
        ),
        (
            "ExpensesForThirdPartysPowerTransmissionServiceOEElectricELE",
            "expenses_for_third_partys_power_transmission_service_electric_utility",
        ),
        (
            "PurchasedPowerFromOtherSuppliersOEElectricELE",
            "purchased_power_from_other_suppliers_electric_utility",
        ),
        (
            "PurchasedPowerFromOtherUtilitiesOEElectricELE",
            "purchased_power_from_other_utilities_electric_utility",
        ),
        (
            "TransferredCostOfElectricityForConstructionAndIncidentalBusinessOEElectricELE",
            "transferred_cost_of_electricity_for_construction_and_incidental_business_electric_utility",
        ),
        (
            "CostOfLoanedFacilitiesOEElectricELE",
            "cost_of_loaned_facilities_electric_utility",
        ),
        (
            "ElectricPowerDevelopmentPromotionTaxOEElectricELE",
            "electric_power_development_promotion_tax_electric_utility",
        ),
        (
            "EnterpriseTaxOEElectricELE",
            "enterprise_tax_electric_utility",
        ),
        (
            "LevyUnderActOnPurchaseOfRenewableEnergySourcedElectricityOEElectricELE",
            "levy_under_act_on_purchase_of_renewable_energy_sourced_electricity_electric_utility",
        ),
        (
            "IncidentalBusinessOperatingExpensesELE",
            "incidental_business_operating_expenses_electric_utility",
        ),
        (
            "OperatingExpensesOtherBusinessesOEIncidentalELE",
            "operating_expenses_other_businesses_incidental_electric_utility",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, ProfitAndLoss, "pl");
    }
}

#[test]
fn high_frequency_batch2_rejects_extensions() {
    let extension = (
        "https://issuer.example.jp/taxonomy/2026/custom".to_string(),
        "AccruedTaxes".to_string(),
    );

    assert!(canonicalize_semantic_concept(&extension).is_none());
}
