use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_semantic_concept, classify_item_name,
    concept_status, is_excluded_concept, statement_for_concept,
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

const ACCUMULATED_DEPRECIATION_AND_IMPAIRMENT_ALIASES: &[&str] = &[
    "AccumulatedDepreciationAndImpairmentAssetsForRentPPE",
    "AccumulatedDepreciationAndImpairmentLossBuildings",
    "AccumulatedDepreciationAndImpairmentLossBuildingsAndAccompanyingFacilities",
    "AccumulatedDepreciationAndImpairmentLossBuildingsAndStructures",
    "AccumulatedDepreciationAndImpairmentLossLeaseAssetsPPE",
    "AccumulatedDepreciationAndImpairmentLossMachineryAndEquipment",
    "AccumulatedDepreciationAndImpairmentLossMachineryEquipmentAndVehicles",
    "AccumulatedDepreciationAndImpairmentLossOilTanksPPE",
    "AccumulatedDepreciationAndImpairmentLossOtherPPE",
    "AccumulatedDepreciationAndImpairmentLossPPEByGroup",
    "AccumulatedDepreciationAndImpairmentLossPpe",
    "AccumulatedDepreciationAndImpairmentLossRealEstateForInvestment",
    "AccumulatedDepreciationAndImpairmentLossRental",
    "AccumulatedDepreciationAndImpairmentLossRightOfUseAssets",
    "AccumulatedDepreciationAndImpairmentLossRightOfUseAssetsPPE",
    "AccumulatedDepreciationAndImpairmentLossStructures",
    "AccumulatedDepreciationAndImpairmentLossToolsFurnitureAndFixtures",
    "AccumulatedDepreciationAndImpairmentLossToolsFurnitureFixturesAndDies",
    "AccumulatedDepreciationAndImpairmentLossToolsFurnitureFixturesAndDiesPPE",
    "AccumulatedDepreciationAndImpairmentLossVehicles",
    "AccumulatedDepreciationAndImpairmentLossVehiclesToolsFurnitureAndFixtures",
];

const ACCUMULATED_DEPRECIATION_ALIASES: &[&str] = &[
    "AccumulatedDeoreclationMetalicMold",
    "AccumulatedDepereciationSimpleBuildingtPPE",
    "AccumulatedDepereciationVehiclesForLeaseAssetsForLeasePPE",
    "AccumulatedDepereciationVehiclesForLeasePPE",
    "AccumulatedDepreciation",
    "AccumulatedDepreciationAircraft",
    "AccumulatedDepreciationAmusementEquipmentsPPE",
    "AccumulatedDepreciationAmusementMachinesAndFacilities",
    "AccumulatedDepreciationAssetsForLease",
    "AccumulatedDepreciationAssetsForLeasePPE",
    "AccumulatedDepreciationAssetsForRent",
    "AccumulatedDepreciationAssetsForRentPPE",
    "AccumulatedDepreciationAutomobileRWY",
    "AccumulatedDepreciationBowlingEquipmentsFacilitiesPPE",
    "AccumulatedDepreciationBuildingNCATelecommunications",
    "AccumulatedDepreciationBuildings",
    "AccumulatedDepreciationBuildingsAndAccompanyingFacilities",
    "AccumulatedDepreciationBuildingsAndBuildingsAndAccompanyingFacilities",
    "AccumulatedDepreciationBuildingsAndStructures",
    "AccumulatedDepreciationBuildingsAndStructuresForOwnUse",
    "AccumulatedDepreciationBuildingsAndStructuresForRent",
    "AccumulatedDepreciationBuildingsAndStructuresInTrustPPE",
    "AccumulatedDepreciationBuildingsAndStructuresRealEstateForInvestmentIOA",
    "AccumulatedDepreciationBuildingsForOwnUse",
    "AccumulatedDepreciationBuildingsForRent",
    "AccumulatedDepreciationBuildingsInTrust",
    "AccumulatedDepreciationBuildingsInTrustINV",
    "AccumulatedDepreciationBuildingsInTrustPPE",
    "AccumulatedDepreciationBuildingsStructuresAndOilTanks",
    "AccumulatedDepreciationCleanEenergyPowerPlantEquipmentPPE",
    "AccumulatedDepreciationCostumesForRentPPE",
    "AccumulatedDepreciationDevelopmentRWY",
    "AccumulatedDepreciationDocksAndBuildingBerthsVES",
    "AccumulatedDepreciationDredger",
    "AccumulatedDepreciationEquipment",
    "AccumulatedDepreciationEquipmentSPF",
    "AccumulatedDepreciationExhibitionBuildings",
    "AccumulatedDepreciationFacilitiesAttachedToBuildings",
    "AccumulatedDepreciationFinalDisposalSitePPE",
    "AccumulatedDepreciationFixedAssetsForRentNCL",
    "AccumulatedDepreciationFixtures",
    "AccumulatedDepreciationForRightOfUseAssetPPE",
    "AccumulatedDepreciationForRightOfUseAssetRightOfUseAsset",
    "AccumulatedDepreciationFunrniture",
    "AccumulatedDepreciationFurnacesMachineryEquipmentAndVehicles",
    "AccumulatedDepreciationFurniture",
    "AccumulatedDepreciationFurnitureAndEquipmentPPE",
    "AccumulatedDepreciationFurnitureAndFixtures",
    "AccumulatedDepreciationFurnitureAndFixturesCMD",
    "AccumulatedDepreciationFurnitureAndFixturesForOwnUse",
    "AccumulatedDepreciationFurnitureAndFixturesForRent",
    "AccumulatedDepreciationFurnitureAndFixturesPPE",
    "AccumulatedDepreciationFurnitureForRentAssetsForRentTract",
    "AccumulatedDepreciationGasPipe",
    "AccumulatedDepreciationGoodsForRent",
    "AccumulatedDepreciationIncidentalRWY",
    "AccumulatedDepreciationIOA",
    "AccumulatedDepreciationIOAByGroup",
    "AccumulatedDepreciationInvestmentsInBuilding",
    "AccumulatedDepreciationKaraokeCabinAndRestaurantFacilitiesPPE",
    "AccumulatedDepreciationKaraokeEquipmentForRentalPPE",
    "AccumulatedDepreciationKitchenFacilities",
    "AccumulatedDepreciationLandfills",
    "AccumulatedDepreciationLeaseAssetPPE",
    "AccumulatedDepreciationLeaseAssetsForOwnUse",
    "AccumulatedDepreciationLeaseAssetsNCATelecommunications",
    "AccumulatedDepreciationLeaseAssetsPPE",
    "AccumulatedDepreciationLeasedAsset",
    "AccumulatedDepreciationLineFacilitiesNCATelecommunications",
    "AccumulatedDepreciationLocalNCATelecommunications",
    "AccumulatedDepreciationMachineEquipmentPPE",
    "AccumulatedDepreciationMachinery",
    "AccumulatedDepreciationMachineryAndEquipment",
    "AccumulatedDepreciationMachineryAndEquipmentForOwnUse",
    "AccumulatedDepreciationMachineryAndEquipmentInTrustINV",
    "AccumulatedDepreciationMachineryAndEquipmentNCATelecommunications",
    "AccumulatedDepreciationMachineryAndEquipmentOwnUsedPPE",
    "AccumulatedDepreciationMachineryAndEquipmentRentPPE",
    "AccumulatedDepreciationMachineryEquipment",
    "AccumulatedDepreciationMachineryEquipmentAndVehicles",
    "AccumulatedDepreciationMachineryEquipmentAndVehiclesForOwnUse",
    "AccumulatedDepreciationMachineryEquipmentAndVehiclesInTrustPPE",
    "AccumulatedDepreciationOtherNoncurrentAssetsRWY",
    "AccumulatedDepreciationOtherRWY",
    "AccumulatedDepreciationPPERelatedRWY",
    "AccumulatedDepreciationRailroadRWY",
    "AccumulatedDepreciationRealEstateRWY",
    "AccumulatedDepreciationSideLineRWY",
    "AccumulatedDepreciationSubmarineNCATelecommunications",
    "AccumulatedDepreciationTerminalNCATelecommunications",
    "AccumulatedDepreciationVehicle",
    "AccumulatedDepreciationVehicles",
    "AccumulatedDepreciationVehiclesCMD",
    "AccumulatedDepreciationVehiclesOnOperatingLeasesPPE",
];

fn assert_balance_sheet_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed semantic alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(
        !is_excluded_concept(local_name),
        "{local_name} should be treated as reviewed, not excluded"
    );
}

#[test]
fn accumulated_depreciation_review_covers_exactly_115_tags() {
    assert_eq!(ACCUMULATED_DEPRECIATION_AND_IMPAIRMENT_ALIASES.len(), 21);
    assert_eq!(ACCUMULATED_DEPRECIATION_ALIASES.len(), 94);
    assert_eq!(
        ACCUMULATED_DEPRECIATION_AND_IMPAIRMENT_ALIASES.len()
            + ACCUMULATED_DEPRECIATION_ALIASES.len(),
        115
    );
}

#[test]
fn accumulated_depreciation_aliases_preserve_balance_sheet_scope() {
    for local_name in ACCUMULATED_DEPRECIATION_AND_IMPAIRMENT_ALIASES {
        assert_balance_sheet_alias(local_name, "accumulated_depreciation_and_impairment_losses");

        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        assert_ne!(resolved.item_name, "depreciation");
        assert_ne!(resolved.item_name, "depreciation_cf_adjustment");
        assert_ne!(resolved.item_name, "impairment_loss");
    }

    for local_name in ACCUMULATED_DEPRECIATION_ALIASES {
        assert_balance_sheet_alias(local_name, "accumulated_depreciation");

        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        assert_ne!(resolved.item_name, "depreciation");
        assert_ne!(resolved.item_name, "depreciation_cf_adjustment");
    }
}

#[test]
fn accumulated_depreciation_aliases_do_not_apply_to_extension_namespaces() {
    for local_name in [
        ACCUMULATED_DEPRECIATION_AND_IMPAIRMENT_ALIASES[0],
        ACCUMULATED_DEPRECIATION_ALIASES[0],
    ] {
        assert_ne!(
            canonicalize_semantic_concept(&extension(local_name)).map(|entry| entry.item_name),
            canonicalize_semantic_concept(&jppfs(local_name)).map(|entry| entry.item_name),
            "{local_name} should stay namespace-gated"
        );
    }
}
