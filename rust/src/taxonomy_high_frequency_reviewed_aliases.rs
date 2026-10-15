use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;
use crate::types::ConceptKey;

fn reviewed_jppfs_high_frequency_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Balance-sheet assets. Keep current assets, other non-current assets,
        // deferred tax assets, and land in separate buckets; collapsing these to
        // total assets would lose liquidity and asset-class information.
        "OtherCA" => "other_current_assets",
        "OtherIOA" => "other_investments_and_other_assets",
        "DeferredTaxAssets" => "deferred_tax_assets",
        "DeferredTaxAssetsCA" => "deferred_tax_assets_current",
        "DeferredTaxAssetsIOA" => "deferred_tax_assets_non_current",
        "ConstructionInProgress" => "construction_in_progress",
        "OtherIA" => "other_intangible_assets",
        "ToolsFurnitureAndFixturesNet" => "tools_furniture_and_fixtures_net",
        "Software" => "software",
        "WorkInProcess" => "work_in_process",
        "Goodwill" => "goodwill",
        "RawMaterialsAndSupplies" => "raw_materials_and_supplies",
        "MerchandiseAndFinishedGoods" => "merchandise_and_finished_goods",
        "LeaseAssetsNetPPE" => "lease_assets_net_ppe",
        "BuildingsNet" => "buildings_net",
        "PrepaidExpenses" => "prepaid_expenses_current",
        "StocksOfSubsidiariesAndAffiliates" => "stocks_of_subsidiaries_and_affiliates",
        "LongTermLoansReceivable" => "long_term_loans_receivable",
        "ToolsFurnitureAndFixtures" => "tools_furniture_and_fixtures",
        "NotesReceivableTrade" => "notes_receivable_trade",
        "ElectronicallyRecordedMonetaryClaimsOperatingCA" => {
            "electronically_recorded_monetary_claims_operating_current"
        }
        "LongTermPrepaidExpenses" => "long_term_prepaid_expenses",
        "BuildingsAndStructuresNet" => "buildings_and_structures_net",
        "OtherNetPPE" => "other_ppe_net",
        "VehiclesNet" => "vehicles_net",
        "StructuresNet" => "structures_net",
        "ShortTermInvestmentSecurities" => "short_term_investment_securities",
        "LeaseAssetsPPE" => "lease_assets_ppe",
        "MachineryAndEquipmentNet" => "machinery_and_equipment_net",
        "MachineryEquipmentAndVehiclesNet" => "machinery_equipment_and_vehicles_net",
        "BuildingsAndStructures" => "buildings_and_structures",
        "LeaseAndGuaranteeDeposits" => "lease_and_guarantee_deposits",
        "Merchandise" => "merchandise",
        "Supplies" => "supplies",
        "GuaranteeDepositsIOA" => "guarantee_deposits_non_current_assets",
        "ContractLiabilities" => "contract_liabilities",
        "Buildings" => "buildings",
        "SoftwareInProgress" => "software_in_progress",
        "OtherPPE" => "other_ppe",
        "InvestmentsInCapital" => "investments_in_capital",
        "LongTermLoansReceivableFromSubsidiariesAndAffiliates" => {
            "long_term_loans_receivable_subsidiaries_affiliates"
        }
        "MachineryEquipmentAndVehicles" => "machinery_equipment_and_vehicles",
        "ClaimsProvableInBankruptcyClaimsProvableInRehabilitationAndOther" => {
            "claims_provable_bankruptcy_rehabilitation_other"
        }
        "AdvancePaymentsTrade" => "advance_payments_trade",
        "InvestmentsInCapitalOfSubsidiariesAndAffiliates" => {
            "investments_in_capital_subsidiaries_affiliates"
        }
        "LeaseAssetsIA" => "lease_assets_intangible_assets",
        "ShortTermLoansReceivable" => "short_term_loans_receivable",
        "TelephoneSubscriptionRight" => "telephone_subscription_right",
        "NetDefinedBenefitAsset" => "net_defined_benefit_asset",
        "Vehicles" => "vehicles",
        "LeaseholdRight" => "leasehold_right",
        "LongTermGuaranteeDeposited" => "long_term_guarantee_deposits",
        "ShortTermLoansReceivableToSubsidiariesAndAffiliates" => {
            "short_term_loans_receivable_subsidiaries_affiliates"
        }
        "Structures" => "structures",
        "MachineryAndEquipment" => "machinery_and_equipment",
        "PrepaidPensionCostIOA" => "prepaid_pension_cost_non_current_assets",
        "DeferredAssets" => "deferred_assets",
        "IncomeTaxesReceivable" => "income_taxes_receivable",
        "FinishedGoods" => "finished_goods",
        "RightOfTrademark" => "trademark_right",
        "Land" => "land",

        // Balance-sheet liabilities. Income taxes payable is a tax liability;
        // OtherCL is a broad current-liability residual, not trade payables.
        "IncomeTaxesPayable" => "income_taxes_payable",
        "OtherCL" => "other_current_liabilities",
        "AccruedConsumptionTaxes" => "accrued_consumption_taxes_current",
        "OtherNCL" => "other_non_current_liabilities",
        "ProvisionForBonuses" => "provision_for_bonuses",
        "LeaseObligationsNCL" => "lease_obligations_non_current",
        "AssetRetirementObligationsNCL" => "asset_retirement_obligations_non_current",
        "LeaseObligationsCL" => "lease_obligations_current",
        "DepositsReceived" => "deposits_received",
        "DeferredTaxLiabilities" => "deferred_tax_liabilities",
        "DeferredTaxLiabilitiesNCL" => "deferred_tax_liabilities_non_current",
        "AdvancesReceived" => "advances_received_current",
        "ElectronicallyRecordedObligationsOperatingCL" => {
            "electronically_recorded_obligations_operating_current"
        }
        "NetDefinedBenefitLiability" => "net_defined_benefit_liability",
        "CurrentPortionOfBonds" => "current_portion_bonds_payable",
        "ProvisionForRetirementBenefits" => "provision_for_retirement_benefits",
        "LongTermAccountsPayableOther" => "long_term_accounts_payable_other",
        "DeferredTaxLiabilitiesForLandRevaluation" => "deferred_tax_liabilities_land_revaluation",
        "NotesPayableTrade" => "notes_payable_trade",
        "ProvisionForProductWarranties" => "provision_for_product_warranties",
        "AssetRetirementObligationsCL" => "asset_retirement_obligations_current",
        "UnearnedRevenue" => "unearned_revenue",
        "ProvisionForLossOnConstructionContracts" => "provision_for_loss_on_construction_contracts",
        "ProvisionForWarrantiesForCompletedConstruction" => {
            "provision_for_warranties_for_completed_construction"
        }
        "ProvisionForLossOnOrderReceivedCL" => "provision_for_loss_on_order_received_current",
        "ProvisionForPointCardCertificatesCL" => "provision_for_point_card_certificates_current",
        "AdvancesReceivedOnUncompletedConstructionContractsCNS" => {
            "advances_received_on_uncompleted_construction_contracts"
        }

        // Equity / net-assets components. Treasury stock is a contra-equity
        // balance-sheet account and must not be merged with treasury-stock
        // purchase cash-flow tags.
        "TreasuryStock" => "treasury_stock",
        "SubscriptionRightsToShares" => "subscription_rights_to_shares",
        "RetainedEarningsBroughtForward" => "retained_earnings_brought_forward",
        "DeferredGainsOrLossesOnHedges" => "deferred_gains_or_losses_on_hedges_equity",
        "LegalRetainedEarnings" => "legal_retained_earnings",
        "OtherCapitalSurplus" => "other_capital_surplus",
        "GeneralReserve" => "general_reserve",
        "NonControllingInterests" => "noncontrolling_interests",
        "ValuationAndTranslationAdjustments" => "valuation_and_translation_adjustments",
        "ValuationDifferenceOnAvailableForSaleSecurities" => {
            "valuation_difference_on_available_for_sale_securities_equity"
        }
        "ForeignCurrencyTranslationAdjustment" => "foreign_currency_translation_adjustment_equity",
        "RemeasurementsOfDefinedBenefitPlans" => "remeasurements_of_defined_benefit_plans_equity",
        "RevaluationReserveForLand" => "revaluation_reserve_for_land",
        "ReserveForAdvancedDepreciationOfNoncurrentAssets" => {
            "reserve_for_advanced_depreciation_of_noncurrent_assets"
        }

        // Allowances are contra-assets. Current-asset and investments-and-other
        // asset presentations are kept separate from each other and from bad
        // debt expense / allowance provision PL tags.
        "AllowanceForDoubtfulAccountsCA" => "allowance_for_doubtful_accounts_current_assets",
        "AllowanceForDoubtfulAccountsIOAByGroup" => {
            "allowance_for_doubtful_accounts_investments_and_other_assets"
        }
        "AccumulatedDepreciationToolsFurnitureAndFixtures"
        | "AccumulatedDepreciationPPEByGroup"
        | "AccumulatedDepreciationOtherPPE"
        | "AccumulatedDepreciationStructures" => "accumulated_depreciation",

        // Profit-and-loss finance / extraordinary items. NOI/NOE/EI/EL suffixes
        // identify presentation sections, but the line item itself remains a
        // specific income or expense account.
        "InterestExpensesNOE" => "interest_expenses",
        "InterestIncomeNOI" => "interest_income",
        "ImpairmentLossEL" => "impairment_loss",
        "DividendsIncomeNOI" => "dividend_income",
        "ForeignExchangeGainsNOI" => "foreign_exchange_gains",
        "ForeignExchangeLossesNOE" => "foreign_exchange_losses",
        "GainOnSalesOfNoncurrentAssetsEI" => "gain_on_sales_of_noncurrent_assets",
        "GainOnSalesOfInvestmentSecuritiesEI" => "gain_on_sales_of_investment_securities",
        "LossOnRetirementOfNoncurrentAssetsEL" => "loss_on_retirement_of_noncurrent_assets",
        "LossOnValuationOfInvestmentSecuritiesEL" => "loss_on_valuation_of_investment_securities",
        "LossOnSalesOfNoncurrentAssetsEL" => "loss_on_sales_of_noncurrent_assets",
        "CommissionFeeNOE" => "commission_fee_non_operating_expenses",
        "DepreciationSGA" => "depreciation_selling_general_admin",
        "RetirementBenefitExpensesSGA" => "retirement_benefit_expenses_selling_general_admin",
        "ProvisionForBonusesSGA" => "provision_for_bonuses_selling_general_admin",
        "RentIncomeNOI" => "rent_income",
        "MiscellaneousIncomeNOI" => "miscellaneous_income",
        "SubsidyIncomeNOIBounty" | "SubsidyIncomeNOI" => "subsidy_income_non_operating",
        "LossOnValuationOfStocksOfSubsidiariesAndAffiliatesEL" => {
            "loss_on_valuation_of_stocks_of_subsidiaries_and_affiliates"
        }
        "InterestAndDividendsIncomeNOI" => "interest_and_dividends_income",
        "LossOnSalesOfInvestmentSecuritiesEL" => "loss_on_sales_of_investment_securities",
        "ProvisionOfAllowanceForDoubtfulAccountsSGA" => {
            "provision_of_allowance_for_doubtful_accounts_sga"
        }
        "ProvisionOfAllowanceForDoubtfulAccountsNOE" => {
            "provision_of_allowance_for_doubtful_accounts_non_operating"
        }
        "LossOnDisposalOfNoncurrentAssetsEL" => "loss_on_disposal_of_noncurrent_assets",
        "GainOnReversalOfSubscriptionRightsToSharesEI" => {
            "gain_on_reversal_of_subscription_rights_to_shares"
        }
        "DirectorsCompensationsSGA" => "directors_compensation_selling_general_admin",
        "MiscellaneousLossNOE" => "miscellaneous_loss_non_operating",
        "SalariesAndAllowancesSGA" => "salaries_and_allowances_selling_general_admin",
        "LossOnSalesAndRetirementOfNoncurrentAssetsEL" => {
            "loss_on_sales_and_retirement_of_noncurrent_assets"
        }
        "InsuranceIncomeNOI" => "insurance_income_non_operating",
        "EquityInEarningsOfAffiliatesNOI" => "equity_in_earnings_of_affiliates",
        "AmortizationExpensesOfGoodwillSGA" | "AmortizationOfGoodwillSGA" => {
            "amortization_of_goodwill_selling_general_admin"
        }
        "ReversalOfAllowanceForDoubtfulAccountsNOI" => {
            "reversal_of_allowance_for_doubtful_accounts"
        }
        "GainOnSalesOfSubsidiariesAndAffiliatesStocksEI" => {
            "gain_on_sales_of_subsidiaries_and_affiliates_stocks"
        }
        "AdvertisingExpensesSGA" => "advertising_expenses_selling_general_admin",
        "SubsidyEI" => "subsidy_income_extraordinary",
        "InterestExpensesOEBNK" => "interest_expenses_bank_operating",
        "InterestIncomeOIBNK" => "interest_income_bank_operating",
        "StockIssuanceCostNOE" => "stock_issuance_cost_non_operating",
        "RentExpensesSGA" => "rent_expenses_selling_general_admin",
        "InsuranceIncomeEI" => "insurance_income_extraordinary",
        "LossOnDisasterEL" => "loss_on_disaster_extraordinary",
        "WelfareExpensesSGA" => "welfare_expenses_selling_general_admin",
        "SalesDiscountsNOE" => "sales_discounts_non_operating",
        "ProvisionForDirectorsBonusesSGA" => {
            "provision_for_directors_bonuses_selling_general_admin"
        }
        "LossOnRetirementOfNoncurrentAssetsNOE" => {
            "loss_on_retirement_of_noncurrent_assets_non_operating"
        }
        "LossOnInvestmentsInPartnershipNOE" => "loss_on_investments_in_partnership_non_operating",
        "ProvisionForDirectorsRetirementBenefitsSGA" => {
            "provision_for_directors_retirement_benefits_selling_general_admin"
        }
        "InterestOnBondsNOE" => "interest_expenses",
        "RealEstateRentNOI" => "real_estate_rent_income",
        "ResearchAndDevelopmentExpensesSGA" => "research_and_development",
        "TaxesAndDuesSGA" => "taxes_and_dues_selling_general_admin",
        "PurchaseDiscountsNOI" => "purchase_discounts_non_operating",
        "LossOnReductionOfNoncurrentAssetsEL" => {
            "loss_on_reduction_of_noncurrent_assets_extraordinary"
        }
        "CommissionFeeSGA" => "commission_fee_selling_general_admin",
        "EquityInLossesOfAffiliatesNOE" => "equity_in_losses_of_affiliates",
        "ProvisionOfAllowanceForDoubtfulAccountsEL" => {
            "provision_of_allowance_for_doubtful_accounts_extraordinary"
        }
        "LegalWelfareExpensesSGA" => "legal_welfare_expenses_selling_general_admin",

        // Cash-flow statement subtotal and working-capital movements. The
        // canonical names retain operating / investing / financing context so a
        // role suffix cannot be confused with the related BS or PL account.
        "NetIncreaseDecreaseInCashAndCashEquivalents" => {
            "net_increase_decrease_in_cash_and_cash_equivalents"
        }
        "SubtotalOpeCF" => "subtotal_operating_cf",
        "OtherNetOpeCF" => "other_net_operating_cf",
        "DecreaseIncreaseInNotesAndAccountsReceivableTradeOpeCF" => {
            "decrease_increase_in_trade_receivables_operating_cf"
        }
        "IncreaseDecreaseInNotesAndAccountsPayableTradeOpeCF" => {
            "increase_decrease_in_trade_payables_operating_cf"
        }
        "DecreaseIncreaseInInventoriesOpeCF" => "decrease_increase_in_inventories_operating_cf",
        "IncomeTaxesPaidOpeCF" => "income_taxes_paid_operating_cf",
        "InterestAndDividendsIncomeOpeCF" => "interest_and_dividends_income_operating_cf",
        "InterestAndDividendsIncomeReceivedOpeCFInvCF" => {
            "interest_and_dividends_received_operating_investing_cf"
        }
        "InterestExpensesPaidOpeCFFinCF" => "interest_expenses_paid_operating_financing_cf",
        "IncreaseDecreaseInAllowanceForDoubtfulAccountsOpeCF" => {
            "increase_decrease_in_allowance_for_doubtful_accounts_operating_cf"
        }
        "OtherNetInvCF" => "other_net_investing_cf",
        "EffectOfExchangeRateChangeOnCashAndCashEquivalents" => {
            "effect_of_exchange_rate_change_on_cash_and_cash_equivalents"
        }
        "IncreaseDecreaseInProvisionForBonusesOpeCF" => {
            "increase_decrease_in_provision_for_bonuses_operating_cf"
        }
        "IncreaseDecreaseInNetDefinedBenefitLiabilityOpeCF" => {
            "increase_decrease_in_net_defined_benefit_liability_operating_cf"
        }
        "NetIncreaseDecreaseInShortTermLoansPayableFinCF" => {
            "net_increase_decrease_in_short_term_loans_payable_financing_cf"
        }
        "LossGainOnSalesOfInvestmentSecuritiesOpeCF" => {
            "loss_gain_on_sales_of_investment_securities_operating_cf"
        }
        "RepaymentsOfLeaseObligationsFinCF" => "repayments_of_lease_obligations_financing_cf",
        "IncreaseDecreaseInAccruedConsumptionTaxesOpeCF" => {
            "increase_decrease_in_accrued_consumption_taxes_operating_cf"
        }
        "OtherNetFinCF" => "other_net_financing_cf",
        "AmortizationOfGoodwillOpeCF" => "amortization_of_goodwill_operating_cf",
        "PaymentsIntoTimeDepositsInvCF" => "payments_into_time_deposits_investing_cf",
        "ProceedsFromWithdrawalOfTimeDepositsInvCF" => {
            "proceeds_from_withdrawal_of_time_deposits_investing_cf"
        }
        "LossOnRetirementOfNoncurrentAssetsOpeCF" => {
            "loss_on_retirement_of_noncurrent_assets_operating_cf"
        }
        "ForeignExchangeLossesGainsOpeCF" => "foreign_exchange_losses_gains_operating_cf",
        "EquityInEarningsLossesOfAffiliatesOpeCF" => {
            "equity_in_earnings_losses_of_affiliates_operating_cf"
        }
        "CollectionOfLoansReceivableInvCF" => "collection_of_loans_receivable_investing_cf",
        "IncreaseDecreaseInAccountsPayableOtherOpeCF" => {
            "increase_decrease_in_other_accounts_payable_operating_cf"
        }
        "LossGainOnValuationOfInvestmentSecuritiesOpeCF" => {
            "loss_gain_on_valuation_of_investment_securities_operating_cf"
        }
        "DividendsPaidToNonControllingInterestsFinCF" => {
            "dividends_paid_to_noncontrolling_interests_financing_cf"
        }
        "IncreaseDecreaseInProvisionForDirectorsRetirementBenefitsOpeCF" => {
            "increase_decrease_in_provision_for_directors_retirement_benefits_operating_cf"
        }
        "LossGainOnSalesOfNoncurrentAssetsOpeCF" => {
            "loss_gain_on_sales_of_noncurrent_assets_operating_cf"
        }
        "PaymentsOfLoansReceivableInvCF" => "payments_of_loans_receivable_investing_cf",
        "IncomeTaxesRefundOpeCF" => "income_taxes_refund_operating_cf",
        "IncreaseDecreaseInProvisionForDirectorsBonusesOpeCF" => {
            "increase_decrease_in_provision_for_directors_bonuses_operating_cf"
        }
        "RedemptionOfBondsFinCF" => "redemption_of_bonds_financing_cf",
        "PurchaseOfInvestmentsInSubsidiariesResultingInChangeInScopeOfConsolidationInvCF" => {
            "purchase_of_investments_in_subsidiaries_resulting_in_change_in_scope_of_consolidation_investing_cf"
        }
        "PaymentsForLeaseAndGuaranteeDepositsInvCF" => {
            "payments_for_lease_and_guarantee_deposits_investing_cf"
        }
        "ProceedsFromCollectionOfLeaseAndGuaranteeDepositsInvCF" => {
            "proceeds_from_collection_of_lease_and_guarantee_deposits_investing_cf"
        }
        "IncreaseDecreaseInAdvancesReceivedOpeCF" => {
            "increase_decrease_in_advances_received_operating_cf"
        }
        "ProceedsFromIssuanceOfCommonStockFinCF" => {
            "proceeds_from_issuance_of_common_stock_financing_cf"
        }
        "IncreaseDecreaseInAccruedExpensesOpeCF" => {
            "increase_decrease_in_accrued_expenses_operating_cf"
        }
        "IncreaseDecreaseInNetDefinedBenefitAssetOpeCF" => {
            "increase_decrease_in_net_defined_benefit_asset_operating_cf"
        }
        "IncomeTaxesPaidRefundOpeCF" => "income_taxes_paid_refund_operating_cf",
        "PaymentsFromChangesInOwnershipInterestsInSubsidiariesThatDoNotResultInChangeInScopeOfConsolidationFinCF" => {
            "payments_from_changes_in_ownership_interests_in_subsidiaries_without_scope_change_financing_cf"
        }
        "PaymentsForGuaranteeDepositsInvCF" => "payments_for_guarantee_deposits_investing_cf",
        "DecreaseInShortTermLoansPayableFinCF" => {
            "decrease_in_short_term_loans_payable_financing_cf"
        }
        "IncreaseDecreaseInOtherCurrentLiabilitiesOpeCF" => {
            "increase_decrease_in_other_current_liabilities_operating_cf"
        }
        "DecreaseIncreaseInOtherCurrentAssetsOpeCF" => {
            "decrease_increase_in_other_current_assets_operating_cf"
        }
        "LossGainOnSalesOfPropertyPlantAndEquipmentOpeCF" => {
            "loss_gain_on_sales_of_ppe_operating_cf"
        }
        "ProceedsFromIssuanceOfBondsFinCF" => "proceeds_from_issuance_of_bonds_financing_cf",
        "IncreaseInShortTermLoansPayableFinCF" => {
            "increase_in_short_term_loans_payable_financing_cf"
        }
        "ProceedsFromCollectionOfGuaranteeDepositsInvCF" => {
            "proceeds_from_collection_of_guarantee_deposits_investing_cf"
        }

        _ => return None,
    };
    Some(field)
}

fn is_jppfs_namespace(uri: &str) -> bool {
    uri.contains("/taxonomy/jppfs/")
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !is_jppfs_namespace(&concept.0) {
        return None;
    }

    let field = reviewed_jppfs_high_frequency_alias(&concept.1)?;
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
    reviewed_jppfs_high_frequency_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

fn reviewed_statement_for_item_name(item_name: &str) -> Option<&'static str> {
    let statement = match item_name {
        "other_current_assets"
        | "other_investments_and_other_assets"
        | "deferred_tax_assets"
        | "deferred_tax_assets_current"
        | "deferred_tax_assets_non_current"
        | "construction_in_progress"
        | "other_intangible_assets"
        | "tools_furniture_and_fixtures_net"
        | "software"
        | "work_in_process"
        | "goodwill"
        | "raw_materials_and_supplies"
        | "merchandise_and_finished_goods"
        | "lease_assets_net_ppe"
        | "buildings_net"
        | "prepaid_expenses_current"
        | "stocks_of_subsidiaries_and_affiliates"
        | "long_term_loans_receivable"
        | "tools_furniture_and_fixtures"
        | "notes_receivable_trade"
        | "electronically_recorded_monetary_claims_operating_current"
        | "long_term_prepaid_expenses"
        | "buildings_and_structures_net"
        | "other_ppe_net"
        | "vehicles_net"
        | "structures_net"
        | "short_term_investment_securities"
        | "lease_assets_ppe"
        | "machinery_and_equipment_net"
        | "machinery_equipment_and_vehicles_net"
        | "buildings_and_structures"
        | "lease_and_guarantee_deposits"
        | "merchandise"
        | "supplies"
        | "guarantee_deposits_non_current_assets"
        | "contract_liabilities"
        | "buildings"
        | "software_in_progress"
        | "other_ppe"
        | "investments_in_capital"
        | "long_term_loans_receivable_subsidiaries_affiliates"
        | "machinery_equipment_and_vehicles"
        | "claims_provable_bankruptcy_rehabilitation_other"
        | "advance_payments_trade"
        | "investments_in_capital_subsidiaries_affiliates"
        | "lease_assets_intangible_assets"
        | "short_term_loans_receivable"
        | "telephone_subscription_right"
        | "net_defined_benefit_asset"
        | "vehicles"
        | "leasehold_right"
        | "long_term_guarantee_deposits"
        | "short_term_loans_receivable_subsidiaries_affiliates"
        | "structures"
        | "machinery_and_equipment"
        | "prepaid_pension_cost_non_current_assets"
        | "deferred_assets"
        | "income_taxes_receivable"
        | "finished_goods"
        | "trademark_right"
        | "land"
        | "income_taxes_payable"
        | "other_current_liabilities"
        | "accrued_consumption_taxes_current"
        | "other_non_current_liabilities"
        | "provision_for_bonuses"
        | "lease_obligations_non_current"
        | "asset_retirement_obligations_non_current"
        | "lease_obligations_current"
        | "deposits_received"
        | "deferred_tax_liabilities"
        | "deferred_tax_liabilities_non_current"
        | "advances_received_current"
        | "electronically_recorded_obligations_operating_current"
        | "net_defined_benefit_liability"
        | "current_portion_bonds_payable"
        | "provision_for_retirement_benefits"
        | "long_term_accounts_payable_other"
        | "deferred_tax_liabilities_land_revaluation"
        | "notes_payable_trade"
        | "provision_for_product_warranties"
        | "asset_retirement_obligations_current"
        | "unearned_revenue"
        | "provision_for_loss_on_construction_contracts"
        | "provision_for_warranties_for_completed_construction"
        | "provision_for_loss_on_order_received_current"
        | "provision_for_point_card_certificates_current"
        | "advances_received_on_uncompleted_construction_contracts"
        | "treasury_stock"
        | "subscription_rights_to_shares"
        | "retained_earnings_brought_forward"
        | "deferred_gains_or_losses_on_hedges_equity"
        | "legal_retained_earnings"
        | "other_capital_surplus"
        | "general_reserve"
        | "noncontrolling_interests"
        | "valuation_and_translation_adjustments"
        | "valuation_difference_on_available_for_sale_securities_equity"
        | "foreign_currency_translation_adjustment_equity"
        | "remeasurements_of_defined_benefit_plans_equity"
        | "revaluation_reserve_for_land"
        | "reserve_for_advanced_depreciation_of_noncurrent_assets"
        | "allowance_for_doubtful_accounts_current_assets"
        | "allowance_for_doubtful_accounts_investments_and_other_assets"
        | "accumulated_depreciation" => "bs",

        "dividend_income"
        | "foreign_exchange_gains"
        | "foreign_exchange_losses"
        | "gain_on_sales_of_noncurrent_assets"
        | "gain_on_sales_of_investment_securities"
        | "loss_on_retirement_of_noncurrent_assets"
        | "loss_on_valuation_of_investment_securities"
        | "loss_on_sales_of_noncurrent_assets"
        | "commission_fee_non_operating_expenses"
        | "depreciation_selling_general_admin"
        | "retirement_benefit_expenses_selling_general_admin"
        | "provision_for_bonuses_selling_general_admin"
        | "rent_income"
        | "miscellaneous_income"
        | "subsidy_income_non_operating"
        | "loss_on_valuation_of_stocks_of_subsidiaries_and_affiliates"
        | "interest_and_dividends_income"
        | "loss_on_sales_of_investment_securities"
        | "provision_of_allowance_for_doubtful_accounts_sga"
        | "provision_of_allowance_for_doubtful_accounts_non_operating"
        | "loss_on_disposal_of_noncurrent_assets"
        | "gain_on_reversal_of_subscription_rights_to_shares"
        | "directors_compensation_selling_general_admin"
        | "miscellaneous_loss_non_operating"
        | "salaries_and_allowances_selling_general_admin"
        | "loss_on_sales_and_retirement_of_noncurrent_assets"
        | "insurance_income_non_operating"
        | "equity_in_earnings_of_affiliates"
        | "amortization_of_goodwill_selling_general_admin"
        | "reversal_of_allowance_for_doubtful_accounts"
        | "gain_on_sales_of_subsidiaries_and_affiliates_stocks"
        | "advertising_expenses_selling_general_admin"
        | "subsidy_income_extraordinary"
        | "interest_expenses_bank_operating"
        | "interest_income_bank_operating"
        | "stock_issuance_cost_non_operating"
        | "rent_expenses_selling_general_admin"
        | "insurance_income_extraordinary"
        | "loss_on_disaster_extraordinary"
        | "welfare_expenses_selling_general_admin"
        | "sales_discounts_non_operating"
        | "provision_for_directors_bonuses_selling_general_admin"
        | "loss_on_retirement_of_noncurrent_assets_non_operating"
        | "loss_on_investments_in_partnership_non_operating"
        | "provision_for_directors_retirement_benefits_selling_general_admin"
        | "real_estate_rent_income"
        | "taxes_and_dues_selling_general_admin"
        | "purchase_discounts_non_operating"
        | "loss_on_reduction_of_noncurrent_assets_extraordinary"
        | "commission_fee_selling_general_admin"
        | "equity_in_losses_of_affiliates"
        | "provision_of_allowance_for_doubtful_accounts_extraordinary"
        | "legal_welfare_expenses_selling_general_admin" => "pl",

        "net_increase_decrease_in_cash_and_cash_equivalents"
        | "subtotal_operating_cf"
        | "other_net_operating_cf"
        | "decrease_increase_in_trade_receivables_operating_cf"
        | "increase_decrease_in_trade_payables_operating_cf"
        | "decrease_increase_in_inventories_operating_cf"
        | "income_taxes_paid_operating_cf"
        | "interest_and_dividends_income_operating_cf"
        | "interest_and_dividends_received_operating_investing_cf"
        | "interest_expenses_paid_operating_financing_cf"
        | "increase_decrease_in_allowance_for_doubtful_accounts_operating_cf"
        | "other_net_investing_cf"
        | "effect_of_exchange_rate_change_on_cash_and_cash_equivalents"
        | "increase_decrease_in_provision_for_bonuses_operating_cf"
        | "increase_decrease_in_net_defined_benefit_liability_operating_cf"
        | "net_increase_decrease_in_short_term_loans_payable_financing_cf"
        | "loss_gain_on_sales_of_investment_securities_operating_cf"
        | "repayments_of_lease_obligations_financing_cf"
        | "increase_decrease_in_accrued_consumption_taxes_operating_cf"
        | "other_net_financing_cf"
        | "amortization_of_goodwill_operating_cf"
        | "payments_into_time_deposits_investing_cf"
        | "proceeds_from_withdrawal_of_time_deposits_investing_cf"
        | "loss_on_retirement_of_noncurrent_assets_operating_cf"
        | "foreign_exchange_losses_gains_operating_cf"
        | "equity_in_earnings_losses_of_affiliates_operating_cf"
        | "collection_of_loans_receivable_investing_cf"
        | "increase_decrease_in_other_accounts_payable_operating_cf"
        | "loss_gain_on_valuation_of_investment_securities_operating_cf"
        | "dividends_paid_to_noncontrolling_interests_financing_cf"
        | "increase_decrease_in_provision_for_directors_retirement_benefits_operating_cf"
        | "loss_gain_on_sales_of_noncurrent_assets_operating_cf"
        | "payments_of_loans_receivable_investing_cf"
        | "income_taxes_refund_operating_cf"
        | "increase_decrease_in_provision_for_directors_bonuses_operating_cf"
        | "redemption_of_bonds_financing_cf"
        | "purchase_of_investments_in_subsidiaries_resulting_in_change_in_scope_of_consolidation_investing_cf"
        | "payments_for_lease_and_guarantee_deposits_investing_cf"
        | "proceeds_from_collection_of_lease_and_guarantee_deposits_investing_cf"
        | "increase_decrease_in_advances_received_operating_cf"
        | "proceeds_from_issuance_of_common_stock_financing_cf"
        | "increase_decrease_in_accrued_expenses_operating_cf"
        | "increase_decrease_in_net_defined_benefit_asset_operating_cf"
        | "income_taxes_paid_refund_operating_cf"
        | "payments_from_changes_in_ownership_interests_in_subsidiaries_without_scope_change_financing_cf"
        | "payments_for_guarantee_deposits_investing_cf"
        | "decrease_in_short_term_loans_payable_financing_cf"
        | "increase_decrease_in_other_current_liabilities_operating_cf"
        | "decrease_increase_in_other_current_assets_operating_cf"
        | "loss_gain_on_sales_of_ppe_operating_cf"
        | "proceeds_from_issuance_of_bonds_financing_cf"
        | "increase_in_short_term_loans_payable_financing_cf"
        | "proceeds_from_collection_of_guarantee_deposits_investing_cf" => "cf",

        _ => return None,
    };
    Some(statement)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match reviewed_statement_for_item_name(item_name) {
        Some("bs") => Some(StatementCategory::BalanceSheet),
        Some("pl") => Some(StatementCategory::ProfitAndLoss),
        Some("cf") => Some(StatementCategory::CashFlow),
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    reviewed_statement_for_item_name(item_name).or_else(|| base::statement_for_item_name(item_name))
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
