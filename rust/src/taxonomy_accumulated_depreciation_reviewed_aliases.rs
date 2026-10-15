use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_accumulated_depreciation_alias(local_name: &str) -> Option<&'static str> {
    if is_land_only_accumulated_depreciation_name(local_name) {
        return None;
    }

    let field = if is_accumulated_depreciation_and_impairment_name(local_name) {
        "accumulated_depreciation_and_impairment_losses"
    } else if is_accumulated_depreciation_name(local_name) {
        "accumulated_depreciation"
    } else {
        return None;
    };
    Some(field)
}

fn is_accumulated_depreciation_and_impairment_name(local_name: &str) -> bool {
    local_name.starts_with("AccumulatedDepreciationAndImpairment")
}

fn is_accumulated_depreciation_name(local_name: &str) -> bool {
    local_name.starts_with("AccumulatedDepreciation")
        || local_name.starts_with("AccumulatedDepereciation")
        || local_name.starts_with("AccumulatedDeoreclation")
        || local_name.starts_with("AccumulatedDeprection")
        || local_name.starts_with("AccumulatedDepriciation")
}

fn is_land_only_accumulated_depreciation_name(local_name: &str) -> bool {
    matches!(
        local_name,
        "AccumulatedDepreciationLand"
            | "AccumulatedDepreciationLandInTrustPPE"
            | "AccumulatedDepreciationLandUsedForMiningOperations"
    )
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_accumulated_depreciation_alias(&concept.1)?;
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
    reviewed_jppfs_accumulated_depreciation_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "accumulated_depreciation" | "accumulated_depreciation_and_impairment_losses" => {
            Some(StatementCategory::BalanceSheet)
        }
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "accumulated_depreciation" | "accumulated_depreciation_and_impairment_losses" => Some("bs"),
        _ => base::statement_for_item_name(item_name),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn q(local: &str) -> ConceptKey {
        (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
            local.to_string(),
        )
    }

    #[test]
    fn reviewed_accumulated_depreciation_variants_are_balance_sheet_contra_assets() {
        for concept in [
            "AccumulatedDepreciationRightOfUseAssets",
            "AccumulatedDepreciationRightOfUseAssetsPPE",
            "AccumulatedDepreciationVehiclesToolsFurnitureAndFixtures",
            "AccumulatedDepreciationVessels",
            "AccumulatedDepreciationWells",
            "AccumulatedDeprectionRightOfUseAssets",
            "AccumulatedDepriciationRightOfUseAssetsPPE",
        ] {
            assert!(
                matches!(
                    concept_status(&q(concept)),
                    TaxonomyConceptStatus::Semantic(canonical)
                        if canonical.item_name == "accumulated_depreciation"
                ),
                "{concept} must be a reviewed semantic accumulated-depreciation alias"
            );
            assert_eq!(raw_statement_for_concept(&q(concept)), Some("bs"));
        }
    }

    #[test]
    fn land_only_depreciation_names_are_not_blanket_grouped() {
        assert!(canonicalize_semantic_concept(&q("AccumulatedDepreciationLand")).is_none());
        assert!(
            canonicalize_semantic_concept(&q("AccumulatedDepreciationLandInTrustPPE")).is_none()
        );
        assert!(
            canonicalize_semantic_concept(&q("AccumulatedDepreciationLandUsedForMiningOperations"))
                .is_none()
        );
    }
}

