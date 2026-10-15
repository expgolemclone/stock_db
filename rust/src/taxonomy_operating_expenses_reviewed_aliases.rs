use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_operating_expense_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // CostOfSalesCOS is a cost-of-sales concept carrying the COS presentation
        // role. The role suffix confirms the cost-of-sales context; it does not
        // change the accounting meaning into an identity-only field.
        "CostOfSalesCOS" => "cost_of_revenue",

        // Industry-specific operating-expense line items. These tags are
        // ordinary operating expenses presented by industry taxonomies. Keep
        // them in a general operating-expense bucket rather than leaving each
        // issuer/industry spelling as a separate identity tag, folding them
        // into SG&A, or narrowing broad operating-expense totals to cost of
        // revenue / cost of sales.
        "ElectricUtilityOperatingExpensesELE"
        | "OperatingExpensesAutomobileRWY"
        | "OperatingExpensesCMD"
        | "OperatingExpensesCoalSaleBusinessOEIncidentalELE"
        | "OperatingExpensesConsultingBusinessOEIncidentalELE"
        | "OperatingExpensesDevelopmentRWY"
        | "OperatingExpensesELE"
        | "OperatingExpensesElectricWaterHeaterLeaseBusinessOEIncidentalELE"
        | "OperatingExpensesElectricWireLeaseBusinessOEIncidentalELE"
        | "OperatingExpensesGasSupplyBusinessOEElectricELE"
        | "OperatingExpensesHeatSupplyBusinessOEIncidentalELE"
        | "OperatingExpensesHouseElectrificationMachineryLeaseBusinessOEIncidentalELE"
        | "OperatingExpensesIncidentalELC"
        | "OperatingExpensesIncidentalRWY"
        | "OperatingExpensesInformationAndCommunicationBusinessOEIncidentalELE"
        | "OperatingExpensesIVT"
        | "OperatingExpensesLNGSupplyBusinessOEIncidentalELE"
        | "OperatingExpensesOEOILTelecommunications"
        | "OperatingExpensesOILTelecommunications"
        | "OperatingExpensesOpticalFiberElectricWireLeaseBusinessOEIncidentalELE"
        | "OperatingExpensesOtherRWY"
        | "OperatingExpensesRailroadRWY"
        | "OperatingExpensesRailwayRWY"
        | "OperatingExpensesRealEstateRentBusinessOEIncidentalELE"
        | "OperatingExpensesRealEstateRWY"
        | "OperatingExpensesRelatedRWY"
        | "OperatingExpensesSideLineRWY"
        | "OperatingExpensesSPF"
        | "OperatingExpensesSteamSupplyBusinessOEIncidentalELE"
        | "OperatingExpensesTotalRWY"
        | "OperatingExpensesTransportationOERWY"
        | "OtherBusinessOperatingExpensesELE"
        | "OtherOperatingExpensesSPF"
        | "ShippingBusinessExpensesAndOtherOperatingExpensesWAT"
        | "TaxesOESideLine"
        | "TransportationOperatingExpensesOEAutomobile"
        | "TransportationOperatingExpensesOERWY"
        | "WarehouseBusinessExpensesOtherBusinessRevenueWAT" => "operating_expenses",

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

    let field = reviewed_jppfs_operating_expense_alias(&concept.1)?;
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
    reviewed_jppfs_operating_expense_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "operating_expenses" => Some(StatementCategory::ProfitAndLoss),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "operating_expenses" => Some("pl"),
        _ => base::statement_for_item_name(item_name),
    }
}

pub fn statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    canonicalize_concept(concept)
        .and_then(|canonical| statement_for_item_name(&canonical.item_name))
}

