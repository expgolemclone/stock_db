use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_high_frequency_batch2_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // AccruedTaxes is a current tax liability. Keep it separate from
        // income_taxes_payable and accrued consumption taxes.
        "AccruedTaxes" => "accrued_taxes_current",

        "CurrentPortionOfNoncurrentLiabilitiesCLELE" => {
            "current_portion_of_non_current_liabilities_electric_utility"
        }
        "CurrentPortionOfNoncurrentLiabilitiesCLGAS" => {
            "current_portion_of_non_current_liabilities_gas"
        }

        // Electric-utility non-current asset classes. These are BS components,
        // not generic PPE or total assets.
        "ElectricUtilityPlantAndEquipmentAssetsELE" => {
            "electric_utility_plant_and_equipment_assets"
        }
        "ConstructionInProgressELE" | "ConstructionInProgressNCAInProgressELE" => {
            "construction_in_progress_electric_utility"
        }
        "ConstructionAndRetirementInProgressNCAInProcessELE" => {
            "construction_and_retirement_in_progress_electric_utility"
        }
        "RetirementInProgressNCAInProgressELE" => "retirement_in_progress_electric_utility",
        "TransmissionFacilitiesNCAElectricELE" => "transmission_facilities_electric_utility",
        "TransformationFacilitiesNCAElectricELE" => "transformation_facilities_electric_utility",
        "DistributionFacilitiesNCAElectricELE" => "distribution_facilities_electric_utility",
        "GeneralFacilitiesNCAElectricELE" => "general_facilities_electric_utility",
        "HydroelectricPowerProductionFacilitiesNCAElectricELE" => {
            "hydroelectric_power_production_facilities_electric_utility"
        }
        "ThermalPowerProductionFacilitiesNCAElectricELE" => {
            "thermal_power_production_facilities_electric_utility"
        }
        "NuclearPowerProductionFacilitiesNCAElectricELE" => {
            "nuclear_power_production_facilities_electric_utility"
        }
        "RenewablePowerProductionFacilitiesNCAElectricELE" => {
            "renewable_power_production_facilities_electric_utility"
        }
        "InternalCombustionEnginePowerProductionFacilitiesNCAElectricELE" => {
            "internal_combustion_engine_power_production_facilities_electric_utility"
        }
        "OtherElectricUtilityPlantAndEquipmentNCAElectricELE" => {
            "other_electric_utility_plant_and_equipment"
        }
        "FacilitiesLoanedNCAElectricELE" => "facilities_loaned_electric_utility",
        "InactiveFacilitiesNCAElectricELE" => "inactive_facilities_electric_utility",
        "NonOperatingFacilitiesELE" => "non_operating_facilities_electric_utility",
        "IncidentalBusinessFacilitiesELE" => "incidental_business_facilities_electric_utility",
        "NuclearFuelNCAELE" => "nuclear_fuel",
        "LoadedNuclearFuelNCAELE" => "loaded_nuclear_fuel",
        "NuclearFuelInProcessingNCAELE" => "nuclear_fuel_in_processing",
        "LongTermInvestmentsELE" => "long_term_investments",
        "LongTermInvestmentForSubsidiariesAndAffiliatesELE" => {
            "long_term_investments_in_subsidiaries_and_affiliates_electric_utility"
        }
        "ReserveFundForReprocessingOfIrradiatedNuclearFuelIOAELE" => {
            "reserve_fund_for_reprocessing_of_irradiated_nuclear_fuel"
        }
        "SpecialAccountRelatedToReprocessingOfSpentNuclearFuelELE" => {
            "special_account_related_to_reprocessing_of_spent_nuclear_fuel"
        }

        // Electric-utility PL concepts. Ordinary revenue is the industry top
        // line and follows the existing OrdinaryIncomeELE => revenue policy.
        // Ordinary expenses include more than operating expenses, so they stay
        // distinct from operating_expenses and cost_of_revenue.
        "OrdinaryRevenueELE" => "revenue",
        "OrdinaryExpensesELE" => "ordinary_expenses_electric_utility",
        "HydroelectricPowerProductionExpensesELE" => {
            "hydroelectric_power_production_expenses_electric_utility"
        }
        "ThermalPowerProductionExpensesOEElectricELE" => {
            "thermal_power_production_expenses_electric_utility"
        }
        "NuclearPowerProductionExpensesELE" => "nuclear_power_production_expenses_electric_utility",
        "RenewablePowerProductionExpensesOEElectricELE" => {
            "renewable_power_production_expenses_electric_utility"
        }
        "InternalCombustionEnginePowerProductionExpensesOEElectricELE" => {
            "internal_combustion_engine_power_production_expenses_electric_utility"
        }
        "TransmissionExpensesOEElectricELE" => "transmission_expenses_electric_utility",
        "TransformationExpensesOEElectricELE" => "transformation_expenses_electric_utility",
        "DistributionExpensesOEElectricELE" => "distribution_expenses_electric_utility",
        "SellingExpensesOEElectricELE" => "selling_expenses_electric_utility",
        "GeneralAndAdministrativeExpensesOEElectricELE" => {
            "general_and_administrative_expenses_electric_utility"
        }
        "ExpensesForThirdPartysPowerTransmissionServiceOEElectricELE" => {
            "expenses_for_third_partys_power_transmission_service_electric_utility"
        }
        "PurchasedPowerFromOtherSuppliersOEElectricELE" => {
            "purchased_power_from_other_suppliers_electric_utility"
        }
        "PurchasedPowerFromOtherUtilitiesOEElectricELE" => {
            "purchased_power_from_other_utilities_electric_utility"
        }
        "TransferredCostOfElectricityForConstructionAndIncidentalBusinessOEElectricELE" => {
            "transferred_cost_of_electricity_for_construction_and_incidental_business_electric_utility"
        }
        "CostOfLoanedFacilitiesOEElectricELE" => "cost_of_loaned_facilities_electric_utility",
        "ElectricPowerDevelopmentPromotionTaxOEElectricELE" => {
            "electric_power_development_promotion_tax_electric_utility"
        }
        "EnterpriseTaxOEElectricELE" => "enterprise_tax_electric_utility",
        "LevyUnderActOnPurchaseOfRenewableEnergySourcedElectricityOEElectricELE" => {
            "levy_under_act_on_purchase_of_renewable_energy_sourced_electricity_electric_utility"
        }
        "IncidentalBusinessOperatingExpensesELE" => {
            "incidental_business_operating_expenses_electric_utility"
        }
        "OperatingExpensesOtherBusinessesOEIncidentalELE" => {
            "operating_expenses_other_businesses_incidental_electric_utility"
        }

        _ => return None,
    };
    Some(field)
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_high_frequency_batch2_alias(&concept.1)?;
    Some(CanonicalConcept {
        item_name: field.to_string(),
        priority: reviewed_priority(&concept.1),
        match_kind: MatchKind::TaxonomyFamilyAlias,
    })
}

fn reviewed_priority(local_name: &str) -> u8 {
    if local_name.contains("SummaryOfBusinessResults")
        || local_name.contains("SummaryOfBusinessResult")
    {
        5
    } else if local_name.contains("IFRS") {
        20
    } else {
        10
    }
}

pub fn canonicalize_semantic_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    reviewed_alias(concept).or_else(|| base::canonicalize_semantic_concept(concept))
}

pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    reviewed_alias(concept).or_else(|| base::canonicalize_concept(concept))
}

pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(alias) = reviewed_alias(concept) {
        return TaxonomyConceptStatus::Semantic(alias);
    }
    base::concept_status(concept)
}

#[cfg(test)]
pub fn is_excluded_concept(local_name: &str) -> bool {
    reviewed_jppfs_high_frequency_batch2_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

fn reviewed_statement_for_item_name(item_name: &str) -> Option<&'static str> {
    let statement = match item_name {
        "accrued_taxes_current"
        | "current_portion_of_non_current_liabilities_electric_utility"
        | "current_portion_of_non_current_liabilities_gas"
        | "electric_utility_plant_and_equipment_assets"
        | "construction_in_progress_electric_utility"
        | "construction_and_retirement_in_progress_electric_utility"
        | "retirement_in_progress_electric_utility"
        | "transmission_facilities_electric_utility"
        | "transformation_facilities_electric_utility"
        | "distribution_facilities_electric_utility"
        | "general_facilities_electric_utility"
        | "hydroelectric_power_production_facilities_electric_utility"
        | "thermal_power_production_facilities_electric_utility"
        | "nuclear_power_production_facilities_electric_utility"
        | "renewable_power_production_facilities_electric_utility"
        | "internal_combustion_engine_power_production_facilities_electric_utility"
        | "other_electric_utility_plant_and_equipment"
        | "facilities_loaned_electric_utility"
        | "inactive_facilities_electric_utility"
        | "non_operating_facilities_electric_utility"
        | "incidental_business_facilities_electric_utility"
        | "nuclear_fuel"
        | "loaded_nuclear_fuel"
        | "nuclear_fuel_in_processing"
        | "long_term_investments"
        | "long_term_investments_in_subsidiaries_and_affiliates_electric_utility"
        | "reserve_fund_for_reprocessing_of_irradiated_nuclear_fuel"
        | "special_account_related_to_reprocessing_of_spent_nuclear_fuel" => "bs",

        "ordinary_expenses_electric_utility"
        | "hydroelectric_power_production_expenses_electric_utility"
        | "thermal_power_production_expenses_electric_utility"
        | "nuclear_power_production_expenses_electric_utility"
        | "renewable_power_production_expenses_electric_utility"
        | "internal_combustion_engine_power_production_expenses_electric_utility"
        | "transmission_expenses_electric_utility"
        | "transformation_expenses_electric_utility"
        | "distribution_expenses_electric_utility"
        | "selling_expenses_electric_utility"
        | "general_and_administrative_expenses_electric_utility"
        | "expenses_for_third_partys_power_transmission_service_electric_utility"
        | "purchased_power_from_other_suppliers_electric_utility"
        | "purchased_power_from_other_utilities_electric_utility"
        | "transferred_cost_of_electricity_for_construction_and_incidental_business_electric_utility"
        | "cost_of_loaned_facilities_electric_utility"
        | "electric_power_development_promotion_tax_electric_utility"
        | "enterprise_tax_electric_utility"
        | "levy_under_act_on_purchase_of_renewable_energy_sourced_electricity_electric_utility"
        | "incidental_business_operating_expenses_electric_utility"
        | "operating_expenses_other_businesses_incidental_electric_utility" => "pl",

        _ => return None,
    };
    Some(statement)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match reviewed_statement_for_item_name(item_name) {
        Some("bs") => Some(StatementCategory::BalanceSheet),
        Some("pl") => Some(StatementCategory::ProfitAndLoss),
        Some("cf") => Some(StatementCategory::CashFlow),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    reviewed_statement_for_item_name(item_name).or_else(|| base::statement_for_item_name(item_name))
}

pub fn statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    canonicalize_concept(concept)
        .and_then(|canonical| statement_for_item_name(&canonical.item_name))
}

pub fn raw_statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    match statement_for_concept(concept)? {
        "bs" => Some("bs"),
        "pl" => Some("pl"),
        "cf" => Some("cf"),
        _ => None,
    }
}

