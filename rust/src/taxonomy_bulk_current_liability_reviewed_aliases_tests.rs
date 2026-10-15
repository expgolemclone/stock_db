use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, is_excluded_concept, statement_for_concept,
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

const CURRENT_PAYABLE_ALIASES: &[&str] = &[
    "AccountsPayable",
    "AccountsPayableAffiliatedCompanies",
    "AccountsPayableConsignment",
    "AccountsPayableDevelopmentBusinessAndOther",
    "AccountsPayableDueToMemberMerchants",
    "AccountsPayableFacilities",
    "AccountsPayableFacility",
    "AccountsPayableForEquipmentLiabilities",
    "AccountsPayableForFacilities",
    "AccountsPayableForFranchisee",
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
    "AgencySecurityCL",
    "AdvancesNotesPayableFacilitiesInSubsidiariesAndAffiliatesCL",
    "AdvancesObligationsFromSubsidiariesAndAffiliatesCL",
];

const ADVANCES_RECEIVED_ALIASES: &[&str] = &[
    "AdvanceDepositsReceivedCL",
    "AdvanceReceivedInterestInstalmentCL",
    "AdvanceReceivedSalesOfMaintenanceServiceCL",
    "AdvancesFromContractsInProgress",
    "AdvancesReceived",
    "AdvancesReceivedOnConstructionConstractsInprogress",
    "AdvancesReceivedOnConstructionContractsInProgress",
    "AdvancesReceivedOnDevelopmentProjectsCNS",
    "AdvancesReceivedOnUmcompletedConstructionContractsAndOtherCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsAndOtherCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsAndOthersCNS",
    "AdvancesReceivedOnUncompletedConstructionContractsCNS",
    "AdvancesReceivedOnUncompletedContracts",
    "AdvancesReceivedRental",
];

const CURRENT_PROVISION_ALIASES: &[&str] = &[
    "AllowanceForAfterCareOfProductsCL",
    "AllowanceForBusinessRestructuringLossesCL",
    "AllowanceForCancellationOfContractCL",
    "AllowanceForCapitalLossOfRelatedCompaniesCL",
    "AllowanceForCoinUsageCL",
    "AllowanceForCompanyBuildingReplacementLossCL",
    "AllowanceForConstructionLossCL",
    "AllowanceForConstructionRepairCL",
    "AllowanceForDemolitionCostsCL",
    "AllowanceForDirectorsRetirementBenefitsCL",
    "AllowanceForDirectorsRetirementCL",
    "AllowanceForEarlyRetirementProgramCL",
    "AllowanceForEarlySubscriptionCancellationsCurrentLiabilitiesLiabilities",
    "AllowanceForEnvironmentalMeasuresCL",
    "AllowanceForEnvironmentalRemediationExpensesCL",
    "AllowanceForExpenseRelatedToRetrospectiveAdjustmentCL",
    "AllowanceForForeignIndirectTaxCL",
    "AllowanceForInvestmentLossCL",
    "AllowanceForInvestmentLossOfSubsidiariesAndAffiliatesCL",
    "AllowanceForLiquidationLossOfAffiliatedCompaniesCL",
    "AllowanceForLossOnCollectingGiftTicketsCurrentLiabilities",
    "AllowanceForLossOnDisposalOfReturnedGoodsCL",
    "AllowanceForLossOnSaleOfRelatedCompanySharesCL",
    "AllowanceForLossOnShortTermCancellationCL",
    "AllowanceForLossOnTransferOfSharesOfAffiliatedCompaniesCL",
    "AllowanceForLossOnVoluntaryRecallOfProductsCL",
    "AllowanceForLossRelatedToAntiMonopolyActCL",
    "AllowanceForLossesFromBusinessTransferCL",
    "AllowanceForLossesOnBusinessCL",
    "AllowanceForLossesOnBusinessTransferCL",
    "AllowanceForLossesOnCollecctingGiftTicketsCL",
    "AllowanceForLossesOnEmployeePensionFundWithdrawalCL",
    "AllowanceForLossesRelatedToTrustTypeStockOptionsCL",
    "AllowanceForOfficeTransferExpensesCL",
    "AllowanceForProductReturnedCL",
    "AllowanceForProductWarrantyCL",
    "AllowanceForProgramProductWarrantyCL",
    "AllowanceForQualityRelatedLossCL",
    "AllowanceForRecallCL",
    "AllowanceForRefundCL",
    "AllowanceForRelocationCostCL",
    "AllowanceForRepairProductCostsCL",
    "AllowanceForRepaymentCL",
    "AllowanceForResourceRecyclingExpensesCL",
    "AllowanceForRestructuringCL",
    "AllowanceForReturnsCL",
    "AllowanceForSalesDiscountCL",
    "AllowanceForShareholdersSpecialBenefitsCL",
    "AllowanceForShortTermCancellationLossCL",
    "AllowanceForSpecialInvestigationCL",
    "AllowanceForStockBasedCompensationCL",
    "AllowanceForSubscriptionCancellationsCL",
    "AllowanceForTransferOfSharesOfSubsidiariesAndAffiliatesCL",
    "AllowanceForWithdrawalOfBusinessCL",
    "AllowanceOfficerRetirementSpecialServiceMoneyCL",
    "AllowancesForLossOnPurchaseAgreementsCL",
    "AllowancesForSalesReturnsCL",
    "AloowanceForInvestmentLossCL",
    "AmendsForProductWarrantiesCL",
];

fn assert_alias(local_name: &str, expected: &str) {
    let concept = jppfs(local_name);
    let resolved =
        canonicalize_concept(&concept).unwrap_or_else(|| panic!("{local_name} should resolve"));
    assert_eq!(resolved.item_name, expected);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(statement_for_concept(&concept), Some("bs"));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
    assert!(!is_excluded_concept(local_name));
}

fn expected_current_payable_alias(local_name: &str) -> &'static str {
    match local_name {
        "AccountsPayableFacilities"
        | "AccountsPayableFacility"
        | "AccountsPayableForEquipmentLiabilities"
        | "AccountsPayableForFacilities" => {
            "accounts_payable_for_property_plant_and_equipment_current"
        }
        "AccountsPayableOtherAndAccruedExpenses" => "accounts_payable_other_and_accrued_expenses",
        "AccountsPayableRealEstate" => "accounts_payable_real_estate_current",
        "AccountsPayableSurveyingSpecific" => "accounts_payable_surveying_current",
        _ => "accounts_payable_current",
    }
}

fn expected_advances_received_alias(local_name: &str) -> &'static str {
    match local_name {
        "AdvancesReceivedOnUncompletedConstructionContractsCNS" => {
            "advances_received_on_uncompleted_construction_contracts"
        }
        _ => "advances_received_current",
    }
}

#[test]
fn reviewed_bulk_current_liability_aliases_cover_exactly_98_unambiguous_tags() {
    assert_eq!(
        CURRENT_PAYABLE_ALIASES.len()
            + ADVANCES_RECEIVED_ALIASES.len()
            + CURRENT_PROVISION_ALIASES.len(),
        98
    );
}

#[test]
fn additional_payable_aliases_preserve_current_liability_scope() {
    for local_name in CURRENT_PAYABLE_ALIASES {
        assert_alias(local_name, expected_current_payable_alias(local_name));
        let item = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item, "accounts_payable_trade");
        assert_ne!(item, "current_provisions");
    }
}

#[test]
fn additional_advances_received_aliases_preserve_contract_liability_scope() {
    for local_name in ADVANCES_RECEIVED_ALIASES {
        assert_alias(local_name, expected_advances_received_alias(local_name));
        let item = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item, "accounts_payable_current");
        assert_ne!(item, "trade_and_other_current_receivables");
    }
}

#[test]
fn current_provision_aliases_fold_to_current_provisions_not_expense_or_payable() {
    for local_name in CURRENT_PROVISION_ALIASES {
        assert_alias(local_name, "current_provisions");
        let item = canonicalize_concept(&jppfs(local_name)).unwrap().item_name;
        assert_ne!(item, "accounts_payable_current");
        assert_ne!(item, "accrued_expenses_current");
        assert_ne!(item, "extraordinary_loss");
    }
}

#[test]
fn non_current_cash_flow_asset_and_loss_allowance_tags_are_not_current_provisions() {
    for local_name in [
        "AllowanceForDoubtfulAccountsCA",
        "AllowanceForProductWarrantyIncreaseDecreaseOpeCF",
        "AllowanceForEnvironmentalRemediationExpensesEL",
        "AllowanceForInvestmentLossEL",
        "AllowanceForProductLiabilitiesNCL",
    ] {
        let resolved = canonicalize_concept(&jppfs(local_name));
        assert!(
            !matches!(resolved, Some(canonical) if canonical.item_name == "current_provisions")
        );
    }
}

#[test]
fn reviewed_bulk_aliases_are_namespace_gated() {
    for local_name in [
        CURRENT_PAYABLE_ALIASES[0],
        ADVANCES_RECEIVED_ALIASES[0],
        CURRENT_PROVISION_ALIASES[0],
    ] {
        assert!(canonicalize_semantic_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
