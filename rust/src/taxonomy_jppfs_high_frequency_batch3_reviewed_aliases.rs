use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_high_frequency_batch3_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Railway non-operating miscellaneous items. Income can reuse the
        // reviewed NOI miscellaneous-income canonical; expenses are kept
        // separate from miscellaneous losses.
        "MiscellaneousIncomeNOIRWY" => "miscellaneous_income",
        "MiscellaneousExpensesNOERWY" => "miscellaneous_expenses_non_operating",

        // Gas utility property, plant and equipment classes.
        "ProductionFacilitiesPPEGAS" => "production_facilities_gas",
        "DistributionFacilitiesPPEGAS" => "distribution_facilities_gas",
        "ServiceAndMaintenanceFacilitiesPPEGAS" => "service_and_maintenance_facilities_gas",
        "OtherFacilitiesPPEGAS" => "other_facilities_gas",
        "FacilitiesForIncidentalBusinessesPPEGAS" => "facilities_for_incidental_businesses_gas",

        // Electric utility statutory reserves and nuclear-fuel provisions.
        "ProvisionForReprocessingOfIrradiatedNuclearFuelNCLELE" => {
            "provision_for_reprocessing_of_irradiated_nuclear_fuel_non_current"
        }
        "ProvisionForPreparationOfTheReprocessingOfIrradiatedNuclearFuelNCLELE" => {
            "provision_for_preparation_of_reprocessing_of_irradiated_nuclear_fuel_non_current"
        }
        "ReserveForFluctuationInWaterLevelsReservesUnderTheSpecialLawsELE" => {
            "reserve_for_fluctuation_in_water_levels"
        }
        "ProvisionOfReserveForFluctuationInWaterLevelsELE" => {
            "provision_of_reserve_for_fluctuation_in_water_levels"
        }
        "ReversalOfReserveForFluctuationInWaterLevelsELE" => {
            "reversal_of_reserve_for_fluctuation_in_water_levels"
        }
        "ProvisionOrReversalOfReserveForFluctuationInWaterLevelsELE" => {
            "provision_or_reversal_of_reserve_for_fluctuation_in_water_levels"
        }

        // Water-transport and railway industry-specific accounts.
        "ProvisionForSpecialRepairsNCLWAT" => {
            "provision_for_special_repairs_non_current_water_transport"
        }
        "LandAndBuildingsForSaleCARWY" => "land_and_buildings_for_sale_current_railway",
        "EquipmentNetPPEWAT" => "equipment_net_water_transport",
        "DeferredAndPrepaidExpensesCAWAT" => {
            "deferred_and_prepaid_expenses_current_water_transport"
        }
        "ShippingBusinessIncomeWAT" => "shipping_business_income_water_transport",
        "ShippingBusinessExpensesWAT" => "shipping_business_expenses_water_transport",
        "OtherShippingBusinessExpensesWAT" => "other_shipping_business_expenses_water_transport",
        "TaxesOERWY" => "taxes_railway_operating_expenses",
        "TaxesAndDuesOEOILTelecommunications" => {
            "taxes_and_dues_telecommunications_operating_expenses"
        }
        "SubsidyEIRWY" => "subsidy_income_extraordinary",

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

    let field = reviewed_jppfs_high_frequency_batch3_alias(&concept.1)?;
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
    reviewed_jppfs_high_frequency_batch3_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

fn reviewed_statement_for_item_name(item_name: &str) -> Option<&'static str> {
    let statement = match item_name {
        "production_facilities_gas"
        | "distribution_facilities_gas"
        | "service_and_maintenance_facilities_gas"
        | "other_facilities_gas"
        | "facilities_for_incidental_businesses_gas"
        | "provision_for_reprocessing_of_irradiated_nuclear_fuel_non_current"
        | "provision_for_preparation_of_reprocessing_of_irradiated_nuclear_fuel_non_current"
        | "reserve_for_fluctuation_in_water_levels"
        | "provision_for_special_repairs_non_current_water_transport"
        | "land_and_buildings_for_sale_current_railway"
        | "equipment_net_water_transport"
        | "deferred_and_prepaid_expenses_current_water_transport" => "bs",

        "miscellaneous_expenses_non_operating"
        | "provision_of_reserve_for_fluctuation_in_water_levels"
        | "reversal_of_reserve_for_fluctuation_in_water_levels"
        | "provision_or_reversal_of_reserve_for_fluctuation_in_water_levels"
        | "shipping_business_income_water_transport"
        | "shipping_business_expenses_water_transport"
        | "other_shipping_business_expenses_water_transport"
        | "taxes_railway_operating_expenses"
        | "taxes_and_dues_telecommunications_operating_expenses" => "pl",

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

