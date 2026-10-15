use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn extension(local_name: &str) -> (String, String) {
    (
        "https://issuer.example.jp/taxonomy/2026/custom".to_string(),
        local_name.to_string(),
    )
}

fn assert_semantic_alias(
    local_name: &str,
    expected_field: &str,
    expected_category: StatementCategory,
    expected_statement: &str,
) {
    let concept = jppfs(local_name);
    let resolved = canonicalize_semantic_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed semantic alias"));

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
fn high_frequency_balance_sheet_aliases_preserve_accounting_scope() {
    use StatementCategory::BalanceSheet;

    for (local_name, expected_field) in [
        ("OtherCA", "other_current_assets"),
        ("OtherIOA", "other_investments_and_other_assets"),
        ("DeferredTaxAssets", "deferred_tax_assets"),
        ("DeferredTaxAssetsCA", "deferred_tax_assets_current"),
        ("DeferredTaxAssetsIOA", "deferred_tax_assets_non_current"),
        ("ConstructionInProgress", "construction_in_progress"),
        ("OtherIA", "other_intangible_assets"),
        (
            "ToolsFurnitureAndFixturesNet",
            "tools_furniture_and_fixtures_net",
        ),
        ("Software", "software"),
        ("WorkInProcess", "work_in_process"),
        ("Goodwill", "goodwill"),
        ("RawMaterialsAndSupplies", "raw_materials_and_supplies"),
        (
            "MerchandiseAndFinishedGoods",
            "merchandise_and_finished_goods",
        ),
        ("LeaseAssetsNetPPE", "lease_assets_net_ppe"),
        ("BuildingsNet", "buildings_net"),
        ("PrepaidExpenses", "prepaid_expenses_current"),
        (
            "StocksOfSubsidiariesAndAffiliates",
            "stocks_of_subsidiaries_and_affiliates",
        ),
        ("LongTermLoansReceivable", "long_term_loans_receivable"),
        ("ToolsFurnitureAndFixtures", "tools_furniture_and_fixtures"),
        ("NotesReceivableTrade", "notes_receivable_trade"),
        (
            "ElectronicallyRecordedMonetaryClaimsOperatingCA",
            "electronically_recorded_monetary_claims_operating_current",
        ),
        ("LongTermPrepaidExpenses", "long_term_prepaid_expenses"),
        ("BuildingsAndStructuresNet", "buildings_and_structures_net"),
        ("OtherNetPPE", "other_ppe_net"),
        ("VehiclesNet", "vehicles_net"),
        ("StructuresNet", "structures_net"),
        (
            "ShortTermInvestmentSecurities",
            "short_term_investment_securities",
        ),
        ("LeaseAssetsPPE", "lease_assets_ppe"),
        ("MachineryAndEquipmentNet", "machinery_and_equipment_net"),
        (
            "MachineryEquipmentAndVehiclesNet",
            "machinery_equipment_and_vehicles_net",
        ),
        ("BuildingsAndStructures", "buildings_and_structures"),
        ("LeaseAndGuaranteeDeposits", "lease_and_guarantee_deposits"),
        ("Merchandise", "merchandise"),
        ("Supplies", "supplies"),
        (
            "GuaranteeDepositsIOA",
            "guarantee_deposits_non_current_assets",
        ),
        ("ContractLiabilities", "contract_liabilities"),
        ("Buildings", "buildings"),
        ("SoftwareInProgress", "software_in_progress"),
        ("OtherPPE", "other_ppe"),
        ("InvestmentsInCapital", "investments_in_capital"),
        (
            "LongTermLoansReceivableFromSubsidiariesAndAffiliates",
            "long_term_loans_receivable_subsidiaries_affiliates",
        ),
        (
            "MachineryEquipmentAndVehicles",
            "machinery_equipment_and_vehicles",
        ),
        (
            "ClaimsProvableInBankruptcyClaimsProvableInRehabilitationAndOther",
            "claims_provable_bankruptcy_rehabilitation_other",
        ),
        ("AdvancePaymentsTrade", "advance_payments_trade"),
        (
            "InvestmentsInCapitalOfSubsidiariesAndAffiliates",
            "investments_in_capital_subsidiaries_affiliates",
        ),
        ("LeaseAssetsIA", "lease_assets_intangible_assets"),
        ("ShortTermLoansReceivable", "short_term_loans_receivable"),
        ("TelephoneSubscriptionRight", "telephone_subscription_right"),
        ("NetDefinedBenefitAsset", "net_defined_benefit_asset"),
        ("Vehicles", "vehicles"),
        ("LeaseholdRight", "leasehold_right"),
        ("LongTermGuaranteeDeposited", "long_term_guarantee_deposits"),
        (
            "ShortTermLoansReceivableToSubsidiariesAndAffiliates",
            "short_term_loans_receivable_subsidiaries_affiliates",
        ),
        ("Structures", "structures"),
        ("MachineryAndEquipment", "machinery_and_equipment"),
        (
            "PrepaidPensionCostIOA",
            "prepaid_pension_cost_non_current_assets",
        ),
        ("DeferredAssets", "deferred_assets"),
        ("IncomeTaxesReceivable", "income_taxes_receivable"),
        ("FinishedGoods", "finished_goods"),
        ("RightOfTrademark", "trademark_right"),
        ("Land", "land"),
        ("IncomeTaxesPayable", "income_taxes_payable"),
        ("OtherCL", "other_current_liabilities"),
        (
            "AccruedConsumptionTaxes",
            "accrued_consumption_taxes_current",
        ),
        ("OtherNCL", "other_non_current_liabilities"),
        ("ProvisionForBonuses", "provision_for_bonuses"),
        ("LeaseObligationsNCL", "lease_obligations_non_current"),
        (
            "AssetRetirementObligationsNCL",
            "asset_retirement_obligations_non_current",
        ),
        ("LeaseObligationsCL", "lease_obligations_current"),
        ("DepositsReceived", "deposits_received"),
        ("DeferredTaxLiabilities", "deferred_tax_liabilities"),
        (
            "DeferredTaxLiabilitiesNCL",
            "deferred_tax_liabilities_non_current",
        ),
        ("AdvancesReceived", "advances_received_current"),
        (
            "ElectronicallyRecordedObligationsOperatingCL",
            "electronically_recorded_obligations_operating_current",
        ),
        (
            "NetDefinedBenefitLiability",
            "net_defined_benefit_liability",
        ),
        ("CurrentPortionOfBonds", "current_portion_bonds_payable"),
        (
            "ProvisionForRetirementBenefits",
            "provision_for_retirement_benefits",
        ),
        (
            "LongTermAccountsPayableOther",
            "long_term_accounts_payable_other",
        ),
        (
            "DeferredTaxLiabilitiesForLandRevaluation",
            "deferred_tax_liabilities_land_revaluation",
        ),
        ("NotesPayableTrade", "notes_payable_trade"),
        (
            "ProvisionForProductWarranties",
            "provision_for_product_warranties",
        ),
        (
            "AssetRetirementObligationsCL",
            "asset_retirement_obligations_current",
        ),
        ("UnearnedRevenue", "unearned_revenue"),
        (
            "ProvisionForLossOnConstructionContracts",
            "provision_for_loss_on_construction_contracts",
        ),
        (
            "ProvisionForWarrantiesForCompletedConstruction",
            "provision_for_warranties_for_completed_construction",
        ),
        (
            "ProvisionForLossOnOrderReceivedCL",
            "provision_for_loss_on_order_received_current",
        ),
        (
            "ProvisionForPointCardCertificatesCL",
            "provision_for_point_card_certificates_current",
        ),
        (
            "AdvancesReceivedOnUncompletedConstructionContractsCNS",
            "advances_received_on_uncompleted_construction_contracts",
        ),
        ("TreasuryStock", "treasury_stock"),
        (
            "SubscriptionRightsToShares",
            "subscription_rights_to_shares",
        ),
        (
            "RetainedEarningsBroughtForward",
            "retained_earnings_brought_forward",
        ),
        (
            "DeferredGainsOrLossesOnHedges",
            "deferred_gains_or_losses_on_hedges_equity",
        ),
        ("LegalRetainedEarnings", "legal_retained_earnings"),
        ("OtherCapitalSurplus", "other_capital_surplus"),
        ("GeneralReserve", "general_reserve"),
        ("NonControllingInterests", "noncontrolling_interests"),
        (
            "ValuationAndTranslationAdjustments",
            "valuation_and_translation_adjustments",
        ),
        (
            "ValuationDifferenceOnAvailableForSaleSecurities",
            "valuation_difference_on_available_for_sale_securities_equity",
        ),
        (
            "ForeignCurrencyTranslationAdjustment",
            "foreign_currency_translation_adjustment_equity",
        ),
        (
            "RemeasurementsOfDefinedBenefitPlans",
            "remeasurements_of_defined_benefit_plans_equity",
        ),
        ("RevaluationReserveForLand", "revaluation_reserve_for_land"),
        (
            "ReserveForAdvancedDepreciationOfNoncurrentAssets",
            "reserve_for_advanced_depreciation_of_noncurrent_assets",
        ),
        (
            "AllowanceForDoubtfulAccountsCA",
            "allowance_for_doubtful_accounts_current_assets",
        ),
        (
            "AllowanceForDoubtfulAccountsIOAByGroup",
            "allowance_for_doubtful_accounts_investments_and_other_assets",
        ),
        (
            "AccumulatedDepreciationToolsFurnitureAndFixtures",
            "accumulated_depreciation",
        ),
        (
            "AccumulatedDepreciationPPEByGroup",
            "accumulated_depreciation",
        ),
        (
            "AccumulatedDepreciationOtherPPE",
            "accumulated_depreciation",
        ),
        (
            "AccumulatedDepreciationStructures",
            "accumulated_depreciation",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, BalanceSheet, "bs");
    }

    assert_ne!(
        canonicalize_concept(&jppfs("OtherCA")).map(|entry| entry.item_name),
        canonicalize_concept(&jppfs("OtherCL")).map(|entry| entry.item_name),
        "current assets and current liabilities must not be collapsed"
    );
    assert_ne!(
        canonicalize_concept(&jppfs("TreasuryStock")).map(|entry| entry.item_name),
        canonicalize_concept(&jppfs("PurchaseOfTreasuryStockFinCF")).map(|entry| entry.item_name),
        "treasury-stock balance and treasury-stock purchase cash flow are different accounts"
    );
}

#[test]
fn high_frequency_profit_loss_aliases_preserve_line_item_scope() {
    use StatementCategory::ProfitAndLoss;

    for (local_name, expected_field) in [
        ("InterestExpensesNOE", "interest_expenses"),
        ("InterestIncomeNOI", "interest_income"),
        ("ImpairmentLossEL", "impairment_loss"),
        ("DividendsIncomeNOI", "dividend_income"),
        ("ForeignExchangeGainsNOI", "foreign_exchange_gains"),
        ("ForeignExchangeLossesNOE", "foreign_exchange_losses"),
        (
            "GainOnSalesOfNoncurrentAssetsEI",
            "gain_on_sales_of_noncurrent_assets",
        ),
        (
            "GainOnSalesOfInvestmentSecuritiesEI",
            "gain_on_sales_of_investment_securities",
        ),
        (
            "LossOnRetirementOfNoncurrentAssetsEL",
            "loss_on_retirement_of_noncurrent_assets",
        ),
        (
            "LossOnValuationOfInvestmentSecuritiesEL",
            "loss_on_valuation_of_investment_securities",
        ),
        (
            "LossOnSalesOfNoncurrentAssetsEL",
            "loss_on_sales_of_noncurrent_assets",
        ),
        ("CommissionFeeNOE", "commission_fee_non_operating_expenses"),
        ("DepreciationSGA", "depreciation_selling_general_admin"),
        (
            "RetirementBenefitExpensesSGA",
            "retirement_benefit_expenses_selling_general_admin",
        ),
        (
            "ProvisionForBonusesSGA",
            "provision_for_bonuses_selling_general_admin",
        ),
        ("RentIncomeNOI", "rent_income"),
        ("MiscellaneousIncomeNOI", "miscellaneous_income"),
        ("SubsidyIncomeNOIBounty", "subsidy_income_non_operating"),
        ("SubsidyIncomeNOI", "subsidy_income_non_operating"),
        (
            "LossOnValuationOfStocksOfSubsidiariesAndAffiliatesEL",
            "loss_on_valuation_of_stocks_of_subsidiaries_and_affiliates",
        ),
        (
            "InterestAndDividendsIncomeNOI",
            "interest_and_dividends_income",
        ),
        (
            "LossOnSalesOfInvestmentSecuritiesEL",
            "loss_on_sales_of_investment_securities",
        ),
        (
            "ProvisionOfAllowanceForDoubtfulAccountsSGA",
            "provision_of_allowance_for_doubtful_accounts_sga",
        ),
        (
            "ProvisionOfAllowanceForDoubtfulAccountsNOE",
            "provision_of_allowance_for_doubtful_accounts_non_operating",
        ),
        (
            "LossOnDisposalOfNoncurrentAssetsEL",
            "loss_on_disposal_of_noncurrent_assets",
        ),
        (
            "GainOnReversalOfSubscriptionRightsToSharesEI",
            "gain_on_reversal_of_subscription_rights_to_shares",
        ),
        (
            "DirectorsCompensationsSGA",
            "directors_compensation_selling_general_admin",
        ),
        ("MiscellaneousLossNOE", "miscellaneous_loss_non_operating"),
        (
            "SalariesAndAllowancesSGA",
            "salaries_and_allowances_selling_general_admin",
        ),
        (
            "LossOnSalesAndRetirementOfNoncurrentAssetsEL",
            "loss_on_sales_and_retirement_of_noncurrent_assets",
        ),
        ("InsuranceIncomeNOI", "insurance_income_non_operating"),
        (
            "EquityInEarningsOfAffiliatesNOI",
            "equity_in_earnings_of_affiliates",
        ),
        (
            "AmortizationExpensesOfGoodwillSGA",
            "amortization_of_goodwill_selling_general_admin",
        ),
        (
            "AmortizationOfGoodwillSGA",
            "amortization_of_goodwill_selling_general_admin",
        ),
        (
            "ReversalOfAllowanceForDoubtfulAccountsNOI",
            "reversal_of_allowance_for_doubtful_accounts",
        ),
        (
            "GainOnSalesOfSubsidiariesAndAffiliatesStocksEI",
            "gain_on_sales_of_subsidiaries_and_affiliates_stocks",
        ),
        (
            "AdvertisingExpensesSGA",
            "advertising_expenses_selling_general_admin",
        ),
        ("SubsidyEI", "subsidy_income_extraordinary"),
        ("InterestExpensesOEBNK", "interest_expenses_bank_operating"),
        ("InterestIncomeOIBNK", "interest_income_bank_operating"),
        ("StockIssuanceCostNOE", "stock_issuance_cost_non_operating"),
        ("RentExpensesSGA", "rent_expenses_selling_general_admin"),
        ("InsuranceIncomeEI", "insurance_income_extraordinary"),
        ("LossOnDisasterEL", "loss_on_disaster_extraordinary"),
        (
            "WelfareExpensesSGA",
            "welfare_expenses_selling_general_admin",
        ),
        ("SalesDiscountsNOE", "sales_discounts_non_operating"),
        (
            "ProvisionForDirectorsBonusesSGA",
            "provision_for_directors_bonuses_selling_general_admin",
        ),
        (
            "LossOnRetirementOfNoncurrentAssetsNOE",
            "loss_on_retirement_of_noncurrent_assets_non_operating",
        ),
        (
            "LossOnInvestmentsInPartnershipNOE",
            "loss_on_investments_in_partnership_non_operating",
        ),
        (
            "ProvisionForDirectorsRetirementBenefitsSGA",
            "provision_for_directors_retirement_benefits_selling_general_admin",
        ),
        ("InterestOnBondsNOE", "interest_expenses"),
        ("RealEstateRentNOI", "real_estate_rent_income"),
        (
            "ResearchAndDevelopmentExpensesSGA",
            "research_and_development",
        ),
        ("TaxesAndDuesSGA", "taxes_and_dues_selling_general_admin"),
        ("PurchaseDiscountsNOI", "purchase_discounts_non_operating"),
        (
            "LossOnReductionOfNoncurrentAssetsEL",
            "loss_on_reduction_of_noncurrent_assets_extraordinary",
        ),
        ("CommissionFeeSGA", "commission_fee_selling_general_admin"),
        (
            "EquityInLossesOfAffiliatesNOE",
            "equity_in_losses_of_affiliates",
        ),
        (
            "ProvisionOfAllowanceForDoubtfulAccountsEL",
            "provision_of_allowance_for_doubtful_accounts_extraordinary",
        ),
        (
            "LegalWelfareExpensesSGA",
            "legal_welfare_expenses_selling_general_admin",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, ProfitAndLoss, "pl");
    }
}

#[test]
fn high_frequency_cash_flow_aliases_keep_cash_flow_context() {
    use StatementCategory::CashFlow;

    for (local_name, expected_field) in [
        (
            "NetIncreaseDecreaseInCashAndCashEquivalents",
            "net_increase_decrease_in_cash_and_cash_equivalents",
        ),
        ("SubtotalOpeCF", "subtotal_operating_cf"),
        ("OtherNetOpeCF", "other_net_operating_cf"),
        (
            "DecreaseIncreaseInNotesAndAccountsReceivableTradeOpeCF",
            "decrease_increase_in_trade_receivables_operating_cf",
        ),
        (
            "IncreaseDecreaseInNotesAndAccountsPayableTradeOpeCF",
            "increase_decrease_in_trade_payables_operating_cf",
        ),
        (
            "DecreaseIncreaseInInventoriesOpeCF",
            "decrease_increase_in_inventories_operating_cf",
        ),
        ("IncomeTaxesPaidOpeCF", "income_taxes_paid_operating_cf"),
        (
            "InterestAndDividendsIncomeOpeCF",
            "interest_and_dividends_income_operating_cf",
        ),
        (
            "InterestAndDividendsIncomeReceivedOpeCFInvCF",
            "interest_and_dividends_received_operating_investing_cf",
        ),
        (
            "InterestExpensesPaidOpeCFFinCF",
            "interest_expenses_paid_operating_financing_cf",
        ),
        (
            "IncreaseDecreaseInAllowanceForDoubtfulAccountsOpeCF",
            "increase_decrease_in_allowance_for_doubtful_accounts_operating_cf",
        ),
        ("OtherNetInvCF", "other_net_investing_cf"),
        (
            "EffectOfExchangeRateChangeOnCashAndCashEquivalents",
            "effect_of_exchange_rate_change_on_cash_and_cash_equivalents",
        ),
        (
            "IncreaseDecreaseInProvisionForBonusesOpeCF",
            "increase_decrease_in_provision_for_bonuses_operating_cf",
        ),
        (
            "IncreaseDecreaseInNetDefinedBenefitLiabilityOpeCF",
            "increase_decrease_in_net_defined_benefit_liability_operating_cf",
        ),
        (
            "NetIncreaseDecreaseInShortTermLoansPayableFinCF",
            "net_increase_decrease_in_short_term_loans_payable_financing_cf",
        ),
        (
            "LossGainOnSalesOfInvestmentSecuritiesOpeCF",
            "loss_gain_on_sales_of_investment_securities_operating_cf",
        ),
        (
            "RepaymentsOfLeaseObligationsFinCF",
            "repayments_of_lease_obligations_financing_cf",
        ),
        (
            "IncreaseDecreaseInAccruedConsumptionTaxesOpeCF",
            "increase_decrease_in_accrued_consumption_taxes_operating_cf",
        ),
        ("OtherNetFinCF", "other_net_financing_cf"),
        (
            "AmortizationOfGoodwillOpeCF",
            "amortization_of_goodwill_operating_cf",
        ),
        (
            "PaymentsIntoTimeDepositsInvCF",
            "payments_into_time_deposits_investing_cf",
        ),
        (
            "ProceedsFromWithdrawalOfTimeDepositsInvCF",
            "proceeds_from_withdrawal_of_time_deposits_investing_cf",
        ),
        (
            "LossOnRetirementOfNoncurrentAssetsOpeCF",
            "loss_on_retirement_of_noncurrent_assets_operating_cf",
        ),
        (
            "ForeignExchangeLossesGainsOpeCF",
            "foreign_exchange_losses_gains_operating_cf",
        ),
        (
            "EquityInEarningsLossesOfAffiliatesOpeCF",
            "equity_in_earnings_losses_of_affiliates_operating_cf",
        ),
        (
            "CollectionOfLoansReceivableInvCF",
            "collection_of_loans_receivable_investing_cf",
        ),
        (
            "IncreaseDecreaseInAccountsPayableOtherOpeCF",
            "increase_decrease_in_other_accounts_payable_operating_cf",
        ),
        (
            "LossGainOnValuationOfInvestmentSecuritiesOpeCF",
            "loss_gain_on_valuation_of_investment_securities_operating_cf",
        ),
        (
            "DividendsPaidToNonControllingInterestsFinCF",
            "dividends_paid_to_noncontrolling_interests_financing_cf",
        ),
        (
            "IncreaseDecreaseInProvisionForDirectorsRetirementBenefitsOpeCF",
            "increase_decrease_in_provision_for_directors_retirement_benefits_operating_cf",
        ),
        (
            "LossGainOnSalesOfNoncurrentAssetsOpeCF",
            "loss_gain_on_sales_of_noncurrent_assets_operating_cf",
        ),
        (
            "PaymentsOfLoansReceivableInvCF",
            "payments_of_loans_receivable_investing_cf",
        ),
        ("IncomeTaxesRefundOpeCF", "income_taxes_refund_operating_cf"),
        (
            "IncreaseDecreaseInProvisionForDirectorsBonusesOpeCF",
            "increase_decrease_in_provision_for_directors_bonuses_operating_cf",
        ),
        ("RedemptionOfBondsFinCF", "redemption_of_bonds_financing_cf"),
        (
            "PurchaseOfInvestmentsInSubsidiariesResultingInChangeInScopeOfConsolidationInvCF",
            "purchase_of_investments_in_subsidiaries_resulting_in_change_in_scope_of_consolidation_investing_cf",
        ),
        (
            "PaymentsForLeaseAndGuaranteeDepositsInvCF",
            "payments_for_lease_and_guarantee_deposits_investing_cf",
        ),
        (
            "ProceedsFromCollectionOfLeaseAndGuaranteeDepositsInvCF",
            "proceeds_from_collection_of_lease_and_guarantee_deposits_investing_cf",
        ),
        (
            "IncreaseDecreaseInAdvancesReceivedOpeCF",
            "increase_decrease_in_advances_received_operating_cf",
        ),
        (
            "ProceedsFromIssuanceOfCommonStockFinCF",
            "proceeds_from_issuance_of_common_stock_financing_cf",
        ),
        (
            "IncreaseDecreaseInAccruedExpensesOpeCF",
            "increase_decrease_in_accrued_expenses_operating_cf",
        ),
        (
            "IncreaseDecreaseInNetDefinedBenefitAssetOpeCF",
            "increase_decrease_in_net_defined_benefit_asset_operating_cf",
        ),
        (
            "IncomeTaxesPaidRefundOpeCF",
            "income_taxes_paid_refund_operating_cf",
        ),
        (
            "PaymentsFromChangesInOwnershipInterestsInSubsidiariesThatDoNotResultInChangeInScopeOfConsolidationFinCF",
            "payments_from_changes_in_ownership_interests_in_subsidiaries_without_scope_change_financing_cf",
        ),
        (
            "PaymentsForGuaranteeDepositsInvCF",
            "payments_for_guarantee_deposits_investing_cf",
        ),
        (
            "DecreaseInShortTermLoansPayableFinCF",
            "decrease_in_short_term_loans_payable_financing_cf",
        ),
        (
            "IncreaseDecreaseInOtherCurrentLiabilitiesOpeCF",
            "increase_decrease_in_other_current_liabilities_operating_cf",
        ),
        (
            "DecreaseIncreaseInOtherCurrentAssetsOpeCF",
            "decrease_increase_in_other_current_assets_operating_cf",
        ),
        (
            "LossGainOnSalesOfPropertyPlantAndEquipmentOpeCF",
            "loss_gain_on_sales_of_ppe_operating_cf",
        ),
        (
            "ProceedsFromIssuanceOfBondsFinCF",
            "proceeds_from_issuance_of_bonds_financing_cf",
        ),
        (
            "IncreaseInShortTermLoansPayableFinCF",
            "increase_in_short_term_loans_payable_financing_cf",
        ),
        (
            "ProceedsFromCollectionOfGuaranteeDepositsInvCF",
            "proceeds_from_collection_of_guarantee_deposits_investing_cf",
        ),
    ] {
        assert_semantic_alias(local_name, expected_field, CashFlow, "cf");
    }
}

#[test]
fn high_frequency_aliases_are_namespace_gated() {
    for local_name in [
        "OtherCA",
        "IncomeTaxesPayable",
        "InterestExpensesNOE",
        "NetIncreaseDecreaseInCashAndCashEquivalents",
    ] {
        let concept = extension(local_name);
        assert!(canonicalize_semantic_concept(&concept).is_none());
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Extension
        ));
    }
}

#[test]
fn summary_business_result_items_remain_unknown_until_separately_reviewed() {
    for local_name in [
        "TotalAssetsSummaryOfBusinessResults",
        "NetAssetsSummaryOfBusinessResults",
        "NetAssetsPerShareSummaryOfBusinessResults",
    ] {
        assert!(matches!(
            concept_status(&jppfs(local_name)),
            TaxonomyConceptStatus::Unknown
        ));
    }
}

