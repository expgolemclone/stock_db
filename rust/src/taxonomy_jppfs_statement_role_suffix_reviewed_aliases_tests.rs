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

fn assert_pl_suffix_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed suffix alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(StatementCategory::ProfitAndLoss)
    );
    assert_eq!(statement_for_concept(&concept), Some("pl"));
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
fn consolidated_statement_role_suffix_aliases_resolve_to_pl_buckets() {
    for (local_name, expected_field) in [
        ("AAANOI", "non_operating_income"),
        ("CashBackIncomeNOI", "non_operating_income"),
        ("ContractBusinessRelatedCostsNOE", "non_operating_expenses"),
        ("CommissionForBorrowingNOE", "non_operating_expenses"),
        ("AnniversaryEL", "extraordinary_loss"),
        (
            "AOneTimeAllowanceOfWithdrawalFromEmployeesPensionFundEL",
            "extraordinary_loss",
        ),
        (
            "AdjustmentForTransferPricingTaxationEIEI",
            "extraordinary_income",
        ),
    ] {
        assert_pl_suffix_alias(local_name, expected_field);
    }
}

#[test]
fn reviewed_extraordinary_income_exceptions_are_preserved() {
    for local_name in [
        "AffiliatedCompanyConsolidationProfitEI",
        "AmortizationOfGoodwillEI",
        "AmortizationOfPriorServiceCostEI",
        "AnIntroductionFeeEI",
        "AntiDumpingDutiesEI",
        "AssetRetirementObligationEI",
        "BuildingDemolitionCostRestitutionProfitEI",
        "BurdenChargeForDevelopmentEI",
        "BusinessCompensationEI",
        "BusinessTaxRefundEI",
        "CashBeneficiaryEI",
        "ChangeProfitEI",
        "ChangesInEquityInterestEI",
        "CityGasCooperationFeeEI",
        "CollectionOfDamagesCompensationEI",
        "CollectionOfReceivableFromSubsidiariesAndAffiliatesEI",
        "CompensationExpensesEI",
        "CompensationForDamageCommisionEI",
        "CompensationForDamageEI",
        "CompensationForEvictionEI",
        "CompensationForHeadOfficeTransferEI",
        "CompensationForLossEI",
        "CompensationForMovingExpensesEI",
        "CompensationForRemovalEI",
        "CompensationForStoreClosingEI",
        "CompensationForTechnicalServicesEI",
        "CompensationForTheDamageEI",
        "CompensationForTransferEI",
        "CompensationForVacationEI",
        "CompensationOfRevenueEI",
        "ConsumptionTaxesDifferentialEI",
        "ConsumptionTaxesForPriorPeriodsEI",
        "ContractCancellationRevenueEI",
        "ContractTerminationRevenueEI",
        "ContributionForConstructionEI",
        "ContributionForConstructionRefundEI",
        "ContributionForDonationEI",
        "ContributionForRestorationEI",
        "ContributionFromRecapitalizationEI",
    ] {
        assert_pl_suffix_alias(local_name, "extraordinary_income");
    }
}

#[test]
fn suffix_fallback_does_not_overwrite_more_specific_reviewed_aliases() {
    for (local_name, expected_field) in [
        ("OtherNOI", "other_non_operating_income"),
        ("OtherNOE", "other_non_operating_expenses"),
        ("OtherEI", "other_extraordinary_income"),
        ("OtherEL", "other_extraordinary_loss"),
    ] {
        let resolved = canonicalize_semantic_concept(&jppfs(local_name))
            .unwrap_or_else(|| panic!("{local_name} should resolve via the base reviewed alias"));
        assert_eq!(resolved.item_name, expected_field);
    }
}

#[test]
fn unreviewed_extraordinary_income_suffix_requires_income_or_gain_wording() {
    let loss_like_ei = "AccidentRelatedLossEI";

    assert_ne!(
        canonicalize_semantic_concept(&jppfs(loss_like_ei))
            .as_ref()
            .map(|entry| entry.item_name.as_str()),
        Some("extraordinary_income"),
        "loss-like EI local names need explicit review instead of generic income folding"
    );
}

#[test]
fn statement_role_suffix_aliases_are_namespace_gated() {
    for local_name in [
        "AFFILIATEPaymentIncomeNOI",
        "AFuelSaleCostPriceNOE",
        "ALossOfCorrespondingToNewInfectionsEL",
        "AdjustmentForTransferPricingTaxationEIEI",
        "CashBeneficiaryEI",
    ] {
        assert!(canonicalize_semantic_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}
