use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP aliases whose local names carry the same accounting
/// meaning as an existing canonical field.
///
/// This layer is intentionally narrower than a statement-category classifier:
/// a tag that is merely a component of current assets, revenue, PPE, or SGA must
/// not be collapsed into the aggregate field. Such tags remain uncanonicalized
/// until a dedicated canonical field exists for their own accounting meaning.
/// When a reviewed component field already exists, map to that component field
/// instead of widening it to the parent aggregate.
fn reviewed_same_meaning_alias(local_name: &str) -> Option<&'static str> {
    if is_current_asset_alias(local_name) {
        return Some("current_assets");
    }
    if is_current_liability_alias(local_name) {
        return Some("current_liabilities");
    }
    if is_noncurrent_asset_alias(local_name) {
        return Some("non_current_assets");
    }
    if is_tangible_fixed_asset_alias(local_name) {
        return Some("tangible_fixed_assets");
    }
    if is_intangible_fixed_asset_alias(local_name) {
        return Some("intangible_fixed_assets");
    }
    if is_other_operating_revenue_alias(local_name) {
        return Some("other_operating_revenue");
    }
    if is_operating_revenue_alias(local_name) {
        return Some("revenue");
    }
    if is_operating_income_alias(local_name) {
        return Some("operating_income");
    }
    if is_operating_expenses_alias(local_name) {
        return Some("operating_expenses");
    }
    if is_cost_of_revenue_alias(local_name) {
        return Some("cost_of_revenue");
    }
    if is_depreciation_expense_alias(local_name) {
        return Some("depreciation");
    }
    if is_selling_general_admin_alias(local_name) {
        return Some("selling_general_admin");
    }
    None
}

fn is_current_asset_alias(local_name: &str) -> bool {
    matches!(local_name, "CurrentAssetsForIncidentalBusinessesCAGAS")
}

fn is_current_liability_alias(local_name: &str) -> bool {
    matches!(local_name, "CurrentLiabilitiesForIncidentalBusinessesCLGAS")
}

fn is_noncurrent_asset_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "NoncurrentAssetsAutomobileRWY"
            | "NoncurrentAssetsDevelopmentDevelopmentRWY"
            | "NoncurrentAssetsIncidentalBusinessIncidentalRWY"
            | "NoncurrentAssetsOtherOtherRWY"
            | "NoncurrentAssetsRailroadRWY"
            | "NoncurrentAssetsRealEstateHotelRWY"
            | "NoncurrentAssetsRelatedRWY"
            | "NoncurrentAssetsSideLineSideLineRWY"
    )
}

fn is_tangible_fixed_asset_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "PropertyPlantAndEquipmentAutomobileRWY"
            | "PropertyPlantAndEquipmentDevelopmentRWY"
            | "PropertyPlantAndEquipmentIncidentalRWY"
            | "PropertyPlantAndEquipmentNCARelatedRWY"
            | "PropertyPlantAndEquipmentNetAutomobileRWY"
            | "PropertyPlantAndEquipmentNetDevelopmentRWY"
            | "PropertyPlantAndEquipmentNetIncidentalRWY"
            | "PropertyPlantAndEquipmentNetOtherNCARWY"
            | "PropertyPlantAndEquipmentNetOtherRWY"
            | "PropertyPlantAndEquipmentNetRailroadRWY"
            | "PropertyPlantAndEquipmentNetRealEstateRWY"
            | "PropertyPlantAndEquipmentNetSideLineRWY"
            | "PropertyPlantAndEquipmentOtherNCARWY"
            | "PropertyPlantAndEquipmentOtherRWY"
            | "PropertyPlantAndEquipmentRailroadRWY"
            | "PropertyPlantAndEquipmentRealEstateRWY"
            | "PropertyPlantAndEquipmentSideLineRWY"
    )
}

fn is_intangible_fixed_asset_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "IntangibleAssetsAutomobileRWY"
            | "IntangibleAssetsDevelopmentRWY"
            | "IntangibleAssetsIncidentalRWY"
            | "IntangibleAssetsOtherNCARWY"
            | "IntangibleAssetsOtherRWY"
            | "IntangibleAssetsRailroadRWY"
            | "IntangibleAssetsRealEstateRWY"
            | "IntangibleAssetsRelatedRWY"
            | "IntangibleAssetsSideLineRWY"
    )
}

fn is_other_operating_revenue_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "AcceptForInterestFromSubsidiariesAndAffiliatesRevOA"
            | "AffiliatedCompanyBusinessManagementFeeRevOA"
            | "AffiliatesBusinessTrustIncomeRevOA"
            | "IncomeFromRadioBusinessRevOA"
            | "IncomeFromTelevisionBusinessRevOA"
            | "MiscellaneousRevenueORRelated"
            | "OtherOperatingRevenueCMD"
            | "OtherOperatingRevenueORIVT"
            | "OtherRevenueOROILTelecommunications"
            | "OtherRevenueORSideLine"
            | "OtherRevenueShipping"
    )
}

fn is_operating_revenue_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "OperatingRevenueTotalRWY"
            | "ShippingBusinessRevenueAndOtherOperatingRevenueWAT"
            | "WarehouseBusinessRevenueAndOtherOperatingRevenueWAT"
    )
}

fn is_operating_income_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "OperatingIncomeAutomobileRWY"
            | "OperatingIncomeDevelopmentRWY"
            | "OperatingIncomeIncidentalRWY"
            | "OperatingIncomeOtherRWY"
            | "OperatingIncomeRailroadOERWY"
            | "OperatingIncomeRealEstateRWY"
            | "OperatingIncomeRelatedRWY"
            | "OperatingIncomeSideLineRWY"
    )
}

fn is_operating_expenses_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "OperatingExpensesTotalRWY"
            | "ShippingBusinessExpensesAndOtherOperatingExpensesWAT"
            | "WarehouseBusinessExpensesOtherBusinessRevenueWAT"
    )
}

fn is_cost_of_revenue_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "CostOfGoodsSalesOEOther"
            | "CostOfSalesOEDevelopment"
            | "CostOfSalesOEIncidental"
            | "CostOfSalesOEOtherRWY"
            | "CostOfSalesOERealEstate"
            | "CostOfSalesOERelated"
            | "CostOfSalesOESideLineRWY"
    )
}

fn is_depreciation_expense_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "DepreciationOEAutomobile"
            | "DepreciationOEDevelopment"
            | "DepreciationOEIncidental"
            | "DepreciationOEOtherRWY"
            | "DepreciationOERealEstate"
            | "DepreciationOERelated"
            | "DepreciationOESideLineRWY"
            | "DepreciationOperatingExpensesIFRS"
    )
}

fn is_selling_general_admin_alias(local_name: &str) -> bool {
    matches!(
        local_name,
        "SellingGeneralAndAdministrativeExpensesOEDevelopment"
            | "SellingGeneralAndAdministrativeExpensesOEIncidental"
            | "SellingGeneralAndAdministrativeExpensesOEOther"
            | "SellingGeneralAndAdministrativeExpensesOERealEstate"
            | "SellingGeneralAndAdministrativeExpensesOERelated"
            | "SellingGeneralAndAdministrativeExpensesOESideLine"
    )
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_same_meaning_alias(&concept.1)?;
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
    reviewed_same_meaning_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    if item_name == "operating_expenses" {
        return Some(StatementCategory::ProfitAndLoss);
    }
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    if item_name == "operating_expenses" {
        return Some("pl");
    }
    base::statement_for_item_name(item_name)
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

