use std::collections::HashSet;

use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
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
        "https://example.com/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

const BULK_CURRENT_ACCOUNTS_PAYABLE_ALIASES: &[&str] = &[
    "AFactoringAccountPayable",
    "AFactoringAccountPayableCL",
    "AccountPayableAndAllowanceForEvaluationLossOfDerivativesCL",
    "AccountPayableForWorksInProgressAndOtherCL",
    "AccountPayableTradeForFranchisedStoreCL",
    "AccountsPayableAffiliatedCompaniesCL",
    "AccountsPayableConsignment",
    "AccountsPayableDevelopmentBusinessAndOther",
    "AccountsPayableDevelopmentProjectsInProgressCNS",
    "AccountsPayableDirectorsBonuses",
    "AccountsPayableDueToMemberMerchants",
    "AccountsPayableFacilities",
    "AccountsPayableFacility",
    "AccountsPayableForAffiliatedCompanyCL",
    "AccountsPayableForConstructionContractsAndOtherCLCNS",
    "AccountsPayableForConstructionContractsAndOtherCNS",
    "AccountsPayableForConstructionContractsCNS",
    "AccountsPayableForFacilities",
    "AccountsPayableForFranchisee",
    "AccountsPayableForSideLineBusinessCNS",
    "AccountsPayableFromSubsidiariesAndAffiliatesOtherCL",
    "AccountsPayableFromSubsidiariesAndAffiliatesTradeCL",
    "AccountsPayableFromSubsidiariesAndAssociatesCL",
    "AccountsPayableGasolineTax",
    "AccountsPayableInstallmentPurchaseCL",
    "AccountsPayableOfBuyNoncurrentAssetsCL",
    "AccountsPayableOfCollectionCL",
    "AccountsPayableOnMarginCL",
    "AccountsPayableOtherAndAccruedExpenses",
    "AccountsPayableOtherFromSubsidiariesAndAffiliatesCL",
    "AccountsPayableOtherToAffiliatesCL",
    "AccountsPayableOtherToSubsidiariesAndAffiliatesCL",
    "AccountsPayableOtherToSubsidiariesAndAffiliatesCLCL",
    "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL",
    "AccountsPayablePropertyPlantAndEquipmentCL",
    "AccountsPayablePurchaseOfCredit",
    "AccountsPayableRentalAssetsCLLiabilities",
    "AccountsPayableSurveyingCL",
    "AccountsPayableToAffiliatedCompanies",
    "AccountsPayableToSubsidiariesAndAffiliatesCL",
    "AccountsPayableToSubsidiariesAndAssociatesCL",
    "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL",
    "AccountsPayableTradeAndAccountsPayableForConstructionContracts",
    "AccountsPayableTradeCL",
    "AccountsPayableTradeForFranchisedStoresCL",
    "AccountsPayableTradeLiabilitiesIFRS",
    "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL",
    "AccountsPayableTradeToSubsidiariesAndAffiliatesCL",
    "AccountsPayableFromSubsidiariesAndAssociatesOther",
    "AccountsPayableOtherToSubsidiariesAndAffiliates",
];

const BULK_CURRENT_RECEIVABLE_ALIASES: &[&str] = &[
    "AccountReceivableMedicalIncomeCA",
    "AccountsReceivableCA",
    "AccountsReceivableCareBusinessCA",
    "AccountsReceivableCareBusinessCAAssets",
    "AccountsReceivableCommissionCA",
    "AccountsReceivableCompletedConstructionCA",
    "AccountsReceivableCompletedOperationCA",
    "AccountsReceivableDevelopmentProjectsCACNS",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS",
    "AccountsReceivableFromCompletedConstructionContractsCA",
    "AccountsReceivableFromCompletedConstructionContractsContractAssetsCACNS",
    "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS",
    "AccountsReceivableFromSideBusinessCA",
    "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA",
    "AccountsReceivableOtherAndContractAssetCA",
    "AccountsReceivableRealEstateBusinessAndOtherCA",
    "AccountsReceivableRealEstateBusinessCA",
    "AccountsReceivableReceivablesArisingFromContractsWithCustomersCurrentAssets",
    "AccountsReceivableSideLineBusinessCA",
    "AccountsReceivableTenantsCA",
    "AccountsReceivableTrade1CA",
    "AccountsReceivableTradeAndAccountsReceivableFromCompletedConstructionContractsCA",
    "AccountsReceivableTradeAndContractAssetsCA",
    "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssetsCA",
    "AccountsReceivableTradeAndContractCA",
    "AccountsReceivableTradeCA",
    "AccountsReceivableTradeCAINV",
    "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA",
    "AccountsReceivablesAndContractAssetsCA",
    "AccountsreceivabletradeandcontractassetsCA",
    "AccruedConsumptionTaxRefundCA",
    "AccruedCorporationTaxCA",
    "AccruedIncomeOtherFromSubsidiariesAndAffiliatesCA",
    "AccruedIncomeTaxCA",
    "AccruedIncomeTaxesCA",
    "AccruedInvestmentTrustManagementFeeCAIVT",
    "AccruedPremiumsAssetsINS",
    "AccruedRevenueCAIFRS",
    "AccruedTaxCA",
    "AccountsReceivableAdvisoryFeeCA",
    "AccountsReceivableAndContractAssetsCA",
    "AccountsReceivableFromCompletedConstructionContractsAccountsReceivableRealEstateBusinessAndAccountsReceivableTradeCA",
    "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsNotesReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
];

fn assert_bs_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

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
}

fn assert_not_collapsed_to(local_name: &str, disallowed_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_ne!(
        resolved.item_name, disallowed_field,
        "{local_name} must stay in its fact-checked broad working-capital bucket"
    );
}

fn expected_bulk_current_payable_alias(local_name: &str) -> &'static str {
    match local_name {
        "AFactoringAccountPayable" | "AFactoringAccountPayableCL" => "factoring_accounts_payable",
        "AccountPayableAndAllowanceForEvaluationLossOfDerivativesCL" => {
            "accounts_payable_and_derivative_valuation_allowance_current"
        }
        "AccountPayableForWorksInProgressAndOtherCL"
        | "AccountsPayableDevelopmentProjectsInProgressCNS"
        | "AccountsPayableForConstructionContractsAndOtherCLCNS"
        | "AccountsPayableForConstructionContractsAndOtherCNS"
        | "AccountsPayableForConstructionContractsCNS"
        | "AccountsPayableTradeAndAccountsPayableForConstructionContracts"
        | "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL"
        | "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL" => {
            "accounts_payable_for_construction_contracts_current"
        }
        "AccountPayableTradeForFranchisedStoreCL"
        | "AccountsPayableForSideLineBusinessCNS"
        | "AccountsPayableFromSubsidiariesAndAffiliatesTradeCL"
        | "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL"
        | "AccountsPayableTradeCL"
        | "AccountsPayableTradeForFranchisedStoresCL"
        | "AccountsPayableTradeLiabilitiesIFRS"
        | "AccountsPayableTradeToSubsidiariesAndAffiliatesCL" => "accounts_payable_trade",
        "AccountsPayableFacilities"
        | "AccountsPayableFacility"
        | "AccountsPayableForFacilities"
        | "AccountsPayableOfBuyNoncurrentAssetsCL"
        | "AccountsPayablePropertyPlantAndEquipmentCL" => {
            "accounts_payable_for_property_plant_and_equipment_current"
        }
        "AccountsPayableOtherAndAccruedExpenses" => "accounts_payable_other_and_accrued_expenses",
        "AccountsPayableRentalAssetsCLLiabilities" => "accounts_payable_for_rental_assets_current",
        "AccountsPayableSurveyingCL" => "accounts_payable_surveying_current",
        _ => "accounts_payable_current",
    }
}

fn expected_bulk_current_receivable_alias(local_name: &str) -> &'static str {
    match local_name {
        "AccountReceivableMedicalIncomeCA"
        | "AccountsReceivableCA"
        | "AccountsReceivableCareBusinessCA"
        | "AccountsReceivableCareBusinessCAAssets"
        | "AccountsReceivableCommissionCA"
        | "AccountsReceivableCompletedConstructionCA"
        | "AccountsReceivableCompletedOperationCA"
        | "AccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromSideBusinessCA"
        | "AccountsReceivableRealEstateBusinessCA"
        | "AccountsReceivableSideLineBusinessCA"
        | "AccountsReceivableTenantsCA"
        | "AccountsReceivableTrade1CA"
        | "AccountsReceivableTradeCA"
        | "AccountsReceivableTradeCAINV"
        | "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA" => {
            "accounts_receivable_trade"
        }
        "AccountsReceivableAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsContractAssetsCACNS"
        | "AccountsReceivableOtherAndContractAssetCA"
        | "AccountsReceivableTradeAndContractAssetsCA"
        | "AccountsReceivableTradeAndContractCA"
        | "AccountsReceivablesAndContractAssetsCA"
        | "AccountsreceivabletradeandcontractassetsCA" => "accounts_receivable_and_contract_assets",
        "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssetsCA" => {
            "notes_and_accounts_receivable_trade_and_contract_assets"
        }
        _ => "trade_and_other_current_receivables",
    }
}

#[test]
fn operating_specific_payable_keeps_broad_current_payable_scope() {
    let local_name = "AccountsPayableOperatingSpecific";
    assert_bs_alias(local_name, "accounts_payable_current");

    let resolved = canonicalize_concept(&jppfs(local_name)).unwrap();
    assert_ne!(resolved.item_name, "accounts_payable_trade");
}

#[test]
fn unreviewed_current_working_capital_tags_are_classified_to_broad_buckets() {
    assert_bs_alias(
        "AccountsForNonReceivedSecuritiesAndOthersCLSEC",
        "accounts_payable_current",
    );
    assert_bs_alias(
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesCA",
        "trade_and_other_current_receivables",
    );
    assert_bs_alias(
        "AccountsReceivableOtherMaterialsCA",
        "trade_and_other_current_receivables",
    );
}

#[test]
fn factchecked_bulk_working_capital_aliases_cover_100_previously_identity_only_tags() {
    let aliases: Vec<&str> = BULK_CURRENT_ACCOUNTS_PAYABLE_ALIASES
        .iter()
        .chain(BULK_CURRENT_RECEIVABLE_ALIASES.iter())
        .copied()
        .collect();
    let unique_aliases: HashSet<&str> = aliases.iter().copied().collect();

    assert_eq!(aliases.len(), 100);
    assert_eq!(unique_aliases.len(), 100);

    for local_name in BULK_CURRENT_ACCOUNTS_PAYABLE_ALIASES {
        let expected_field = expected_bulk_current_payable_alias(local_name);
        assert_bs_alias(local_name, expected_field);
        if expected_field != "accounts_payable_trade" {
            assert_not_collapsed_to(local_name, "accounts_payable_trade");
        }
    }
    for local_name in BULK_CURRENT_RECEIVABLE_ALIASES {
        let expected_field = expected_bulk_current_receivable_alias(local_name);
        assert_bs_alias(local_name, expected_field);
        if expected_field != "accounts_receivable_trade" {
            assert_not_collapsed_to(local_name, "accounts_receivable_trade");
        }
    }
}

#[test]
fn factchecked_working_capital_aliases_are_namespace_gated() {
    for local_name in [
        "AccountsPayableOperatingSpecific",
        "AccountsForNonReceivedSecuritiesAndOthersCLSEC",
        "AccountsReceivableOtherFromSubsidiariesAndAffiliatesCA",
        "AccountsReceivableOtherMaterialsCA",
    ] {
        assert!(canonicalize_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }

    for local_name in BULK_CURRENT_ACCOUNTS_PAYABLE_ALIASES
        .iter()
        .chain(BULK_CURRENT_RECEIVABLE_ALIASES.iter())
    {
        assert!(canonicalize_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
