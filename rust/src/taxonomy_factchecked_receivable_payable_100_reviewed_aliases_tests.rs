use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_semantic_concept, classify_item_name,
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
        "https://filer.example.co.jp/taxonomy/2026".to_string(),
        local_name.to_string(),
    )
}

const TRADE_RECEIVABLE_ALIASES: &[&str] = &[
    "AccountReceivableMedicalIncomeCA",
    "AccountsReceivableAdvisoryFeeCA",
    "AccountsReceivableCareBusiness",
    "AccountsReceivableCareBusinessCA",
    "AccountsReceivableCareBusinessCAAssets",
    "AccountsReceivableCommissionCA",
    "AccountsReceivableCompletedConstructionCA",
    "AccountsReceivableCompletedOperation",
    "AccountsReceivableCompletedOperationCA",
    "AccountsReceivableCompletedSurveyCA",
    "AccountsReceivableDevelopmentBusiness",
    "AccountsReceivableDueFromFranchisedStores",
    "AccountsReceivableFromFranchisee",
    "AccountsReceivableFromSalesOnSideLineBusiness",
    "AccountsReceivableFromSideBusinessCA",
    "AccountsReceivableFromSideLineBusinessCNS",
    "AccountsReceivableFromSubsidiariesAndAffiliatesTrade",
    "AccountsReceivableInstallment",
    "AccountsReceivableRealEstateBusiness",
    "AccountsReceivableRealEstateBusinessCA",
    "AccountsReceivableSideLineBusinessCA",
    "AccountsReceivableTenantsCA",
    "AccountsReceivableTrade1CA",
    "AccountsReceivableTradeAccountsReceivableTrade",
    "AccountsReceivableTradeCAINV",
    "AccountsReceivableTradeNet",
    "AccountsReceivableTradeReceivablesFromContractsWithCustomers",
    "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA",
    "AccountsReceivablerealEstateBusiness",
    "AccountsReceivableFromCompletedConstructionContractsCNS",
    "AccounAccountsReceivableFromCompletedConstructionContractstsReceivableFromCompletedConstructionContracts",
];

const BROAD_CURRENT_RECEIVABLE_ALIASES: &[&str] = &[
    "AccountsReceivableDevelopmentBusinessAndOther",
    "AccountsReceivableFromSubsidiariesAndAssociatesOtherCA",
    "AccountsReceivableOtherAndContractAssetCA",
    "AccountsReceivableOtherNotesConsolidatedBalanceSheet",
    "AccountsReceivableRealEstateBusinessAndOther",
    "AccountsReceivableFromCompletedConstructionContractsAndOther",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssets",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA",
    "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS",
    "AccruedConsumptionTaxRefundCA",
    "AccruedIncome",
    "AccruedIncomeOtherFromSubsidiariesAndAffiliates",
    "AccruedIncomeOtherFromSubsidiariesAndAffiliatesCA",
    "AccruedInterest",
    "AccruedInterestFromSubsidiariesAndAssociatesOther",
    "AccruedInvestmentTrustManagementFeeCAIVT",
    "AccruedPremiumsAssetsINS",
    "AccruedRentalIncome",
    "AccruedTaxCA",
    "AccountsReceivableOtherFromSubsidiariesAndAffiliates",
    "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA",
    "AccountsReceivableOtherFromFactor",
    "AccountsReceivableOtherNetCA",
    "AccountsReceivableOtherOfStorageAgencyCA",
    "AccountsReceivablePaidProvisionCA",
    "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA",
    "AccountsReceivableRentalCA",
    "AccountsReceivableRentalCAAssets",
    "AccountsReceivableOtherBusinessCAWAT",
    "AccountsReceivableCompletedConstructionContractsCA",
    "AccountsReceivableFromCompletedConstructionContractsCA",
    "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA",
    "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS",
];

const CURRENT_PAYABLE_ALIASES: &[&str] = &[
    "AccountPaylableSubsidiariesAndAssociates",
    "AccountsPayableAffiliatedCompanies",
    "AccountsPayableConsignment",
    "AccountsPayableDevelopmentBusinessAndOther",
    "AccountsPayableDueToMemberMerchants",
    "AccountsPayableFacilities",
    "AccountsPayableFacility",
    "AccountsPayableForFranchisee",
    "AccountsPayableForSideLineBusinessCNS",
    "AccountsPayableFormSubsideiariesAndAssociateOther",
    "AccountsPayableFromSubsidiariesAndAffiliatesOther",
    "AccountsPayableFromSubsidiariesAndAssociatesOther",
    "AccountsPayableInstallmentPurchase",
    "AccountsPayableInstallmentPurchaseCL",
    "AccountsPayableOtherAndAccruedExpenses",
    "AccountsPayableOtherFromSubsidiariesAndAssociates",
    "AccountsPayableOtherToSubsidiariesAndAffiliates",
    "AccountsPayablePurchaseOfCredit",
    "AccountsPayableRealEstate",
    "AccountsPayableSurveyingSpecific",
    "AccountsPayableToAffiliatedCompanies",
    "AccountsPayableToSubsidiariesAndAssociates",
    "AccountsPayableOtherToSubsidiariesAndAffiliatesCLCL",
    "AccountsPayableFromSubsidiariesAndAssociatesCL",
    "AccountsPayableOfCollectionCL",
    "AccountsPayableOnMarginCL",
    "AccountsPayableOtherToAffiliatesCL",
    "AccountsPayableOtherFromSubsidiariesAndAffiliatesCL",
    "AccountsPayableToSubsidiariesAndAffiliatesCL",
];

fn assert_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
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

fn expected_other_current_receivable_alias(local_name: &str) -> &'static str {
    match local_name {
        "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssets"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS"
        | "AccountsReceivableOtherAndContractAssetCA" => "accounts_receivable_and_contract_assets",
        "AccountsReceivableCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS" => {
            "accounts_receivable_trade"
        }
        _ => "trade_and_other_current_receivables",
    }
}

fn expected_current_payable_alias(local_name: &str) -> &'static str {
    match local_name {
        "AccountsPayableFacilities"
        | "AccountsPayableFacility"
        | "AccountsPayableForFacilities" => {
            "accounts_payable_for_property_plant_and_equipment_current"
        }
        "AccountsPayableForSideLineBusinessCNS" => "accounts_payable_trade",
        "AccountsPayableOtherAndAccruedExpenses" => "accounts_payable_other_and_accrued_expenses",
        "AccountsPayableRealEstate" => "accounts_payable_real_estate_current",
        "AccountsPayableSurveyingSpecific" => "accounts_payable_surveying_current",
        _ => "accounts_payable_current",
    }
}

#[test]
fn reviewed_receivable_payable_aliases_cover_exactly_100_tags() {
    let total = TRADE_RECEIVABLE_ALIASES.len()
        + BROAD_CURRENT_RECEIVABLE_ALIASES.len()
        + CURRENT_PAYABLE_ALIASES.len();

    assert_eq!(total, 100);
}

#[test]
fn customer_and_trade_receivable_aliases_use_trade_receivable_bucket() {
    for local_name in TRADE_RECEIVABLE_ALIASES {
        assert_alias(local_name, "accounts_receivable_trade");

        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        assert_ne!(resolved.item_name, "trade_and_other_current_receivables");
    }
}

#[test]
fn other_current_receivable_aliases_preserve_reviewed_scope() {
    for local_name in BROAD_CURRENT_RECEIVABLE_ALIASES {
        let expected_field = expected_other_current_receivable_alias(local_name);
        assert_alias(local_name, expected_field);

        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        if expected_field != "accounts_receivable_trade" {
            assert_ne!(resolved.item_name, "accounts_receivable_trade");
        }
        assert_ne!(resolved.item_name, "contract_assets");
    }
}

#[test]
fn current_payable_aliases_preserve_reviewed_scope() {
    for local_name in CURRENT_PAYABLE_ALIASES {
        let expected_field = expected_current_payable_alias(local_name);
        assert_alias(local_name, expected_field);

        let resolved = canonicalize_semantic_concept(&jppfs(local_name)).unwrap();
        if expected_field != "accounts_payable_trade" {
            assert_ne!(resolved.item_name, "accounts_payable_trade");
        }
    }
}

#[test]
fn reviewed_receivable_payable_aliases_are_namespace_gated() {
    for local_name in [
        TRADE_RECEIVABLE_ALIASES[0],
        BROAD_CURRENT_RECEIVABLE_ALIASES[0],
        CURRENT_PAYABLE_ALIASES[0],
    ] {
        assert!(canonicalize_semantic_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
