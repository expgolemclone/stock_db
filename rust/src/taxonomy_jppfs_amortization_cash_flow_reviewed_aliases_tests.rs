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

const AMORTIZATION_EXPENSE_ALIASES: &[&str] = &[
    "AmortizationExpensesOfGoodwillSGA",
    "AmortizationOfCustomerRelatedAssetsSGA",
    "AmortizationOfCustomerRelatedIntangibleAssetsSGA",
    "AmortizationOfGoodwill",
    "AmortizationOfGoodwillSGA",
    "AmortizationOfLongTermPrepaidExpenses",
    "AmortizationOfLongTermPrepaidExpensesSGA",
    "AmortizationOfTrademarkRightSGA",
    "AmortizationOfGuaranteeCommission",
    "AmortizationOfGuaranteeDeposits",
];

const AMORTIZATION_CF_ADJUSTMENT_ALIASES: &[&str] = &[
    "AmortizationNetOfPremiumDiscountOnMarketableSecuritiesOpeCFIFRS",
    "AmortizationOfBondIssuanceCost2OpeCF",
    "AmortizationOfBondIssuanceCostOpeCF",
    "AmortizationOfBondIssuanceExpensesOpeCF",
    "AmortizationOfBusinessCommencementExpensesOpeCF",
    "AmortizationOfContentsOpeCF",
    "AmortizationOfDeferredAssetsUnderArticle113OfTheInsuranceBusinessLawOpeCFINS",
    "AmortizationOfDeferredInsuranceAcquisitionCostsOpeCFIFRS",
    "AmortizationOfDepositOpeCF",
    "AmortizationOfDepositsOpeCF",
    "AmortizationOfEquityInvestmentOpeCF",
    "AmortizationOfGoodwillAndAmortizationOfNegativeGoodwillOpeCF",
    "AmortizationOfGoodwillAndNegativeGoodwillOpeCF",
    "AmortizationOfGoodwillAndNegativegoodwillOpeCF",
    "AmortizationOfGoodwillELOpeCF",
    "AmortizationOfGoodwillExtraordinaryLossOpeCF",
    "AmortizationOfGoodwillNegativeGoodwillOpeCF",
    "AmortizationOfGoodwillOpeCF",
    "AmortizationOfGuaranteeDeposits1OpeCF",
    "AmortizationOfGuaranteeDepositsOpeCF",
    "AmortizationOfIdentifiableAssetsOpeCFIFRS",
    "AmortizationOfInvestmentCorporationBondIssuanceCostsOpeCFINV",
    "AmortizationOfInvestmentsInCapitalOpeCF",
    "AmortizationOfLeaseAndGuaranteeDepositsOpeCF",
    "AmortizationOfLeaseDepositsOpeCF",
    "AmortizationOfLeaseholdAndGuaranteeDepositsOpeCF",
    "AmortizationOfLeaseholdDepositsOpeCF",
    "AmortizationOfLeaseholdRightOpeCFOpeCF",
    "AmortizationOfLicenseContractDepositOpeCF",
    "AmortizationOfLongTermPrepaidExpensesOpeCF",
    "AmortizationOfLongtermPrepaidExpensesOpeCF",
    "AmortizationOfNegativeGoodwillOpeCF",
    "AmortizationOfNetRetirementBenefitObligationAtTransitionOpeCF",
    "AmortizationOfOpeningExpensesOpeCF",
    "AmortizationOfOrganizationExpensesOpeCFINV",
    "AmortizationOfPrepaidExpensesOpeCF",
    "AmortizationOfPriorServiceCostOpeCF",
    "AmortizationOfPriorServiceCostsOpeCF",
    "AmortizationOfRentalInventoryAssetsOpeCF",
    "AmortizationOfRestrictedStockRemunerationOpeCF",
    "AmortizationOfSecurityDepositAndSecurityDepositOpeCF",
    "AmortizationOfSecurityDepositOpeCF",
    "AmortizationOfSogtwareAndOthersOpeCF",
    "AmortizationOfStockIssuanceCostOpeCF",
    "AmortizationOfTademarkRightOpeCF",
    "AmortizationOfTechnologyAssetsOpeCF",
    "AmortizationOfTrademarkOpeCF",
    "AmortizationOfTrademarkRightOpeCF",
    "AmortizationOfTrademarksOpeCF",
    "AmortizationOfUnrecognizedItemsOpeCF",
];

const NON_OPERATING_AMORTIZATION_EXPENSE_ALIASES: &[&str] = &[
    "AmortizationNOE",
    "AmortizationOfActuarialGainForEmployeeSRetirementBenefitsNOE",
    "AmortizationOfBondIssuanceCostNOE",
    "AmortizationOfBondIssuanceCostsEtcNOE",
    "AmortizationOfBondIssuanceExpensesNOE",
    "AmortizationOfBusinessCommencementExpensesNOE",
    "AmortizationOfBusinessNOE",
    "AmortizationOfCopyrightNOE",
    "AmortizationOfDeferredAssetsNOE",
    "AmortizationOfDeferredAssetsUnderArticle113OfInsuranceBusinessLawNOE",
    "AmortizationOfDeferredOrganizationExpensesNOE",
    "AmortizationOfDevelopmentExpensesNOE",
    "AmortizationOfEquityInvestmentNOE",
    "AmortizationOfGoodwillNOE",
    "AmortizationOfInvestmentCorporationBondIssuanceCostsNOEINV",
    "AmortizationOfIssuanceCostOfSubscriptionRightsToSharesNOE",
];

const EXTRAORDINARY_AMORTIZATION_LOSS_ALIASES: &[&str] = &[
    "AmortisationOfGoodwillEL",
    "AmortizatinOfGoodwillEL",
    "AmortizationGoodwillEL",
    "AmortizationOfBusinessCommencementExpensesEL",
    "AmortizationOfDeferredAssetsEL",
    "AmortizationOfDepositsEL",
    "AmortizationOfGoodWillEL",
    "AmortizationOfGoodwill1EL",
    "AmortizationOfGoodwillAndNegativeGoodwillEL",
    "AmortizationOfGoodwillEL",
    "AmortizationOfGoodwillExtraordinaryLoss",
    "AmortizationOfGoodwillExtraordinaryLossEL",
    "AmortizationOfGoodwillExtraordinaryLosses",
    "AmortizationOfKeyMoneyEL",
    "AmortizationOfLongTermPrepaidExpensesEL",
    "AmortizationOfNegativeGoodwillEL",
    "AmortizationOfPastServiceCostEL",
    "AmortizationOfPastServiceCostsEL",
    "AmortizationOfPrepaidExpensesEL",
    "AmortizationOfPriorServiceCostEL",
    "AmortizationOfPriorServiceCostsEL",
    "AmortizationOrGoodwillEL",
    "AmortrizationOfGoodwillEL",
    "AmotizationOfGoodwillEL",
];

fn assert_reviewed_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
    expected_statement: &str,
) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(resolved.item_name, expected_field);
    assert_eq!(
        classify_item_name(&resolved.item_name),
        Some(expected_category)
    );
    assert_eq!(statement_for_concept(&concept), Some(expected_statement));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(_)
    ));
}

#[test]
fn reviewed_amortization_aliases_classify_one_hundred_historical_tags() {
    assert_eq!(
        AMORTIZATION_EXPENSE_ALIASES.len()
            + AMORTIZATION_CF_ADJUSTMENT_ALIASES.len()
            + NON_OPERATING_AMORTIZATION_EXPENSE_ALIASES.len()
            + EXTRAORDINARY_AMORTIZATION_LOSS_ALIASES.len(),
        100
    );

    for local_name in AMORTIZATION_EXPENSE_ALIASES {
        let expected_field = match *local_name {
            "AmortizationExpensesOfGoodwillSGA" | "AmortizationOfGoodwillSGA" => {
                "amortization_of_goodwill_selling_general_admin"
            }
            _ => "amortization",
        };
        assert_reviewed_alias(
            local_name,
            expected_field,
            StatementCategory::ProfitAndLoss,
            "pl",
        );
        assert_ne!(
            canonicalize_semantic_concept(&jppfs(local_name))
                .unwrap()
                .item_name,
            "amortization_cf_adjustment"
        );
    }

    for local_name in AMORTIZATION_CF_ADJUSTMENT_ALIASES {
        assert_reviewed_alias(
            local_name,
            "amortization_cf_adjustment",
            StatementCategory::CashFlow,
            "cf",
        );
        assert_ne!(
            canonicalize_semantic_concept(&jppfs(local_name))
                .unwrap()
                .item_name,
            "amortization"
        );
    }

    for local_name in NON_OPERATING_AMORTIZATION_EXPENSE_ALIASES {
        assert_reviewed_alias(
            local_name,
            "non_operating_expenses",
            StatementCategory::ProfitAndLoss,
            "pl",
        );
        assert_ne!(
            canonicalize_semantic_concept(&jppfs(local_name))
                .unwrap()
                .item_name,
            "amortization"
        );
    }

    for local_name in EXTRAORDINARY_AMORTIZATION_LOSS_ALIASES {
        assert_reviewed_alias(
            local_name,
            "extraordinary_loss",
            StatementCategory::ProfitAndLoss,
            "pl",
        );
        assert_ne!(
            canonicalize_semantic_concept(&jppfs(local_name))
                .unwrap()
                .item_name,
            "impairment_loss"
        );
    }
}
