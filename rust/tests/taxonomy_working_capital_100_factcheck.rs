#![allow(dead_code)]

#[path = "../src/types.rs"]
mod types;

#[path = "../src/taxonomy_working_capital_100_reviewed_aliases.rs"]
mod taxonomy;

use taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
    concept_status, statement_for_concept,
};

const REVIEWED_WORKING_CAPITAL_100: &[(&str, &str)] = &[
    (
        "AccountReceivableMedicalIncomeCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivable", "accounts_receivable_trade"),
    (
        "AccountsReceivableAdvisoryFeeCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableAssetsIFRS", "accounts_receivable_trade"),
    ("AccountsReceivableAssetsINS", "accounts_receivable_trade"),
    ("AccountsReceivableCA", "accounts_receivable_trade"),
    (
        "AccountsReceivableCareBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCareBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCareBusinessCAAssets",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCommissionCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedConstructionCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedOperation",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedOperationCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableCompletedSurveyCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDevelopmentBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDevelopmentBusinessAndOther",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableDueFromFranchisedStores",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContracts",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsCNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromFranchisee",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSalesOnSideLineBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSideBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSideLineBusinessCNS",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableFromSubsidiariesAndAffiliatesTrade",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusinessCA",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableSideLineBusinessCA",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTenantsCA", "accounts_receivable_trade"),
    ("AccountsReceivableTrade1CA", "accounts_receivable_trade"),
    (
        "AccountsReceivableTradeAccountsReceivableTrade",
        "accounts_receivable_trade",
    ),
    ("AccountsReceivableTradeCA", "accounts_receivable_trade"),
    ("AccountsReceivableTradeCAINV", "accounts_receivable_trade"),
    ("AccountsReceivableTradeNet", "accounts_receivable_trade"),
    (
        "AccountsReceivablerealEstateBusiness",
        "accounts_receivable_trade",
    ),
    (
        "AccountsReceivableRealEstateBusinessAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableRealEstateBusinessAndOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOther",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
        "trade_and_other_current_receivables",
    ),
    (
        "ACcountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    ("AFactoringAccountPayable", "factoring_accounts_payable"),
    ("AFactoringAccountPayableCL", "factoring_accounts_payable"),
    ("AccountPayableFactoring", "factoring_accounts_payable"),
    (
        "AccountPayableForWorksInProgressAndOtherCL",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociates",
        "accounts_payable_current",
    ),
    (
        "AccountPaylableSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    ("AccountsPayable", "accounts_payable_current"),
    (
        "AccountsPayableAffiliatedCompanies",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableAffiliatedCompaniesCL",
        "accounts_payable_current",
    ),
    ("AccountsPayableCL", "accounts_payable_current"),
    ("AccountsPayableCLIFRS", "accounts_payable_current"),
    ("AccountsPayableCLINV", "accounts_payable_current"),
    ("AccountsPayableConsignment", "accounts_payable_current"),
    (
        "AccountsPayableDevelopmentBusinessAndOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableDevelopmentProjectsInProgressCNS",
        "accounts_payable_for_construction_contracts_current",
    ),
    (
        "AccountsPayableDueToMemberMerchants",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFacilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableFacility",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForAffiliatedCompanyCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableForEquipmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForEquipmentInvestmentCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForEquipmentLiabilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilities",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    (
        "AccountsPayableForFacilitiesvCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AccountsPayableForFactoring", "factoring_accounts_payable"),
    (
        "AccountsPayableForFactoringCL",
        "factoring_accounts_payable",
    ),
    ("AccountsPayableForFranchisee", "accounts_payable_current"),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFormSubsideiariesAndAssociateOtherCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesOtherCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAssociatesCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAssociatesOther",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableInstallmentPurchase",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableInstallmentPurchaseCL",
        "accounts_payable_current",
    ),
    (
        "AccountsPayableOfBuyNoncurrentAssetsCL",
        "accounts_payable_for_property_plant_and_equipment_current",
    ),
    ("AccountsPayableOfCollectionCL", "accounts_payable_current"),
    ("AccountsPayableOnMarginCL", "accounts_payable_current"),
    ("AccountsPayableOtherBusinessCL", "accounts_payable_current"),
    (
        "AccountPayableTradeForFranchisedStoreCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableFromSubsidiariesAndAffiliatesTradeCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL",
        "accounts_payable_trade",
    ),
    ("AccountsPayableTrade", "accounts_payable_trade"),
    ("AccountsPayableTradeCL", "accounts_payable_trade"),
    (
        "AccountsPayableTradeForFranchisedStores",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeForFranchisedStoresCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeFromSubsidiariesAndAffiliates",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeLiabilitiesIFRS",
        "accounts_payable_trade",
    ),
    (
        "AccountsPayableTradeToSubsidiariesAndAffiliatesCL",
        "accounts_payable_trade",
    ),
    (
        "AccountsReceivableOtherBusinessCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherBusinessCAWAT",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromFactor",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromSubsidiariesAndAffiliates",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherMaterialsCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherNetCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivableOtherOfStorageAgencyCA",
        "trade_and_other_current_receivables",
    ),
    (
        "AccountsReceivablePaidProvisionCA",
        "trade_and_other_current_receivables",
    ),
];

fn jppfs(local_name: &str) -> types::ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> types::ConceptKey {
    (
        "https://issuer.example.com/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

#[test]
fn reviewed_working_capital_batch_covers_exactly_100_tags() {
    assert_eq!(REVIEWED_WORKING_CAPITAL_100.len(), 100);
}

#[test]
fn reviewed_working_capital_aliases_are_balance_sheet_semantic_aliases() {
    for (local_name, expected_field) in REVIEWED_WORKING_CAPITAL_100 {
        let concept = jppfs(local_name);
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

        assert_eq!(resolved.item_name, *expected_field);
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::BalanceSheet),
            "{local_name} should stay in balance-sheet working capital scope"
        );
        assert_eq!(statement_for_concept(&concept), Some("bs"));
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
    }
}

#[test]
fn reviewed_working_capital_aliases_are_namespace_gated() {
    for (local_name, expected_field) in REVIEWED_WORKING_CAPITAL_100 {
        assert_ne!(
            canonicalize_concept(&extension(local_name)).map(|entry| entry.item_name),
            Some((*expected_field).to_string()),
            "{local_name} must not be collapsed when it appears in an issuer extension namespace"
        );
    }
}

#[test]
fn and_other_receivable_aliases_do_not_narrow_to_trade_receivables() {
    for local_name in [
        "AccountsReceivableRealEstateBusinessAndOther",
        "AccountsReceivableRealEstateBusinessAndOtherCA",
        "AccountsReceivableFromCompletedConstructionContractsAndOther",
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
        "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name)).unwrap();
        assert_eq!(resolved.item_name, "trade_and_other_current_receivables");
        assert_ne!(resolved.item_name, "accounts_receivable_trade");
    }
}

#[test]
fn specific_payable_aliases_do_not_collapse_to_trade_payables() {
    for (local_name, expected_field) in [
        (
            "AccountPayableForWorksInProgressAndOtherCL",
            "accounts_payable_for_construction_contracts_current",
        ),
        (
            "AccountsPayableForEquipmentCL",
            "accounts_payable_for_property_plant_and_equipment_current",
        ),
        ("AFactoringAccountPayableCL", "factoring_accounts_payable"),
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name)).unwrap();
        assert_eq!(resolved.item_name, expected_field);
        assert_ne!(resolved.item_name, "accounts_payable_trade");
    }
}
