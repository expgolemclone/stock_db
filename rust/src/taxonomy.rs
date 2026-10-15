use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde_json::Value;

use crate::types::ConceptKey;

const TAXONOMY_JSON: &str = include_str!("../../src/stock_db/storage/taxonomy.json");
#[cfg(test)]
const IDENTITY_CONCEPTS: &str = include_str!("taxonomy_identity_concepts.txt");
#[cfg(not(test))]
const IDENTITY_CONCEPTS: &str = "";

/// Recognized XBRL taxonomy family, derived from a concept's namespace URI.
///
/// EDINET publishes its taxonomies under date-stamped URIs such as
/// `http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/<DATE>/jppfs_cor`. The
/// `<DATE>` segment varies per taxonomy year, so family recognition matches the
/// taxonomy token rather than the full URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonomyFamily {
    /// EDINET J-GAAP primary financial statements (jppfs).
    EdinetJppfs,
    /// EDINET IFRS group reporting (jpigp).
    EdinetJpigp,
    /// EDINET corporate reporting / disclosure ordinance (jpcrp).
    EdinetJpcrp,
    /// EDINET document & entity information (jpdei).
    EdinetJpDei,
    /// FASB US-GAAP taxonomy.
    UsGaap,
    /// IFRS Foundation taxonomy.
    Ifrs,
}

/// How a concept was matched during canonicalization. Distinguishes full-QName
/// overrides, family-scoped reviewed mappings, cross-family safe aliases, and
/// identity (snake_case candidate) resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// A full expanded-name (namespace URI + local name) override.
    ExactExpandedName,
    /// A reviewed mapping scoped to a specific recognized taxonomy family.
    TaxonomyFamilyAlias,
    /// A reviewed local-name alias valid across all recognized families.
    SafeLocalAlias,
    /// A known identity concept with no reviewed mapping (snake_case candidate).
    Identity,
}

impl MatchKind {
    /// Stable lowercase label used in the taxonomy audit output.
    pub fn as_str(self) -> &'static str {
        match self {
            MatchKind::ExactExpandedName => "exact_expanded_name",
            MatchKind::TaxonomyFamilyAlias => "taxonomy_family_alias",
            MatchKind::SafeLocalAlias => "safe_local_alias",
            MatchKind::Identity => "identity",
        }
    }
}

/// Financial statement category for a canonical field name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementCategory {
    BalanceSheet,
    ProfitAndLoss,
    CashFlow,
    ChangesInEquity,
    OtherComprehensiveIncome,
    PerShare,
    ShareData,
    Forecast,
    Disclosure,
    Other,
}

/// Classify a canonical field name into its financial statement category.
///
/// This is a secondary classifier for canonical field names. Real statement
/// classification should prefer presentation-linkbase roles when available.
pub fn classify_item_name(name: &str) -> Option<StatementCategory> {
    use StatementCategory::*;
    let category = match name {
        // ── BS ──
        "total_assets"
        | "current_assets"
        | "non_current_assets"
        | "current_liabilities"
        | "non_current_liabilities"
        | "cash_and_deposits"
        | "cash_and_cash_equivalents"
        | "notes_and_accounts_receivable_trade"
        | "trade_receivables"
        | "trade_and_other_current_receivables"
        | "accounts_receivable_trade"
        | "accounts_receivable_net"
        | "accounts_receivable_net_current"
        | "notes_and_accounts_payable_trade"
        | "trade_payables"
        | "trade_and_other_current_payables"
        | "accounts_payable_trade"
        | "accounts_payable_current"
        | "inventories"
        | "retained_earnings"
        | "stockholders_equity" // legacy canonical field
        | "shareholders_equity"
        | "equity_attributable_to_owners_of_parent"
        | "noncontrolling_interests"
        | "total_equity"
        | "capital_stock"
        | "total_liabilities"
        | "tangible_fixed_assets"
        | "intangible_fixed_assets"
        | "intangible_assets"
        | "investment_securities"
        | "corporate_bonds_investment_securities_assets"
        | "long_term_loans"
        | "short_term_loans"
        | "long_term_debt"
        | "short_term_debt"
        | "bonds_payable"
        | "bonds_and_borrowings"
        | "debt_securities"
        | "investments_and_other_assets"
        | "capital_surplus"
        | "share_premium"
        | "additional_paid_in_capital"
        | "legal_capital_surplus"
        | "total_liabilities_and_net_assets"
        | "current_portion_long_term_loans_payable"
        | "provision_for_directors_bonuses"
        | "provision_for_directors_retirement_benefits"
        | "commercial_paper" => BalanceSheet,

        // ── PL ──
        "revenue"
        | "operating_revenue_oil_telecommunications"
        | "operating_revenue_incidental_elc"
        | "cost_of_revenue"
        | "gross_profit"
        | "selling_general_admin"
        | "research_and_development"
        | "operating_income"
        | "ordinary_income"
        | "profit_loss_from_continuing_operations"
        | "income_before_tax"
        | "income_before_tax_continuing_operations"
        | "net_income"
        | "net_income_parent"
        | "net_income_minority"
        | "depreciation"
        | "amortization"
        | "depreciation_and_amortization"
        | "interest_expenses"
        | "interest_income"
        | "finance_costs"
        | "finance_income"
        | "non_operating_income"
        | "non_operating_expenses"
        | "other_income"
        | "other_expense_by_nature"
        | "other_gains_losses"
        | "extraordinary_income"
        | "extraordinary_loss"
        | "income_taxes"
        | "income_taxes_continuing_operations"
        | "income_taxes_current"
        | "income_taxes_deferred"
        | "impairment_loss"
        | "commission_fee_cost"
        | "commission_fee_extraordinary_income"
        | "commission_fee_non_operating_income"
        | "other_non_operating_income"
        | "other_non_operating_expenses"
        | "other_selling_general_admin"
        | "operating_expenses_ins"
        | "other_extraordinary_loss"
        | "other_extraordinary_income"
        | "other_operating_revenue"
        | "other_cost_of_sales"
        | "other_cost_of_sales_expenses_operating_activities" => ProfitAndLoss,

        // ── CF ──
        "operating_cf"
        | "investing_cf"
        | "financing_cf"
        | "capex"
        | "intangible_asset_purchases"
        | "ppe_sold"
        | "investment_securities_sold"
        | "investment_securities_purchased"
        | "cash_dividends_paid"
        | "cash_dividends_paid_financing_cf"
        | "dividends_paid_financing_cf"
        | "dividends_paid_investing_cf"
        | "depreciation_cf_adjustment"
        | "amortization_cf_adjustment"
        | "depreciation_and_amortization_cf_adjustment"
        | "intangible_assets_amortization_cf_adjustment"
        | "extraordinary_loss_cf_adjustment"
        | "impairment_loss_cf_adjustment"
        | "income_taxes_deferred_cf_adjustment"
        | "income_before_tax_operating_cf"
        | "operating_income_operating_cf"
        | "interest_expenses_financing_cf"
        | "interest_expenses_operating_cf"
        | "interest_income_operating_cf"
        | "current_portion_long_term_loans_payable_operating_cf"
        | "commission_fee_financing_cf"
        | "cash_and_cash_equivalents_beginning"
        | "cash_and_cash_equivalents_end"
        | "treasury_stock_purchase"
        | "treasury_stock_change"
        | "treasury_stock_change_financing_cf"
        | "lt_debt_repayment"
        | "lt_debt_proceeds" => CashFlow,

        // ── changes in equity ──
        "dividends_from_surplus" | "dividends_paid_from_surplus" => ChangesInEquity,

        // ── OCI ──
        "other_comprehensive_income"
        | "total_comprehensive_income"
        | "revaluation_surplus"
        | "foreign_currency_translation_adjustment"
        | "available_for_sale_securities_valuation_difference"
        | "cash_flow_hedge_valuation_difference"
        | "remeasurement_of_defined_benefit_plans" => OtherComprehensiveIncome,

        // ── per-share / share data ──
        "dividends_per_share"
        | "dividends_per_share_summary"
        | "interim_dividends_per_share_summary"
        | "dividends_per_share_from_surplus"
        | "earnings_per_share"
        | "diluted_earnings_per_share" => PerShare,
        "shares_outstanding" => ShareData,

        // ── forecast / disclosure ──
        "forecast_revenue"
        | "forecast_operating_income"
        | "forecast_ordinary_income"
        | "forecast_net_income" => Forecast,
        "roe" | "num_employees" | "payout_ratio" => Disclosure,
        "has_preferred_shares" => Other,

        _ => return None,
    };
    Some(category)
}

/// Base stems for key financial items.  Suffix-aware normalization strips known
/// suffixes (IFRS, industry codes, role codes, etc.) from a raw concept name and
/// looks up the resulting stem here.  This absorbs tag-name variations across
/// IFRS / J-GAAP / US-GAAP and industry-specific XBRL extensions.
const STEM_MAP: &[(&str, &str)] = &[
    // ── BS ──
    ("Assets", "total_assets"),
    ("CurrentAssets", "current_assets"),
    ("NoncurrentAssets", "non_current_assets"),
    ("CurrentLiabilities", "current_liabilities"),
    ("TotalCurrentLiabilities", "current_liabilities"),
    ("NoncurrentLiabilities", "non_current_liabilities"),
    ("NonCurrentLiabilities", "non_current_liabilities"),
    ("NonCurrentLabilities", "non_current_liabilities"),
    ("CashAndDeposits", "cash_and_deposits"),
    ("CashAndCashEquivalents", "cash_and_cash_equivalents"),
    (
        "CashAndCashEquivalentsAtBeginningOfPeriod",
        "cash_and_cash_equivalents_beginning",
    ),
    (
        "CashAndCashEquivalentsAtEndOfPeriod",
        "cash_and_cash_equivalents_end",
    ),
    (
        "NotesAndAccountsReceivableTrade",
        "notes_and_accounts_receivable_trade",
    ),
    ("TradeReceivables", "trade_receivables"),
    (
        "TradeAndOtherCurrentReceivables",
        "trade_and_other_current_receivables",
    ),
    ("AccountsReceivableTrade", "accounts_receivable_trade"),
    ("AccountsReceivableNet", "accounts_receivable_net"),
    (
        "AccountsReceivableNetCurrent",
        "accounts_receivable_net_current",
    ),
    (
        "NotesAndAccountsPayableTrade",
        "notes_and_accounts_payable_trade",
    ),
    ("TradePayables", "trade_payables"),
    (
        "TradeAndOtherCurrentPayables",
        "trade_and_other_current_payables",
    ),
    ("AccountsPayableTrade", "accounts_payable_trade"),
    ("AccountsPayableCurrent", "accounts_payable_current"),
    ("AccountsPayableTradeCurrent", "accounts_payable_trade"),
    ("Inventories", "inventories"),
    ("RetainedEarnings", "retained_earnings"),
    ("RetainedEarningsAccumulatedDeficit", "retained_earnings"),
    ("ShareholdersEquity", "shareholders_equity"),
    (
        "StockholdersEquity",
        "equity_attributable_to_owners_of_parent",
    ),
    (
        "EquityAttributableToOwnersOfParent",
        "equity_attributable_to_owners_of_parent",
    ),
    ("NoncontrollingInterests", "noncontrolling_interests"),
    (
        "StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest",
        "total_equity",
    ),
    ("Equity", "total_equity"),
    ("CapitalStock", "capital_stock"),
    ("IssuedCapital", "capital_stock"),
    ("Liabilities", "total_liabilities"),
    ("PropertyPlantAndEquipment", "tangible_fixed_assets"),
    ("TangibleFixedAssets", "tangible_fixed_assets"),
    ("IntangibleAssets", "intangible_fixed_assets"),
    ("IntangibleFixedAssets", "intangible_fixed_assets"),
    ("InvestmentSecurities", "investment_securities"),
    (
        "CorporateBondsInvestmentSecuritiesAssetsBNK",
        "corporate_bonds_investment_securities_assets",
    ),
    ("LongTermLoansPayable", "long_term_loans"),
    ("LongTermBorrowings", "long_term_debt"),
    ("ShortTermLoansPayable", "short_term_loans"),
    ("ShortTermBorrowings", "short_term_debt"),
    ("BondsPayable", "bonds_payable"),
    ("DebtSecurities", "debt_securities"),
    ("InvestmentsAndOtherAssets", "investments_and_other_assets"),
    ("CapitalSurplus", "capital_surplus"),
    ("SharePremium", "share_premium"),
    ("AdditionalPaidInCapital", "additional_paid_in_capital"),
    ("LegalCapitalSurplus", "legal_capital_surplus"),
    (
        "LiabilitiesAndNetAssets",
        "total_liabilities_and_net_assets",
    ),
    ("EquityAndLiabilities", "total_liabilities_and_net_assets"),
    (
        "CurrentPortionOfLongTermLoansPayable",
        "current_portion_long_term_loans_payable",
    ),
    // ── PL ──
    ("NetSales", "revenue"),
    ("Revenue", "revenue"),
    ("Revenues", "revenue"),
    ("SalesRevenueNet", "revenue"),
    ("OperatingRevenue", "revenue"),
    (
        "OperatingRevenueOILTelecommunications",
        "operating_revenue_oil_telecommunications",
    ),
    (
        "OperatingRevenueIncidentalELC",
        "operating_revenue_incidental_elc",
    ),
    ("CostOfSales", "cost_of_revenue"),
    ("CostOfGoodsSold", "cost_of_revenue"),
    ("CostOfGoodsAndServicesSold", "cost_of_revenue"),
    ("CostOfRevenue", "cost_of_revenue"),
    ("GrossProfit", "gross_profit"),
    ("GrossProfitLoss", "gross_profit"),
    (
        "SellingGeneralAndAdministrativeExpenses",
        "selling_general_admin",
    ),
    (
        "SellingGeneralAndAdministrativeExpense",
        "selling_general_admin",
    ),
    ("ResearchAndDevelopmentExpenses", "research_and_development"),
    ("OperatingIncome", "operating_income"),
    ("OperatingProfit", "operating_income"),
    ("OperatingProfitLoss", "operating_income"),
    ("OperatingIncomeLoss", "operating_income"),
    ("ProfitLossFromOperatingActivities", "operating_income"),
    ("OrdinaryIncome", "ordinary_income"),
    ("OrdinaryIncomeLoss", "ordinary_income"),
    (
        "ProfitLossFromContinuingOperations",
        "profit_loss_from_continuing_operations",
    ),
    (
        "IncomeLossFromContinuingOperations",
        "profit_loss_from_continuing_operations",
    ),
    ("IncomeBeforeIncomeTaxes", "income_before_tax"),
    ("ProfitLossBeforeTax", "income_before_tax"),
    (
        "IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest",
        "income_before_tax_continuing_operations",
    ),
    ("ProfitLoss", "net_income"),
    ("NetIncome", "net_income"),
    ("NetIncomeLoss", "net_income"),
    (
        "ProfitLossAttributableToOwnersOfParent",
        "net_income_parent",
    ),
    (
        "ProfitLossAttributableToNonControllingInterests",
        "net_income_minority",
    ),
    (
        "ProfitLossAttributableToNoncontrollingInterests",
        "net_income_minority",
    ),
    (
        "NetIncomeLossAttributableToNoncontrollingInterest",
        "net_income_minority",
    ),
    (
        "DepreciationAndAmortization",
        "depreciation_and_amortization",
    ),
    (
        "DepreciationAndAmortisationExpense",
        "depreciation_and_amortization",
    ),
    ("Depreciation", "depreciation"),
    ("Amortization", "amortization"),
    ("InterestExpenses", "interest_expenses"),
    ("InterestExpense", "interest_expenses"),
    ("InterestIncome", "interest_income"),
    ("RevenueFromInterest", "interest_income"),
    ("FinanceCosts", "finance_costs"),
    ("FinanceIncome", "finance_income"),
    ("NonOperatingIncome", "non_operating_income"),
    ("NonOperatingExpenses", "non_operating_expenses"),
    ("OtherIncome", "other_income"),
    ("OtherExpenseByNature", "other_expense_by_nature"),
    ("OtherGainsLosses", "other_gains_losses"),
    ("OperatingExpensesINS", "operating_expenses_ins"),
    ("ExtraordinaryIncome", "extraordinary_income"),
    ("ExtraordinaryLoss", "extraordinary_loss"),
    ("IncomeTaxes", "income_taxes"),
    (
        "IncomeTaxExpenseContinuingOperations",
        "income_taxes_continuing_operations",
    ),
    ("IncomeTaxesCurrent", "income_taxes_current"),
    ("IncomeTaxesDeferred", "income_taxes_deferred"),
    ("ImpairmentLoss", "impairment_loss"),
    ("OtherNOI", "other_non_operating_income"),
    ("OtherNOE", "other_non_operating_expenses"),
    ("OtherSGA", "other_selling_general_admin"),
    ("OtherEL", "other_extraordinary_loss"),
    ("OtherEI", "other_extraordinary_income"),
    ("OtherRevOA", "other_operating_revenue"),
    ("OtherCOS", "other_cost_of_sales"),
    (
        "OtherCOSExpOA",
        "other_cost_of_sales_expenses_operating_activities",
    ),
    // ── OCI (IFRS / J-GAAP) ──
    ("OtherComprehensiveIncome", "other_comprehensive_income"),
    (
        "GainsLossesOnRemeasuringRecognisedNetDefinedBenefitPlansNotYetRecognised",
        "remeasurement_of_defined_benefit_plans",
    ),
    (
        "GainsLossesOnExchangeDifferencesOnTranslation",
        "foreign_currency_translation_adjustment",
    ),
    // ── CF ──
    ("NetCashProvidedByUsedInOperatingActivities", "operating_cf"),
    ("CashFlowsFromUsedInOperatingActivities", "operating_cf"),
    (
        "NetCashProvidedByUsedInInvestmentActivities",
        "investing_cf",
    ),
    ("NetCashProvidedByUsedInInvestingActivities", "investing_cf"),
    ("CashFlowsFromUsedInInvestingActivities", "investing_cf"),
    ("CashFlowsFromUsedInInvestmentActivities", "investing_cf"),
    ("NetCashProvidedByUsedInFinancingActivities", "financing_cf"),
    ("CashFlowsFromUsedInFinancingActivities", "financing_cf"),
    ("PurchaseOfPropertyPlantAndEquipment", "capex"),
    (
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
        "capex",
    ),
    ("PurchaseOfPropertyPlantAndEquipmentInvCF", "capex"),
    ("PurchaseOfIntangibleAssets", "intangible_asset_purchases"),
    (
        "PurchaseOfIntangibleAssetsClassifiedAsInvestingActivities",
        "intangible_asset_purchases",
    ),
    (
        "PurchaseOfIntangibleAssetsInvCF",
        "intangible_asset_purchases",
    ),
    ("ProceedsFromSalesOfPropertyPlantAndEquipment", "ppe_sold"),
    (
        "ProceedsFromSalesOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
        "ppe_sold",
    ),
    (
        "ProceedsFromSalesOfPropertyPlantAndEquipmentInvCF",
        "ppe_sold",
    ),
    (
        "ProceedsFromSalesOfInvestmentSecurities",
        "investment_securities_sold",
    ),
    (
        "ProceedsFromSalesOfInvestmentSecuritiesInvCF",
        "investment_securities_sold",
    ),
    (
        "PurchaseOfInvestmentSecurities",
        "investment_securities_purchased",
    ),
    (
        "PurchaseOfInvestmentSecuritiesInvCF",
        "investment_securities_purchased",
    ),
    ("CashDividendsPaid", "cash_dividends_paid"),
    ("CashDividendsPaidFinCF", "cash_dividends_paid_financing_cf"),
    ("DividendsPaid", "cash_dividends_paid"),
    ("DividendsPaidFinCF", "dividends_paid_financing_cf"),
    (
        "DividendsPaidClassifiedAsFinancingActivities",
        "dividends_paid_financing_cf",
    ),
    ("DividendsPaidInvCF", "dividends_paid_investing_cf"),
    ("DividendsPaidFromSurplus", "dividends_paid_from_surplus"),
    ("PurchaseOfTreasuryStock", "treasury_stock_purchase"),
    ("PurchaseOfTreasuryStockFinCF", "treasury_stock_purchase"),
    ("AcquisitionOfTreasuryStock", "treasury_stock_purchase"),
    ("DecreaseIncreaseInTreasuryStock", "treasury_stock_change"),
    (
        "DecreaseIncreaseInTreasuryStockFinCF",
        "treasury_stock_change_financing_cf",
    ),
    ("RepaymentOfLongTermLoansPayable", "lt_debt_repayment"),
    ("RepaymentOfLongTermLoansPayableFinCF", "lt_debt_repayment"),
    ("ProceedsFromLongTermLoansPayable", "lt_debt_proceeds"),
    ("ProceedsFromLongTermLoansPayableFinCF", "lt_debt_proceeds"),
    ("DepreciationOpeCF", "depreciation_cf_adjustment"),
    ("AmortizationOpeCF", "amortization_cf_adjustment"),
    (
        "DepreciationAndAmortizationOpeCF",
        "depreciation_and_amortization_cf_adjustment",
    ),
    (
        "IntangibleAssetsOpeCF",
        "intangible_assets_amortization_cf_adjustment",
    ),
    ("ExtraordinaryLossOpeCF", "extraordinary_loss_cf_adjustment"),
    ("ImpairmentLossOpeCF", "impairment_loss_cf_adjustment"),
    (
        "IncomeTaxesDeferredOpeCF",
        "income_taxes_deferred_cf_adjustment",
    ),
    (
        "IncomeBeforeIncomeTaxesOpeCF",
        "income_before_tax_operating_cf",
    ),
    ("OperatingIncomeOpeCF", "operating_income_operating_cf"),
    ("InterestExpensesFinCF", "interest_expenses_financing_cf"),
    ("InterestExpensesOpeCF", "interest_expenses_operating_cf"),
    ("InterestIncomeOpeCF", "interest_income_operating_cf"),
    (
        "CurrentPortionOfLongTermLoansPayableOpeCF",
        "current_portion_long_term_loans_payable_operating_cf",
    ),
    ("CommissionFeeFinCF", "commission_fee_financing_cf"),
    ("CommissionFeeCOSExpOA", "commission_fee_cost"),
    ("CommissionFeeEI", "commission_fee_extraordinary_income"),
    ("CommissionFeeNOI", "commission_fee_non_operating_income"),
    ("BondsAndBorrowingsCLIFRS", "short_term_debt"),
    ("BondsAndBorrowingsNCLIFRS", "long_term_debt"),
    ("BondsAndBorrowingsLiabilitiesIFRS", "bonds_and_borrowings"),
    // ── forecast / dividend / per-share ──
    (
        "ForecastNetSalesSummaryOfBusinessResults",
        "forecast_revenue",
    ),
    (
        "ForecastOperatingIncomeSummaryOfBusinessResults",
        "forecast_operating_income",
    ),
    (
        "ForecastOrdinaryIncomeSummaryOfBusinessResults",
        "forecast_ordinary_income",
    ),
    (
        "ForecastNetIncomeSummaryOfBusinessResults",
        "forecast_net_income",
    ),
    ("DividendPerShare", "dividends_per_share"),
    (
        "DividendPaidPerShareSummaryOfBusinessResults",
        "dividends_per_share_summary",
    ),
    (
        "InterimDividendPaidPerShareSummaryOfBusinessResults",
        "interim_dividends_per_share_summary",
    ),
    (
        "NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc",
        "shares_outstanding",
    ),
    // ── misc retained aliases ──
    ("DividendsFromSurplus", "dividends_from_surplus"),
    (
        "ProvisionForDirectorsBonuses",
        "provision_for_directors_bonuses",
    ),
    (
        "ProvisionForDirectorsRetirementBenefits",
        "provision_for_directors_retirement_benefits",
    ),
];

/// Suffixes to strip when attempting stem normalization.
/// Ordered longest-first within each group so that compound suffixes are tried
/// before their shorter components.
const SUFFIXES: &[&str] = &[
    // Compound suffixes (try before simpler ones)
    "IFRSSummaryOfBusinessResults",
    "USGAAPSummaryOfBusinessResults",
    "JMISSummaryOfBusinessResults",
    "ContinuingOperationsIFRS",
    "SummaryOfBusinessResults",
    "SummaryOfBusinessResult",
    "Abstract",
    // Accounting-standard suffixes. JMIS (banking-industry standard) is
    // recognized; JMISS is retained for older taxonomy data.
    "IFRS",
    "USGAAP",
    "JMIS",
    "JMISS",
    // Industry codes (BNK/INS/SEC/ELC/...) are intentionally NOT stripped
    // here: an industry suffix can change the presented line item or accounting
    // meaning, so each industry-tagged alias must be reviewed and added
    // explicitly to taxonomy.json field_map (or STEM_MAP) rather than
    // blanket-collapsed onto the generic concept.
];

/// Compiled lookup tables for namespace-aware concept canonicalization.
struct TaxonomyMaps {
    /// Full expanded-name overrides: (namespace_uri, local_name) → canonical.
    /// Matched first, for any namespace (even unrecognized). Lets a specific
    /// QName pin a mapping that differs from the family/local default.
    exact: HashMap<ConceptKey, CanonicalConcept>,
    /// Reviewed mappings scoped to a recognized taxonomy family:
    /// (family, local_name) → canonical. Built from taxonomy.json field_map,
    /// which keys entries by `prefix:local`; the prefix selects the family.
    family: HashMap<(TaxonomyFamily, String), CanonicalConcept>,
    /// Reviewed cross-family local-name aliases → canonical. Built from
    /// STEM_MAP. Valid for any recognized family (the namespace gate is
    /// enforced by the caller before this table is consulted).
    safe_local: HashMap<String, CanonicalConcept>,
}

/// Classify a concept namespace URI into a recognized taxonomy family.
///
/// Returns `None` for unrecognized namespaces — typically company-specific
/// extensions. Canonicalization deliberately does NOT fall back to local-name
/// matching for unrecognized namespaces, so a company extension that happens to
/// reuse a standard local name (e.g. `Revenue`) is classified as an extension
/// boundary rather than silently collapsed onto the standard concept.
pub fn classify_namespace_uri(uri: &str) -> Option<TaxonomyFamily> {
    use TaxonomyFamily::*;
    if uri.contains("/taxonomy/jppfs/") {
        Some(EdinetJppfs)
    } else if uri.contains("/taxonomy/jpigp/") {
        Some(EdinetJpigp)
    } else if uri.contains("/taxonomy/jpcrp/") {
        Some(EdinetJpcrp)
    } else if uri.contains("/taxonomy/jpdei/") {
        Some(EdinetJpDei)
    } else if uri.contains("fasb.org/us-gaap") {
        Some(UsGaap)
    } else if uri.contains("ifrs.org") || uri.contains("xbrl.ifrs") {
        Some(Ifrs)
    } else {
        None
    }
}

/// Map a taxonomy.json field_map prefix (e.g. `jppfs_cor`, `jpcrp-esr_cor`) to
/// the taxonomy family its namespace URI belongs to. Returns `None` when the
/// prefix's taxonomy token is not a recognized EDINET family. Such entries
/// must be added explicitly instead of being promoted to a namespace-agnostic
/// alias, because an unknown prefix can be an issuer extension.
fn classify_field_map_prefix(prefix: &str) -> Option<TaxonomyFamily> {
    use TaxonomyFamily::*;
    // The taxonomy token is the leading segment before the first `_` or `-`
    // (e.g. `jppfs_cor` → `jppfs`, `jpcrp-esr_cor` → `jpcrp`).
    let token = prefix.split(['_', '-']).next().unwrap_or(prefix);
    match token {
        "jppfs" | "jplvh" | "jpsps" | "jpctl" => Some(EdinetJppfs),
        "jpigp" => Some(EdinetJpigp),
        "jpcrp" => Some(EdinetJpcrp),
        "jpdei" => Some(EdinetJpDei),
        _ => None,
    }
}

/// Build the namespace-aware lookup tables from taxonomy.json + STEM_MAP.
///
/// Unlike the old `rsplit_once(':')` builder, this preserves the family each
/// curated mapping belongs to, so the same local name under two namespaces is
/// not silently collapsed. Identity concepts are excluded here — they are
/// resolved by a separate identity pass.
fn build_taxonomy_maps() -> TaxonomyMaps {
    let taxonomy: Value =
        serde_json::from_str(TAXONOMY_JSON).expect("embedded taxonomy.json must be valid JSON");

    let mut family: HashMap<(TaxonomyFamily, String), CanonicalConcept> = HashMap::new();
    let mut safe_local: HashMap<String, CanonicalConcept> = HashMap::new();
    let mut exact: HashMap<ConceptKey, CanonicalConcept> = HashMap::new();

    if let Some(field_map) = taxonomy.get("field_map").and_then(Value::as_object) {
        for (full_key, entry) in field_map {
            if entry.get("category").and_then(Value::as_str) == Some("dei") {
                continue;
            }
            let (prefix, local_name) = match full_key.split_once(':') {
                Some((p, l)) => (p, l),
                None => (full_key.as_str(), full_key.as_str()),
            };
            if is_non_scalar_concept(local_name) {
                continue;
            }
            let Some(field_name) = entry.get("field").and_then(Value::as_str) else {
                continue;
            };
            if field_name.is_empty() {
                continue;
            }
            let concept = CanonicalConcept {
                item_name: field_name.to_string(),
                priority: concept_priority(local_name),
                match_kind: MatchKind::TaxonomyFamilyAlias,
            };
            let family_kind = classify_field_map_prefix(prefix).unwrap_or_else(|| {
                panic!(
                    concat!(
                        "taxonomy.json field_map entry {:?} uses unclassified prefix {:?}; ",
                        "classify the prefix explicitly or use qname_overrides"
                    ),
                    full_key, prefix
                )
            });
            family.insert((family_kind, local_name.to_string()), concept);
        }
    }

    // Full-QName exact overrides (namespace_uri + local_name).
    if let Some(overrides) = taxonomy.get("qname_overrides").and_then(Value::as_array) {
        for entry in overrides {
            let Some(ns) = entry.get("namespace_uri").and_then(Value::as_str) else {
                continue;
            };
            let Some(local) = entry.get("local_name").and_then(Value::as_str) else {
                continue;
            };
            let Some(field_name) = entry.get("field").and_then(Value::as_str) else {
                continue;
            };
            if field_name.is_empty() {
                continue;
            }
            exact.insert(
                (ns.to_string(), local.to_string()),
                CanonicalConcept {
                    item_name: field_name.to_string(),
                    priority: concept_priority(local),
                    match_kind: MatchKind::ExactExpandedName,
                },
            );
        }
    }

    // Cross-standard aliases (no namespace). These are safe under any
    // recognized family because the namespace gate is enforced at lookup.
    for (local_name, field_name) in STEM_MAP {
        safe_local.insert(
            (*local_name).to_string(),
            CanonicalConcept {
                item_name: (*field_name).to_string(),
                priority: concept_priority(local_name),
                match_kind: MatchKind::SafeLocalAlias,
            },
        );
    }

    TaxonomyMaps {
        exact,
        family,
        safe_local,
    }
}

/// Lazily compiled taxonomy lookup tables.
fn taxonomy_maps() -> &'static TaxonomyMaps {
    static MAPS: OnceLock<TaxonomyMaps> = OnceLock::new();
    MAPS.get_or_init(build_taxonomy_maps)
}

#[derive(Debug, Clone)]
pub struct CanonicalConcept {
    pub item_name: String,
    pub priority: u8,
    pub match_kind: MatchKind,
}

#[cfg(test)]
static FIELD_METADATA: OnceLock<serde_json::Map<String, Value>> = OnceLock::new();
static IDENTITY_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
static EXCLUDED_CONCEPTS: OnceLock<HashSet<String>> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum TaxonomyConceptStatus {
    /// Reviewed semantic mapping exists (exact QName, family-scoped, or safe
    /// local alias) for a recognized taxonomy namespace.
    Semantic(CanonicalConcept),
    /// Known identity concept with no reviewed mapping. Carries a snake_case
    /// candidate name for future alias review, but is NOT a confirmed semantic
    /// canonical mapping.
    Identity(CanonicalConcept),
    /// Company-specific extension namespace. This is an explicit classification
    /// boundary, not a local-name fallback: the tag remains non-canonical unless
    /// a full QName override is reviewed and added.
    Extension,
    /// Explicitly excluded concept (reviewed decision not to map).
    Excluded,
    /// Unrecognized namespace or unmapped concept.
    Unknown,
}

/// Classify a concept (namespace URI + local name) by mapping confidence.
///
/// Order matters: a reviewed semantic mapping wins (`Semantic`); then an
/// explicitly excluded concept (`Excluded`); then company-extension namespaces
/// (`Extension`); then a known-but-unmapped identity concept (`Identity`);
/// anything else is `Unknown`. `Excluded` is checked
/// before identity so a concept present in both the exclude list and the
/// identity set is treated as excluded.
///
/// Namespace gate: a concept whose namespace URI is not a recognized taxonomy
/// family is classified as `Extension`. No local-name fallback is applied to
/// company-extension namespaces.
pub fn concept_status(concept: &ConceptKey) -> TaxonomyConceptStatus {
    if let Some(canonical) = canonicalize_semantic_concept(concept) {
        return TaxonomyConceptStatus::Semantic(canonical);
    }
    // Excluded/identity only apply to recognized namespaces. A company
    // extension that reuses a standard local name is Extension, not Semantic,
    // Identity, or Excluded.
    if classify_namespace_uri(&concept.0).is_none() {
        return TaxonomyConceptStatus::Extension;
    }
    if is_excluded_concept(&concept.1) {
        return TaxonomyConceptStatus::Excluded;
    }
    if let Some(candidate) = canonicalize_identity_concept(concept) {
        return TaxonomyConceptStatus::Identity(candidate);
    }
    TaxonomyConceptStatus::Unknown
}

pub fn is_excluded_concept(local_name: &str) -> bool {
    excluded_concepts().contains(local_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    use StatementCategory::*;

    match classify_item_name(item_name)? {
        BalanceSheet | ShareData => Some("bs"),
        ProfitAndLoss => Some("pl"),
        CashFlow => Some("cf"),
        ChangesInEquity => Some("changes_in_equity"),
        OtherComprehensiveIncome => Some("oci"),
        PerShare => Some("dividend"),
        Forecast => Some("forecast"),
        Disclosure => Some("disclosure"),
        Other => None,
    }
}

#[cfg(test)]
pub fn statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    canonicalize_concept(concept)
        .and_then(|canonical| statement_for_item_name(&canonical.item_name))
}

#[cfg(test)]
pub fn raw_statement_for_concept(concept: &ConceptKey) -> Option<&'static str> {
    match statement_for_concept(concept)? {
        "bs" => Some("bs"),
        "pl" => Some("pl"),
        "cf" => Some("cf"),
        _ => None,
    }
}

fn excluded_concepts() -> &'static HashSet<String> {
    EXCLUDED_CONCEPTS.get_or_init(|| {
        let taxonomy: Value =
            serde_json::from_str(TAXONOMY_JSON).expect("embedded taxonomy.json must be valid JSON");
        let mut result: HashSet<String> = HashSet::new();
        let Some(groups) = taxonomy.get("excluded_concepts").and_then(Value::as_object) else {
            return result;
        };
        for concepts in groups.values().filter_map(Value::as_array) {
            for concept in concepts {
                if let Some(local_name) = concept.as_str() {
                    result.insert(local_name.to_string());
                }
            }
        }
        result
    })
}

/// Suffixes used for identity stem normalization (Layer 2.5).
///
/// Same as `SUFFIXES`: accounting standard and industry suffixes are normalized,
/// but statement role and balance-sheet position suffixes are preserved.
const IDENTITY_SUFFIXES: &[&str] = &[
    // Compound suffixes (try before simpler ones)
    "IFRSSummaryOfBusinessResults",
    "USGAAPSummaryOfBusinessResults",
    "JMISSummaryOfBusinessResults",
    "ContinuingOperationsIFRS",
    "SummaryOfBusinessResults",
    "SummaryOfBusinessResult",
    "Abstract",
    // Accounting-standard suffixes. JMIS (banking-industry standard) is
    // recognized; JMISS is retained for older taxonomy data.
    "IFRS",
    "USGAAP",
    "JMIS",
    "JMISS",
    // Industry codes (BNK/INS/SEC/ELC/...) are intentionally NOT stripped
    // here: an industry suffix can change the presented line item or accounting
    // meaning, so each industry-tagged alias must be reviewed and added
    // explicitly to taxonomy.json field_map (or STEM_MAP) rather than
    // blanket-collapsed onto the generic concept.
];

/// Pre-loaded set of identity concept local names for fast lookup.
fn identity_set() -> &'static HashSet<&'static str> {
    IDENTITY_SET.get_or_init(|| {
        IDENTITY_CONCEPTS
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    })
}

/// Convert a CamelCase concept stem to snake_case.
///
/// Consecutive uppercase letters are treated as a single acronym:
/// `IFRS` → `ifrs`, `NetSales` → `net_sales`.
/// An underscore is inserted before an uppercase letter when:
/// - it follows a lowercase letter, or
/// - it follows a digit, or
/// - it follows an uppercase letter AND the next letter is lowercase.
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 8);
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
                if prev.is_lowercase()
                    || prev.is_ascii_digit()
                    || (prev.is_uppercase() && next_is_lower)
                {
                    result.push('_');
                }
            }
            for c in ch.to_lowercase() {
                result.push(c);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Resolve a concept's reviewed semantic mapping (exact QName, family-scoped,
/// or safe local alias). Identity concepts are NOT considered here.
fn resolve_semantic(concept: &ConceptKey, maps: &TaxonomyMaps) -> Option<CanonicalConcept> {
    if is_non_scalar_concept(&concept.1) {
        return None;
    }
    // Layer 1: full expanded-name override (any namespace).
    if let Some(entry) = maps.exact.get(concept) {
        return Some(entry.clone());
    }
    // Layers 2-3 require a recognized taxonomy family.
    let family = classify_namespace_uri(&concept.0)?;
    // Some identically named concepts are not accounting-equivalent across
    // standards. Keep them on the identity path unless a full expanded-name
    // override explicitly supplies a reviewed mapping.
    if must_remain_identity_concept(family, &concept.1) {
        return None;
    }
    // Layer 2: family-scoped reviewed mapping (exact then suffix-stem).
    if let Some(entry) = family_lookup(&concept.1, family, maps) {
        return Some(entry);
    }
    // Layer 3: cross-family safe local alias (exact then suffix-stem).
    safe_local_lookup(&concept.1, maps)
}

/// Concepts whose local names look portable but whose accounting scope differs
/// by standard. These must not use family or cross-family semantic aliases.
fn must_remain_identity_concept(family: TaxonomyFamily, local_name: &str) -> bool {
    if !matches!(
        family,
        TaxonomyFamily::Ifrs | TaxonomyFamily::EdinetJpigp | TaxonomyFamily::UsGaap
    ) {
        return false;
    }

    // Under IFRS, `IntangibleAssets` excludes goodwill when goodwill is
    // presented separately. Under J-GAAP, goodwill is included in the
    // intangible fixed assets section. For US-GAAP, this local-name-only
    // fallback is also too broad: without a reviewed expanded-name or family
    // mapping, it cannot prove equivalence to the J-GAAP fixed-asset bucket.
    // Merging these into `intangible_fixed_assets` changes the measurement
    // scope.
    local_name == "IntangibleAssets" || local_name.strip_suffix("IFRS") == Some("IntangibleAssets")
}

/// Family-scoped lookup: exact match, then suffix-stripped stem against the
/// family map. Summary items are never stem-normalized (they are condensed
/// duplicates that can conflict with real statement items).
fn family_lookup(
    local_name: &str,
    family: TaxonomyFamily,
    maps: &TaxonomyMaps,
) -> Option<CanonicalConcept> {
    if let Some(entry) = maps.family.get(&(family, local_name.to_string())) {
        return Some(entry.clone());
    }
    if local_name.contains("SummaryOfBusinessResults")
        || local_name.contains("SummaryOfBusinessResult")
    {
        return None;
    }
    for suffix in SUFFIXES {
        if let Some(stem) = local_name.strip_suffix(suffix) {
            if stem.is_empty() {
                continue;
            }
            if let Some(entry) = maps.family.get(&(family, stem.to_string())) {
                return Some(CanonicalConcept {
                    item_name: entry.item_name.clone(),
                    priority: concept_priority(local_name),
                    match_kind: MatchKind::TaxonomyFamilyAlias,
                });
            }
        }
    }
    None
}

/// Cross-family safe-local lookup: exact match, then suffix-stripped stem
/// against the safe_local map. Summary items are never stem-normalized.
fn safe_local_lookup(local_name: &str, maps: &TaxonomyMaps) -> Option<CanonicalConcept> {
    if let Some(entry) = maps.safe_local.get(local_name) {
        return Some(entry.clone());
    }
    if local_name.contains("SummaryOfBusinessResults")
        || local_name.contains("SummaryOfBusinessResult")
    {
        return None;
    }
    for suffix in SUFFIXES {
        if let Some(stem) = local_name.strip_suffix(suffix) {
            if stem.is_empty() {
                continue;
            }
            if let Some(entry) = maps.safe_local.get(stem) {
                return Some(CanonicalConcept {
                    item_name: entry.item_name.clone(),
                    priority: concept_priority(local_name),
                    match_kind: MatchKind::SafeLocalAlias,
                });
            }
        }
    }
    None
}

/// Reviewed semantic mapping only (Layers 1-3): exact QName override,
/// family-scoped mappings, and cross-family safe aliases (taxonomy.json
/// field_map + STEM_MAP). Identity concepts are NOT considered canonical here.
/// Use `canonicalize_concept` for the full pipeline that also accepts identity
/// snake_case candidate names.
pub fn canonicalize_semantic_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    resolve_semantic(concept, taxonomy_maps())
}

/// Identity-layer mapping only (Layer 2.5 + Layer 3): produces snake_case
/// candidate names for known concepts that have no reviewed semantic mapping.
/// Only resolves concepts in recognized taxonomy namespaces.
fn canonicalize_identity_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if is_non_scalar_concept(&concept.1) {
        return None;
    }
    if classify_namespace_uri(&concept.0).is_none() {
        return None;
    }
    // Layer 2.5: identity stem normalization (intentionally skips Summary items)
    if let Some(entry) = canonicalize_identity_stem(&concept.1) {
        return Some(entry);
    }
    // Layer 3: exact-match identity concept -> snake_case candidate.
    // Catches Summary items that Layer 2.5 deliberately skips.
    identity_exact(&concept.1)
}

/// Layer 3 identity: a known identity concept local name converted verbatim to
/// snake_case. Caller is responsible for the non-scalar / family gate.
fn identity_exact(local_name: &str) -> Option<CanonicalConcept> {
    if identity_set().contains(local_name) {
        Some(CanonicalConcept {
            item_name: to_snake_case(local_name),
            priority: concept_priority(local_name),
            match_kind: MatchKind::Identity,
        })
    } else {
        None
    }
}

/// Canonicalize a concept (namespace URI + local name) using the full pipeline
/// (semantic OR identity).
///
/// 1. **Semantic mapping** — `resolve_semantic` (reviewed aliases)
/// 2. **Identity mapping** — `canonicalize_identity_concept` (snake_case names)
///
/// Prefer `canonicalize_semantic_concept` when only confirmed canonical names
/// are acceptable (canonical statements, `concept_status` Semantic branch).
pub fn canonicalize_concept(concept: &ConceptKey) -> Option<CanonicalConcept> {
    canonicalize_semantic_concept(concept).or_else(|| canonicalize_identity_concept(concept))
}

/// Try to canonicalize by stripping identity-safe suffixes and looking up
/// the stem in the identity concept set.  CF role suffixes are intentionally
/// preserved because they carry statement context.
///
/// BS position codes (`CA`, `NCA`, `CL`, `NCL`) are intentionally excluded
/// from stripping because they distinguish assets from liabilities.
///
/// Only processes concepts that are known to exist in XBRL data (i.e., are
/// present in the identity concept set).  The stem does NOT need to be in
/// the identity set — it just needs to be a non-empty string after stripping.
/// Concepts without any matching suffix are still converted to snake_case.
fn canonicalize_identity_stem(local_name: &str) -> Option<CanonicalConcept> {
    if local_name.contains("SummaryOfBusinessResults")
        || local_name.contains("SummaryOfBusinessResult")
    {
        return None;
    }
    // Only normalize known identity concepts
    if !identity_set().contains(local_name) {
        return None;
    }
    // Try stripping identity suffixes first
    for suffix in IDENTITY_SUFFIXES {
        if let Some(stem) = local_name.strip_suffix(suffix) {
            if !stem.is_empty() {
                return Some(CanonicalConcept {
                    item_name: to_snake_case(stem),
                    priority: concept_priority(local_name),
                    match_kind: MatchKind::Identity,
                });
            }
        }
    }
    // No suffix stripped; convert the concept itself to snake_case
    Some(CanonicalConcept {
        item_name: to_snake_case(local_name),
        priority: concept_priority(local_name),
        match_kind: MatchKind::Identity,
    })
}

pub fn canonicalize_synthetic(item_name: &str) -> Option<CanonicalConcept> {
    match item_name {
        "inventories" | "has_preferred_shares" => Some(CanonicalConcept {
            item_name: item_name.to_string(),
            priority: 255,
            match_kind: MatchKind::SafeLocalAlias,
        }),
        _ => None,
    }
}

/// Local-name -> canonical field-name map for taxonomy invariants.
#[cfg(test)]
fn field_name_map() -> HashMap<String, String> {
    let maps = taxonomy_maps();
    let mut result: HashMap<String, String> = HashMap::new();
    for ((_family, local_name), entry) in &maps.family {
        result.insert(local_name.clone(), entry.item_name.clone());
    }
    for (local_name, entry) in &maps.safe_local {
        result
            .entry(local_name.clone())
            .or_insert_with(|| entry.item_name.clone());
    }
    for local_name in IDENTITY_CONCEPTS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_non_scalar_concept(line))
    {
        result
            .entry(local_name.to_string())
            .or_insert_with(|| to_snake_case(local_name));
    }
    result
}

#[cfg(test)]
fn field_metadata() -> &'static serde_json::Map<String, Value> {
    FIELD_METADATA.get_or_init(|| {
        let taxonomy: Value =
            serde_json::from_str(TAXONOMY_JSON).expect("embedded taxonomy.json must be valid JSON");
        taxonomy
            .get("canonical_fields")
            .and_then(Value::as_object)
            .expect("taxonomy.json must define canonical_fields")
            .clone()
    })
}

fn is_non_scalar_concept(local_name: &str) -> bool {
    local_name.contains("TextBlock")
        || local_name.ends_with("Abstract")
        || local_name.ends_with("Table")
        || local_name.ends_with("LineItems")
        || local_name.ends_with("Axis")
        || local_name.ends_with("Domain")
        || local_name.ends_with("Member")
        || local_name.ends_with("Heading")
}

fn concept_priority(local_name: &str) -> u8 {
    // Summary items (condensed business results) are less authoritative than
    // actual statement items and must not override them.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a ConceptKey under the EDINET JPPFS namespace. Used for STEM_MAP /
    /// cross-standard alias tests where the family only needs to be recognized.
    fn q(local: &str) -> ConceptKey {
        (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
            local.to_string(),
        )
    }

    fn ifrs(local: &str) -> ConceptKey {
        (
            "https://xbrl.ifrs.org/taxonomy/2024-03-27/ifrs-full".to_string(),
            local.to_string(),
        )
    }

    fn jpigp(local: &str) -> ConceptKey {
        (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2020-11-01/jpigp_cor".to_string(),
            local.to_string(),
        )
    }

    fn us_gaap(local: &str) -> ConceptKey {
        (
            "https://fasb.org/us-gaap/2024".to_string(),
            local.to_string(),
        )
    }

    /// Build the namespace URI that matches a taxonomy.json field_map prefix's
    /// family, so family-scoped lookups resolve in the per-entry test.
    fn uri_for_prefix(prefix: &str) -> String {
        let token = prefix.split(['_', '-']).next().unwrap_or(prefix);
        let core = match token {
            "jppfs" | "jplvh" | "jpsps" | "jpctl" => "jppfs",
            "jpigp" => "jpigp",
            "jpcrp" => "jpcrp",
            "jpdei" => "jpdei",
            other => panic!("unclassified taxonomy.json field_map prefix: {other}"),
        };
        format!("http://disclosure.edinet-fsa.go.jp/taxonomy/{core}/2020-11-01/{prefix}")
    }

    #[test]
    fn canonicalizes_taxonomy_aliases() {
        assert_eq!(
            canonicalize_concept(&q("NetSales")).map(|entry| entry.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CurrentAssetsIFRS")).map(|entry| entry.item_name),
            Some("current_assets".to_string())
        );
    }

    #[test]
    fn taxonomy_json_metadata_matches_accounting_standards() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();
        let reverse_map = taxonomy
            .get("reverse_map")
            .and_then(Value::as_object)
            .unwrap();

        let owners_parent = field_map
            .get("jppfs_cor:ProfitLossAttributableToOwnersOfParent")
            .unwrap();
        assert_eq!(
            owners_parent.get("field").and_then(Value::as_str),
            Some("net_income_parent")
        );
        assert_eq!(
            owners_parent.get("category").and_then(Value::as_str),
            Some("financial_statement")
        );
        assert_eq!(
            owners_parent.get("standard").and_then(Value::as_str),
            Some("jp_gaap")
        );

        let dividend = field_map
            .get("jpcrp_cor:DividendPerShareDividendsOfSurplus")
            .unwrap();
        assert_eq!(
            dividend.get("category").and_then(Value::as_str),
            Some("per_share")
        );
        assert_eq!(
            dividend.get("standard").and_then(Value::as_str),
            Some("jp_gaap")
        );

        let parent_reverse_entries = reverse_map
            .get("net_income_parent")
            .and_then(Value::as_array)
            .unwrap();
        assert!(parent_reverse_entries.iter().any(|entry| {
            entry.get("element_id").and_then(Value::as_str)
                == Some("jppfs_cor:ProfitLossAttributableToOwnersOfParent")
        }));
        let net_income_entries = reverse_map
            .get("net_income")
            .and_then(Value::as_array)
            .unwrap();
        assert!(!net_income_entries.iter().any(|entry| {
            entry.get("element_id").and_then(Value::as_str)
                == Some("jppfs_cor:ProfitLossAttributableToOwnersOfParent")
        }));
    }

    #[test]
    fn industry_top_line_income_tags_are_intentionally_revenue() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();
        let revenue = taxonomy
            .get("canonical_fields")
            .and_then(Value::as_object)
            .and_then(|fields| fields.get("revenue"))
            .and_then(Value::as_object)
            .unwrap();
        let jp_gaap_note = revenue
            .get("standard_notes")
            .and_then(Value::as_object)
            .and_then(|notes| notes.get("jp_gaap"))
            .and_then(Value::as_str)
            .unwrap();
        let description_jp = revenue
            .get("description_jp")
            .and_then(Value::as_str)
            .unwrap();

        assert!(jp_gaap_note.contains("銀行・保険"));
        assert!(description_jp.contains("経常収益"));

        for concept in [
            "jppfs_cor:OrdinaryIncomeBNK",
            "jppfs_cor:OperatingIncomeINS",
        ] {
            let entry = field_map.get(concept).unwrap();
            assert_eq!(entry.get("field").and_then(Value::as_str), Some("revenue"));
            assert_eq!(
                entry.get("category").and_then(Value::as_str),
                Some("financial_statement")
            );
            assert_eq!(
                entry.get("standard").and_then(Value::as_str),
                Some("jp_gaap")
            );
        }
    }

    #[test]
    fn taxonomy_json_field_map_prefixes_are_explicitly_classified() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();

        for full_key in field_map.keys() {
            let prefix = full_key
                .split_once(':')
                .map_or(full_key.as_str(), |(prefix, _)| prefix);
            assert!(
                classify_field_map_prefix(prefix).is_some(),
                "taxonomy.json field_map prefix {prefix:?} must be explicitly classified"
            );
        }
    }

    #[test]
    fn taxonomy_json_metadata_counts_match_embedded_tables() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let meta = taxonomy.get("_meta").and_then(Value::as_object).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();
        let reverse_map = taxonomy
            .get("reverse_map")
            .and_then(Value::as_object)
            .unwrap();
        let canonical_fields = taxonomy
            .get("canonical_fields")
            .and_then(Value::as_object)
            .unwrap();

        assert_eq!(
            meta.get("field_map_count").and_then(Value::as_u64),
            Some(field_map.len() as u64)
        );
        assert_eq!(
            meta.get("reverse_map_count").and_then(Value::as_u64),
            Some(reverse_map.len() as u64)
        );
        assert_eq!(
            meta.get("canonical_fields_count").and_then(Value::as_u64),
            Some(canonical_fields.len() as u64)
        );
    }

    #[test]
    fn taxonomy_json_canonical_fields_cover_reverse_map_and_key_stems() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let reverse_map = taxonomy
            .get("reverse_map")
            .and_then(Value::as_object)
            .unwrap();
        let canonical_fields = taxonomy
            .get("canonical_fields")
            .and_then(Value::as_object)
            .unwrap();

        for field in reverse_map.keys() {
            assert!(
                canonical_fields.contains_key(field),
                "reverse_map field {field} is missing canonical metadata"
            );
        }

        for field in [
            "inventories",
            "current_liabilities",
            "non_current_liabilities",
            "capital_stock",
            "capital_surplus",
            "legal_capital_surplus",
            "cost_of_revenue",
            "gross_profit",
            "income_taxes",
            "dividends_per_share",
        ] {
            assert!(
                canonical_fields.contains_key(field),
                "key STEM_MAP field {field} is missing canonical metadata"
            );
        }
    }

    #[test]
    fn taxonomy_json_canonical_field_entries_have_required_shape() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let canonical_fields = taxonomy
            .get("canonical_fields")
            .and_then(Value::as_object)
            .unwrap();

        for (field, entry) in canonical_fields {
            let object = entry
                .as_object()
                .unwrap_or_else(|| panic!("{field} metadata must be an object"));
            for key in [
                "label_jp",
                "label_en",
                "category",
                "statement",
                "description_jp",
            ] {
                assert!(
                    object.get(key).and_then(Value::as_str).is_some(),
                    "{field}.{key} must be a string"
                );
            }
            for key in ["standards", "sources"] {
                assert!(
                    object
                        .get(key)
                        .and_then(Value::as_array)
                        .is_some_and(|values| !values.is_empty()),
                    "{field}.{key} must be a non-empty array"
                );
            }
            assert!(
                object
                    .get("standard_notes")
                    .and_then(Value::as_object)
                    .is_some_and(|values| !values.is_empty()),
                "{field}.standard_notes must be a non-empty object"
            );
        }
    }

    #[test]
    fn taxonomy_json_captures_jp_gaap_ifrs_inventory_difference() {
        let metadata = field_metadata();
        let inventories = metadata
            .get("inventories")
            .and_then(Value::as_object)
            .unwrap();
        let label_jp = inventories.get("label_jp").and_then(Value::as_str).unwrap();
        let notes = inventories
            .get("standard_notes")
            .and_then(Value::as_object)
            .unwrap();
        let jp_gaap = notes.get("jp_gaap").and_then(Value::as_str).unwrap();
        let ifrs = notes.get("ifrs").and_then(Value::as_str).unwrap();

        assert!(label_jp.contains("事務用消耗品"));
        assert!(jp_gaap.contains("企業会計基準第9号第3項"));
        assert!(jp_gaap.contains("事務用消耗品"));
        assert!(ifrs.contains("IAS 2"));
        assert!(ifrs.contains("材料及び貯蔵品"));
    }

    #[test]
    fn taxonomy_json_captures_companies_act_capital_terms() {
        let metadata = field_metadata();
        for field in ["capital_stock", "capital_surplus", "legal_capital_surplus"] {
            let entry = metadata.get(field).and_then(Value::as_object).unwrap();
            let notes = entry
                .get("standard_notes")
                .and_then(Value::as_object)
                .unwrap();
            let jp_gaap = notes.get("jp_gaap").and_then(Value::as_str).unwrap();
            assert!(
                jp_gaap.contains("会社法445条"),
                "{field} must cite Companies Act article 445 in jp_gaap notes"
            );
        }
    }

    #[test]
    fn preserves_cross_standard_intangible_assets_scope() {
        let jp_gaap = canonicalize_concept(&q("IntangibleAssets")).unwrap();
        assert_eq!(jp_gaap.item_name, "intangible_fixed_assets");

        for concept in [
            ifrs("IntangibleAssets"),
            jpigp("IntangibleAssetsIFRS"),
            us_gaap("IntangibleAssets"),
        ] {
            assert!(canonicalize_semantic_concept(&concept).is_none());
            match concept_status(&concept) {
                TaxonomyConceptStatus::Identity(candidate) => {
                    assert_eq!(candidate.item_name, "intangible_assets");
                    assert_eq!(
                        classify_item_name(&candidate.item_name),
                        Some(StatementCategory::BalanceSheet)
                    );
                }
                status => panic!("expected identity concept, got {status:?}"),
            }
        }
    }

    #[test]
    fn canonicalizes_official_ifrs_aliases() {
        for (concept, expected) in [
            ("IssuedCapital", "capital_stock"),
            ("SharePremium", "share_premium"),
            (
                "EquityAttributableToOwnersOfParent",
                "equity_attributable_to_owners_of_parent",
            ),
            ("NoncontrollingInterests", "noncontrolling_interests"),
            ("EquityAndLiabilities", "total_liabilities_and_net_assets"),
            (
                "TradeAndOtherCurrentReceivables",
                "trade_and_other_current_receivables",
            ),
            (
                "TradeAndOtherCurrentPayables",
                "trade_and_other_current_payables",
            ),
            ("ProfitLossFromOperatingActivities", "operating_income"),
            (
                "ProfitLossFromContinuingOperations",
                "profit_loss_from_continuing_operations",
            ),
            ("RevenueFromInterest", "interest_income"),
            ("FinanceIncome", "finance_income"),
            ("FinanceCosts", "finance_costs"),
            (
                "IncomeTaxExpenseContinuingOperations",
                "income_taxes_continuing_operations",
            ),
            (
                "ProfitLossAttributableToOwnersOfParent",
                "net_income_parent",
            ),
            (
                "ProfitLossAttributableToNoncontrollingInterests",
                "net_income_minority",
            ),
            ("DebtSecurities", "debt_securities"),
        ] {
            assert_eq!(
                canonicalize_concept(&ifrs(concept)).map(|entry| entry.item_name),
                Some(expected.to_string()),
                "{concept} should canonicalize to {expected}"
            );
        }
    }

    #[test]
    fn canonicalizes_official_us_gaap_aliases() {
        for (concept, expected) in [
            ("Revenues", "revenue"),
            ("SalesRevenueNet", "revenue"),
            (
                "AccountsReceivableNetCurrent",
                "accounts_receivable_net_current",
            ),
            ("AccountsPayableCurrent", "accounts_payable_current"),
            ("AccountsPayableTradeCurrent", "accounts_payable_trade"),
            (
                "StockholdersEquity",
                "equity_attributable_to_owners_of_parent",
            ),
            (
                "StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest",
                "total_equity",
            ),
            ("RetainedEarningsAccumulatedDeficit", "retained_earnings"),
            ("AdditionalPaidInCapital", "additional_paid_in_capital"),
            ("OperatingIncomeLoss", "operating_income"),
            (
                "IncomeLossFromContinuingOperations",
                "profit_loss_from_continuing_operations",
            ),
            (
                "NetIncomeLossAttributableToNoncontrollingInterest",
                "net_income_minority",
            ),
            ("InterestExpense", "interest_expenses"),
        ] {
            assert_eq!(
                canonicalize_concept(&us_gaap(concept)).map(|entry| entry.item_name),
                Some(expected.to_string()),
                "{concept} should canonicalize to {expected}"
            );
        }
    }

    #[test]
    fn canonicalizes_long_ifrs_cash_flow_aliases() {
        for (concept, expected) in [
            (
                "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
                "capex",
            ),
            (
                "PurchaseOfIntangibleAssetsClassifiedAsInvestingActivities",
                "intangible_asset_purchases",
            ),
            (
                "ProceedsFromSalesOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
                "ppe_sold",
            ),
            (
                "DividendsPaidClassifiedAsFinancingActivities",
                "dividends_paid_financing_cf",
            ),
        ] {
            assert_eq!(
                canonicalize_concept(&ifrs(concept)).map(|entry| entry.item_name),
                Some(expected.to_string()),
                "{concept} should canonicalize to {expected}"
            );
        }
    }

    #[test]
    fn canonicalizes_supplementary_aliases() {
        assert_eq!(
            canonicalize_concept(&q("PurchaseOfTreasuryStockFinCF")).map(|entry| entry.item_name),
            Some("treasury_stock_purchase".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("ForecastNetIncomeSummaryOfBusinessResults"))
                .map(|entry| entry.item_name),
            Some("forecast_net_income".to_string())
        );
    }

    #[test]
    fn preserves_accounting_scope_in_cross_standard_aliases() {
        for (concept, expected) in [
            ("ShareholdersEquity", "shareholders_equity"),
            (
                "EquityAttributableToOwnersOfParent",
                "equity_attributable_to_owners_of_parent",
            ),
            ("AccountsReceivableTrade", "accounts_receivable_trade"),
            ("AccountsReceivableNet", "accounts_receivable_net"),
            ("AccountsPayableTrade", "accounts_payable_trade"),
            ("AccountsPayableCurrent", "accounts_payable_current"),
            ("DebtSecurities", "debt_securities"),
            ("BondsPayable", "bonds_payable"),
            ("BondsAndBorrowingsLiabilitiesIFRS", "bonds_and_borrowings"),
            ("BondsAndBorrowingsCLIFRS", "short_term_debt"),
            ("BondsAndBorrowingsNCLIFRS", "long_term_debt"),
            ("SharePremium", "share_premium"),
            ("CapitalSurplus", "capital_surplus"),
        ] {
            assert_eq!(
                canonicalize_concept(&q(concept)).map(|entry| entry.item_name),
                Some(expected.to_string()),
                "{concept} must preserve its accounting scope",
            );
        }
    }

    #[test]
    fn preserves_continuing_operations_scope() {
        assert_eq!(
            canonicalize_concept(&q(
                "IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest",
            ))
            .map(|entry| entry.item_name),
            Some("income_before_tax_continuing_operations".to_string()),
        );
        assert_eq!(
            canonicalize_concept(&q("IncomeTaxExpenseContinuingOperations"))
                .map(|entry| entry.item_name),
            Some("income_taxes_continuing_operations".to_string()),
        );
    }

    #[test]
    fn classifies_cash_and_cash_equivalents_as_balance_sheet() {
        assert_eq!(
            statement_for_concept(&q("CashAndCashEquivalents")),
            Some("bs")
        );
    }

    /// Detect asset/liability collapse.  classify_item_name only distinguishes
    /// the broad BS bucket, so a bug that maps an asset concept (DebtSecurities)
    /// to a liability field (bonds_payable) would still pass the BS-vs-PL test.
    /// This guard keeps asset- and liability-side concepts in disjoint canonical
    /// fields and prevents the DebtSecurities/BondsPayable inversion from regressing.
    #[test]
    fn asset_and_liability_concepts_do_not_share_canonical_field() {
        use std::collections::HashSet;
        let assets = [
            "DebtSecurities",
            "AccountsReceivableNet",
            "AccountsReceivableNetCurrent",
        ];
        let liabilities = [
            "BondsPayable",
            "AccountsPayableCurrent",
            "TradeAndOtherCurrentPayables",
        ];
        let asset_fields: HashSet<String> = assets
            .iter()
            .map(|c| canonicalize_concept(&q(c)).unwrap().item_name)
            .collect();
        let liability_fields: HashSet<String> = liabilities
            .iter()
            .map(|c| canonicalize_concept(&q(c)).unwrap().item_name)
            .collect();
        let overlap: Vec<&String> = asset_fields.intersection(&liability_fields).collect();
        assert!(
            overlap.is_empty(),
            "asset and liability concepts must not share a canonical field, but found: {overlap:?}"
        );
        // Explicit guard on the historical asset/liability inversion.
        assert_ne!(
            canonicalize_concept(&q("DebtSecurities")).map(|e| e.item_name),
            canonicalize_concept(&q("BondsPayable")).map(|e| e.item_name),
        );
    }

    #[test]
    fn commercial_paper_assets_are_not_liability_commercial_paper() {
        assert_eq!(
            canonicalize_concept(&q("CommercialPapersLiabilities")).map(|entry| entry.item_name),
            Some("commercial_paper".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CommercialPapersCAFND")).map(|entry| entry.item_name),
            Some("debt_securities".to_string())
        );
        assert_ne!(
            canonicalize_concept(&q("CommercialPapersCAFND"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("CommercialPapersLiabilities"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn preserves_distinct_statement_context_for_known_conflict_aliases() {
        assert_eq!(
            canonicalize_concept(&q("OperatingRevenue1")).map(|entry| entry.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CostOfGoodsSold")).map(|entry| entry.item_name),
            Some("cost_of_revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("BondsAndBorrowingsCLIFRS")).map(|entry| entry.item_name),
            Some("short_term_debt".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("BondsAndBorrowingsNCLIFRS")).map(|entry| entry.item_name),
            Some("long_term_debt".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("DecreaseIncreaseInTreasuryStockFinCF"))
                .map(|entry| entry.item_name),
            Some("treasury_stock_change_financing_cf".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CorporateBondsInvestmentSecuritiesAssetsBNK"))
                .map(|entry| entry.item_name),
            Some("corporate_bonds_investment_securities_assets".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CorporateBondsAssetsINS")).map(|entry| entry.item_name),
            Some("corporate_bonds_investment_securities_assets".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("OperatingExpensesINS")).map(|entry| entry.item_name),
            Some("operating_expenses_ins".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("OperatingRevenueIncidentalELC")).map(|entry| entry.item_name),
            Some("operating_revenue_incidental_elc".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("OperatingRevenueOILTelecommunications"))
                .map(|entry| entry.item_name),
            Some("operating_revenue_oil_telecommunications".to_string())
        );
    }

    #[test]
    fn keeps_registered_identity_concepts_explicit() {
        // Identity concepts are now normalized to snake_case via Layer 2.5
        assert_eq!(
            canonicalize_concept(&q("AccountsPayableOther")).map(|entry| entry.item_name),
            Some("accounts_payable_other".to_string())
        );
    }

    #[test]
    fn rejects_unknown_concepts() {
        assert!(canonicalize_concept(&q("DefinitelyUnknownConceptForStrictTaxonomy")).is_none());
    }

    #[test]
    fn classifies_taxonomy_statuses() {
        assert!(matches!(
            concept_status(&q("AccruedTaxes")),
            TaxonomyConceptStatus::Excluded
        ));
        assert!(matches!(
            concept_status(&q("DefinitelyUnknownConceptForStrictTaxonomy")),
            TaxonomyConceptStatus::Unknown
        ));
        assert!(matches!(
            concept_status(&jpigp("NetRevenueIFRS")),
            TaxonomyConceptStatus::Semantic(canonical) if canonical.item_name == "revenue"
        ));
        assert_eq!(
            raw_statement_for_concept(&q("PurchaseOfTreasuryStockFinCF")),
            Some("cf")
        );
    }

    #[test]
    fn identity_only_concepts_are_identity_not_semantic() {
        // AccountsPayableOther is a known identity concept with no reviewed
        // semantic mapping. It must NOT be reported as Semantic.
        assert!(matches!(
            concept_status(&q("AccountsPayableOther")),
            TaxonomyConceptStatus::Identity(_)
        ));
        // The candidate name is still the snake_case identity name.
        match concept_status(&q("AccountsPayableOther")) {
            TaxonomyConceptStatus::Identity(c) => {
                assert_eq!(c.item_name, "accounts_payable_other");
            }
            _ => panic!("expected Identity"),
        }
        // Summary items that fall through to Layer 3 are also identity.
        assert!(matches!(
            concept_status(&q("TotalAssetsSummaryOfBusinessResults")),
            TaxonomyConceptStatus::Identity(_)
        ));
    }

    #[test]
    fn semantic_canonicalize_ignores_identity_concepts() {
        // Reviewed STEM_MAP mappings resolve via the semantic path.
        assert_eq!(
            canonicalize_semantic_concept(&q("Revenue")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_semantic_concept(&q("PurchaseOfTreasuryStockFinCF")).map(|e| e.item_name),
            Some("treasury_stock_purchase".to_string())
        );
        // Identity-only concepts are NOT canonical via the semantic path.
        assert!(canonicalize_semantic_concept(&q("AccountsPayableOther")).is_none());
        // But the full pipeline still resolves them as candidates.
        assert_eq!(
            canonicalize_concept(&q("AccountsPayableOther")).map(|e| e.item_name),
            Some("accounts_payable_other".to_string())
        );
    }

    #[test]
    fn rejects_non_scalar_taxonomy_concepts() {
        for concept in [
            "CurrentAssetsAbstract",
            "BalanceSheetTable",
            "BalanceSheetLineItems",
            "ClassesOfSharesAxis",
            "ConsolidatedOrNonConsolidatedDomain",
            "NetAssetsMember",
            "EquityIFRSMember",
            "ConsolidatedFinancialStatementIFRSHeading",
            "RevenuesFromExternalCustomersInformationForEachRegionTextBlock",
        ] {
            assert!(
                canonicalize_concept(&q(concept)).is_none(),
                "{concept} must not canonicalize to a scalar financial item"
            );
        }
    }

    #[test]
    fn field_name_map_excludes_non_scalar_concepts() {
        let map = field_name_map();
        for concept in [
            "NetAssetsMember",
            "EquityIFRSMember",
            "RevenuesFromExternalCustomersInformationForEachRegionTextBlock",
        ] {
            assert!(
                !map.contains_key(concept),
                "{concept} must not be exposed in the taxonomy field map"
            );
        }
    }

    #[test]
    fn suffix_strips_ifrs() {
        assert_eq!(
            canonicalize_concept(&q("NetSalesIFRS")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("RevenueIFRS")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn industry_suffixes_are_not_blanket_stripped() {
        // Reviewed industry aliases (taxonomy.json field_map) still resolve,
        // because each is an explicit family-scoped entry rather than a
        // suffix-stripped guess.
        assert_eq!(
            canonicalize_concept(&q("OperatingRevenueFND")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("OrdinaryIncomeBNK")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("CorporateBondsAssetsINS")).map(|e| e.item_name),
            Some("corporate_bonds_investment_securities_assets".to_string())
        );
        // An industry-tagged concept WITHOUT a reviewed explicit mapping must
        // NOT be blanket-collapsed onto the generic concept by stripping the
        // industry code.
        assert_ne!(
            canonicalize_concept(&q("OperatingRevenueBNK")).map(|e| e.item_name),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn role_suffixes_are_preserved_for_context_bearing_items() {
        assert_eq!(
            canonicalize_concept(&q("CostOfSalesCOS")).map(|e| e.item_name),
            Some("cost_of_sales_cos".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("DepreciationNOE")).map(|e| e.item_name),
            Some("depreciation_noe".to_string())
        );
        assert_ne!(
            canonicalize_concept(&q("OperatingIncomeSGA")).map(|e| e.item_name),
            Some("operating_income".to_string())
        );
    }

    #[test]
    fn summary_items_fall_through_to_identity_snake_case() {
        // Summary items are excluded from identity stem normalization (Layer 2.5),
        // so they fall through to Layer 3 exact identity mapping.
        let result =
            canonicalize_concept(&q("TotalAssetsSummaryOfBusinessResults")).map(|e| e.item_name);
        assert_eq!(
            result,
            Some("total_assets_summary_of_business_results".to_string())
        );
        // Real statement items still canonicalize correctly
        assert_eq!(
            canonicalize_concept(&q("Assets")).map(|e| e.item_name),
            Some("total_assets".to_string())
        );
    }

    #[test]
    fn suffix_strips_compound_ifrs_summary() {
        // Summary items are excluded from stem normalization to avoid conflicts
        assert!(
            canonicalize_concept(&q("ProfitLossIFRSSummaryOfBusinessResults")).is_none()
                || canonicalize_concept(&q("ProfitLossIFRSSummaryOfBusinessResults"))
                    .map(|e| e.item_name == "ProfitLossIFRSSummaryOfBusinessResults")
                    .unwrap_or(false)
        );
    }

    #[test]
    fn cf_role_suffix_is_not_stripped_into_pl_or_bs() {
        assert_eq!(
            canonicalize_concept(&q("DepreciationOpeCF")).map(|e| e.item_name),
            Some("depreciation_cf_adjustment".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("PurchaseOfTreasuryStockFinCF")).map(|e| e.item_name),
            Some("treasury_stock_purchase".to_string())
        );
    }

    #[test]
    fn bs_position_suffixes_are_preserved_for_context_bearing_items() {
        assert_eq!(
            canonicalize_concept(&q("AccountsPayableTradeCL")).map(|e| e.item_name),
            Some("accounts_payable_trade_cl".to_string())
        );
    }

    #[test]
    fn suffix_unknown_stem_falls_through_to_identity() {
        // Identity concepts are now normalized to snake_case via Layer 2.5
        let result = canonicalize_concept(&q("AccountsPayableOther"));
        assert_eq!(
            result.map(|e| e.item_name),
            Some("accounts_payable_other".to_string())
        );
    }

    #[test]
    fn suffix_stripping_does_not_produce_false_positives() {
        // "ProfitLoss" itself should still map to net_income, not via stripping
        assert_eq!(
            canonicalize_concept(&q("ProfitLoss")).map(|e| e.item_name),
            Some("net_income".to_string())
        );
    }

    // ── Identity stem normalization (Layer 2.5) tests ──

    #[test]
    fn explicit_cf_role_mapping_preserves_context() {
        assert_eq!(
            canonicalize_concept(&q("CommissionFeeFinCF")).map(|e| e.item_name),
            Some("commission_fee_financing_cf".to_string())
        );
    }

    #[test]
    fn identity_stem_preserves_pl_role() {
        assert_eq!(
            canonicalize_concept(&q("RetirementBenefitExpensesSGA")).map(|e| e.item_name),
            Some("retirement_benefit_expenses_sga".to_string())
        );
    }

    #[test]
    fn identity_stem_strips_ifrs() {
        // OperatingExpensesIFRS → strip IFRS → OperatingExpenses (in identity set)
        assert_eq!(
            canonicalize_concept(&q("OperatingExpensesIFRS")).map(|e| e.item_name),
            Some("operating_expenses".to_string())
        );
    }

    #[test]
    fn identity_stem_preserves_bs_position_codes() {
        // OtherCA and OtherCL must NOT be collapsed to the same canonical name.
        // BS position codes (CA, NCA, CL, NCL) are excluded from identity suffixes.
        let ca = canonicalize_concept(&q("OtherCA")).map(|e| e.item_name);
        let cl = canonicalize_concept(&q("OtherCL")).map(|e| e.item_name);
        // They should resolve to different names (or stay as identity)
        assert_ne!(
            ca, cl,
            "OtherCA and OtherCL must not map to the same canonical name"
        );
    }

    #[test]
    fn field_name_map_uses_snake_case_for_identity_concepts() {
        let map = field_name_map();

        assert_eq!(
            map.get("OperatingExpenses").map(String::as_str),
            Some("operating_expenses")
        );
        assert_eq!(
            map.get("RetirementBenefitExpensesSGA").map(String::as_str),
            Some("retirement_benefit_expenses_sga")
        );
    }

    #[test]
    fn commission_fee_variants_are_distinct() {
        let variants = [
            "CommissionFee",
            "CommissionFeeFinCF",
            "CommissionFeeCOSExpOA",
            "CommissionFeeEI",
            "CommissionFeeNOI",
        ];
        let canonicals: std::collections::HashSet<String> = variants
            .iter()
            .filter_map(|c| canonicalize_concept(&q(c)).map(|e| e.item_name))
            .collect();
        assert_eq!(
            canonicals.len(),
            variants.len(),
            "CommissionFee variants must keep accounting context: {canonicals:?}"
        );
    }

    #[test]
    fn identity_stem_excludes_summary_items() {
        // SummaryOfBusinessResults items should NOT be stem-normalized
        let result = canonicalize_concept(&q("CommissionFeeSummaryOfBusinessResults"));
        // Should either be None or identity pass-through, NOT snake_case
        assert!(
            result.is_none()
                || result
                    .as_ref()
                    .is_some_and(|e| e.item_name.contains("SummaryOfBusinessResults")),
            "Summary items should not be identity-stem-normalized, got: {result:?}"
        );
    }

    #[test]
    fn to_snake_case_conversions() {
        assert_eq!(
            to_snake_case("ProvisionForProductWarranties"),
            "provision_for_product_warranties"
        );
        assert_eq!(to_snake_case("CommissionFee"), "commission_fee");
        assert_eq!(to_snake_case("IFRS"), "ifrs");
        assert_eq!(to_snake_case("NetSales"), "net_sales");
        assert_eq!(to_snake_case("Revenue"), "revenue");
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case("OtherCA"), "other_ca");
        assert_eq!(to_snake_case("IFRSSummary"), "ifrs_summary");
        assert_eq!(
            to_snake_case("BenefitsForCovid19OpeCF"),
            "benefits_for_covid19_ope_cf"
        );
        assert_eq!(
            to_snake_case("PaymentsForTransferOfBusiness2InvCF"),
            "payments_for_transfer_of_business2_inv_cf"
        );
    }

    #[test]
    fn identity_does_not_strip_industry_code() {
        // Industry codes are no longer stripped in the identity layer either:
        // OperatingExpensesFND must NOT collapse to operating_expenses. The
        // reviewed operating_expenses_ins alias is an explicit STEM_MAP entry,
        // not a suffix-strip result.
        let resolved = canonicalize_concept(&q("OperatingExpensesFND")).map(|e| e.item_name);
        assert_ne!(resolved, Some("operating_expenses".to_string()));
    }

    // ── StatementCategory classification tests ──

    #[test]
    fn classify_bs_items() {
        use StatementCategory::*;
        assert_eq!(classify_item_name("total_assets"), Some(BalanceSheet));
        assert_eq!(classify_item_name("current_assets"), Some(BalanceSheet));
        assert_eq!(
            classify_item_name("current_liabilities"),
            Some(BalanceSheet)
        );
        assert_eq!(
            classify_item_name("stockholders_equity"),
            Some(BalanceSheet)
        );
        assert_eq!(classify_item_name("total_equity"), Some(BalanceSheet));
        assert_eq!(classify_item_name("total_liabilities"), Some(BalanceSheet));
        assert_eq!(classify_item_name("cash_and_deposits"), Some(BalanceSheet));
        assert_eq!(classify_item_name("inventories"), Some(BalanceSheet));
        assert_eq!(
            classify_item_name("tangible_fixed_assets"),
            Some(BalanceSheet)
        );
        assert_eq!(classify_item_name("capital_stock"), Some(BalanceSheet));
    }

    #[test]
    fn classify_pl_items() {
        use StatementCategory::*;
        assert_eq!(classify_item_name("revenue"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("operating_income"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("ordinary_income"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("net_income"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("cost_of_revenue"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("gross_profit"), Some(ProfitAndLoss));
        assert_eq!(
            classify_item_name("selling_general_admin"),
            Some(ProfitAndLoss)
        );
        assert_eq!(classify_item_name("depreciation"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("amortization"), Some(ProfitAndLoss));
        assert_eq!(
            classify_item_name("depreciation_and_amortization"),
            Some(ProfitAndLoss)
        );
        assert_eq!(classify_item_name("income_taxes"), Some(ProfitAndLoss));
        assert_eq!(classify_item_name("impairment_loss"), Some(ProfitAndLoss));
    }

    #[test]
    fn classify_cf_items() {
        use StatementCategory::*;
        assert_eq!(classify_item_name("operating_cf"), Some(CashFlow));
        assert_eq!(classify_item_name("investing_cf"), Some(CashFlow));
        assert_eq!(classify_item_name("financing_cf"), Some(CashFlow));
        assert_eq!(classify_item_name("capex"), Some(CashFlow));
        assert_eq!(
            classify_item_name("cash_dividends_paid_financing_cf"),
            Some(CashFlow)
        );
        assert_eq!(
            classify_item_name("depreciation_and_amortization_cf_adjustment"),
            Some(CashFlow)
        );
        assert_eq!(
            classify_item_name("treasury_stock_purchase"),
            Some(CashFlow)
        );
        assert_eq!(classify_item_name("lt_debt_repayment"), Some(CashFlow));
        assert_eq!(classify_item_name("lt_debt_proceeds"), Some(CashFlow));
    }

    #[test]
    fn classify_non_statement_items() {
        use StatementCategory::*;
        assert_eq!(classify_item_name("dividends_per_share"), Some(PerShare));
        assert_eq!(
            classify_item_name("dividends_per_share_summary"),
            Some(PerShare)
        );
        assert_eq!(classify_item_name("shares_outstanding"), Some(ShareData));
        assert_eq!(classify_item_name("forecast_net_income"), Some(Forecast));
        assert_eq!(
            classify_item_name("dividends_from_surplus"),
            Some(ChangesInEquity)
        );
    }

    #[test]
    fn classify_unknown_returns_none() {
        assert_eq!(classify_item_name("definitely_unknown_item"), None);
    }

    /// Verify that every canonical field name produced by STEM_MAP is classified.
    #[test]
    fn stem_map_covers_all_classifications() {
        for &(concept, field_name) in STEM_MAP {
            let category = classify_item_name(field_name);
            assert!(
                category.is_some(),
                "canonical field name {field_name:?} (from concept {concept:?}) \
                 is not covered by classify_item_name"
            );
        }
    }

    /// Verify accounting consistency: BS items are assets/liabilities/equity,
    /// PL items are revenue/expenses/profit, CF items are cash flows.
    #[test]
    fn classification_follows_accounting_rules() {
        use StatementCategory::*;

        // Assets → BS
        for name in [
            "total_assets",
            "current_assets",
            "non_current_assets",
            "cash_and_deposits",
            "inventories",
            "tangible_fixed_assets",
            "intangible_fixed_assets",
            "investment_securities",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(BalanceSheet),
                "{name} should be BalanceSheet (asset)"
            );
        }

        // Liabilities → BS
        for name in [
            "total_liabilities",
            "current_liabilities",
            "non_current_liabilities",
            "long_term_loans",
            "short_term_loans",
            "long_term_debt",
            "short_term_debt",
            "bonds_payable",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(BalanceSheet),
                "{name} should be BalanceSheet (liability)"
            );
        }

        // Equity → BS
        for name in [
            "total_equity",
            "stockholders_equity",
            "retained_earnings",
            "capital_stock",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(BalanceSheet),
                "{name} should be BalanceSheet (equity)"
            );
        }

        // Revenue/Profit → PL
        for name in [
            "revenue",
            "gross_profit",
            "operating_income",
            "ordinary_income",
            "net_income",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(ProfitAndLoss),
                "{name} should be ProfitAndLoss (revenue/profit)"
            );
        }

        // Expenses → PL
        for name in [
            "cost_of_revenue",
            "selling_general_admin",
            "depreciation",
            "amortization",
            "depreciation_and_amortization",
            "interest_expenses",
            "income_taxes",
            "impairment_loss",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(ProfitAndLoss),
                "{name} should be ProfitAndLoss (expense)"
            );
        }

        // Cash flow items → CF
        for name in ["operating_cf", "investing_cf", "financing_cf", "capex"] {
            assert_eq!(
                classify_item_name(name),
                Some(CashFlow),
                "{name} should be CashFlow"
            );
        }
    }

    #[test]
    fn forecast_items_differ_from_actuals() {
        assert_ne!(
            canonicalize_concept(&q("ForecastNetIncomeSummaryOfBusinessResults"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("NetIncome")).unwrap().item_name,
        );
    }

    #[test]
    fn summary_per_share_is_not_forecast() {
        assert_eq!(
            canonicalize_concept(&q("DividendPaidPerShareSummaryOfBusinessResults"))
                .unwrap()
                .item_name,
            "dividends_per_share_summary"
        );
    }

    #[test]
    fn generic_dividends_paid_is_a_cash_flow_item() {
        let generic = canonicalize_concept(&ifrs("DividendsPaid")).unwrap();
        assert_eq!(generic.item_name, "cash_dividends_paid");
        assert_eq!(
            classify_item_name(&generic.item_name),
            Some(StatementCategory::CashFlow)
        );
        assert_ne!(
            generic.item_name,
            canonicalize_concept(&q("DividendsPaidFromSurplus"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn dividends_from_surplus_is_not_cf_dividend() {
        assert_ne!(
            canonicalize_concept(&q("DividendsFromSurplus"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("CashDividendsPaid"))
                .unwrap()
                .item_name,
        );
        assert_ne!(
            canonicalize_concept(&q("DividendsPaidFromSurplus"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("CashDividendsPaid"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn dividends_paid_cf_roles_are_distinct() {
        assert_ne!(
            canonicalize_concept(&q("DividendsPaidFinCF"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("DividendsPaidInvCF"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn depreciation_cf_differs_from_pl() {
        assert_ne!(
            canonicalize_concept(&q("DepreciationOpeCF"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("Depreciation")).unwrap().item_name,
        );
    }

    #[test]
    fn depreciation_and_amortization_cf_differs_from_pl() {
        assert_ne!(
            canonicalize_concept(&q("DepreciationAndAmortizationOpeCF"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("DepreciationAndAmortization"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn interest_expense_cf_roles_are_distinct() {
        assert_ne!(
            canonicalize_concept(&q("InterestExpensesFinCF"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("InterestExpensesOpeCF"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn cash_deposits_differ_from_equivalents() {
        assert_ne!(
            canonicalize_concept(&q("CashAndDeposits"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("CashAndCashEquivalents"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn cash_equivalents_beginning_differs_from_end() {
        assert_ne!(
            canonicalize_concept(&q("CashAndCashEquivalentsAtBeginningOfPeriod"))
                .unwrap()
                .item_name,
            canonicalize_concept(&q("CashAndCashEquivalentsAtEndOfPeriod"))
                .unwrap()
                .item_name,
        );
    }

    #[test]
    fn cf_suffix_not_stripped_for_identity_concepts() {
        let item = canonicalize_concept(&q("RetirementBenefitExpensesOpeCF"))
            .unwrap()
            .item_name;
        assert!(
            item.ends_with("_ope_cf") || item.contains("_operating_cf"),
            "CF suffix context must be preserved: {item}"
        );
    }

    #[test]
    fn shares_outstanding_is_share_data_not_per_share() {
        assert_eq!(
            classify_item_name("shares_outstanding"),
            Some(StatementCategory::ShareData),
        );
    }

    #[test]
    fn taxonomy_json_reverse_map_is_consistent() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();
        let reverse_map = taxonomy
            .get("reverse_map")
            .and_then(Value::as_object)
            .unwrap();

        let mut field_entries: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for (element_id, entry) in field_map {
            let field = entry.get("field").and_then(Value::as_str).unwrap();
            field_entries
                .entry(field.to_string())
                .or_default()
                .insert(element_id.to_string());
        }

        for (field, element_ids) in &field_entries {
            let reverse_entries = reverse_map
                .get(field)
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("missing reverse_map field {field}"));
            let reverse_ids: std::collections::HashSet<String> = reverse_entries
                .iter()
                .map(|entry| {
                    entry
                        .get("element_id")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string()
                })
                .collect();
            assert_eq!(
                &reverse_ids, element_ids,
                "reverse_map mismatch for {field}"
            );
        }

        for (field, reverse_entries) in reverse_map {
            let expected = field_entries
                .get(field)
                .unwrap_or_else(|| panic!("reverse_map has unknown field {field}"));
            for entry in reverse_entries.as_array().unwrap() {
                let element_id = entry.get("element_id").and_then(Value::as_str).unwrap();
                assert!(
                    expected.contains(element_id),
                    "reverse_map entry {field}/{element_id} is absent from field_map"
                );
            }
        }
    }

    #[test]
    fn oci_concepts_canonicalize_and_classify() {
        use StatementCategory::*;
        // STEM_MAP OCI entries
        assert_eq!(
            canonicalize_concept(&q("OtherComprehensiveIncome")).map(|e| e.item_name),
            Some("other_comprehensive_income".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q(
                "GainsLossesOnRemeasuringRecognisedNetDefinedBenefitPlansNotYetRecognised"
            ))
            .map(|e| e.item_name),
            Some("remeasurement_of_defined_benefit_plans".to_string())
        );
        assert_eq!(
            canonicalize_concept(&q("GainsLossesOnExchangeDifferencesOnTranslation"))
                .map(|e| e.item_name),
            Some("foreign_currency_translation_adjustment".to_string())
        );
        // Classification
        for name in [
            "other_comprehensive_income",
            "total_comprehensive_income",
            "revaluation_surplus",
            "foreign_currency_translation_adjustment",
            "available_for_sale_securities_valuation_difference",
            "cash_flow_hedge_valuation_difference",
            "remeasurement_of_defined_benefit_plans",
        ] {
            assert_eq!(
                classify_item_name(name),
                Some(OtherComprehensiveIncome),
                "{name} should be OtherComprehensiveIncome"
            );
        }
    }

    /// Verify that every taxonomy.json field_map concept canonicalizes to the
    /// field value declared in the JSON.  This catches the class of bug where
    /// STEM_MAP silently overwrites a taxonomy.json mapping and the reverse_map
    /// becomes stale.
    #[test]
    fn taxonomy_json_field_map_matches_actual_canonicalization() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();

        let mut mismatches: Vec<(&str, &str, String)> = Vec::new();
        for (full_key, entry) in field_map {
            // DEI and summary categories are not financial-item concepts.
            if entry.get("category").and_then(Value::as_str) == Some("dei") {
                continue;
            }
            let expected_field = entry.get("field").and_then(Value::as_str).unwrap();
            if expected_field.is_empty() {
                continue;
            }
            let (prefix, local_name) = full_key
                .split_once(':')
                .unwrap_or((full_key.as_str(), full_key.as_str()));

            // Skip concepts that are non-scalar — they are intentionally filtered
            // from canonicalization and would return None.
            if is_non_scalar_concept(local_name) {
                continue;
            }

            // Canonicalization is namespace-aware: build the ConceptKey under
            // the family this entry's prefix belongs to.
            let key = (uri_for_prefix(prefix), local_name.to_string());
            let Some(actual) = canonicalize_concept(&key) else {
                mismatches.push((full_key, expected_field, "<none>".to_string()));
                continue;
            };
            if actual.item_name != expected_field {
                mismatches.push((full_key, expected_field, actual.item_name.clone()));
            }
        }
        assert!(
            mismatches.is_empty(),
            "taxonomy.json field_map mismatches with canonicalization:\n{}",
            mismatches
                .iter()
                .map(|(k, expected, actual)| format!("  {k}: json={expected}, actual={actual}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Verify that classify_item_name covers every canonical field name produced
    /// by taxonomy.json field_map (not just STEM_MAP).
    #[test]
    fn classify_item_name_covers_taxonomy_json_fields() {
        let taxonomy: Value = serde_json::from_str(TAXONOMY_JSON).unwrap();
        let field_map = taxonomy
            .get("field_map")
            .and_then(Value::as_object)
            .unwrap();

        for (full_key, entry) in field_map {
            if entry.get("category").and_then(Value::as_str) == Some("dei") {
                continue;
            }
            let field_name = entry.get("field").and_then(Value::as_str).unwrap();
            if field_name.is_empty() {
                continue;
            }
            let local_name = full_key
                .split_once(':')
                .map_or(full_key.as_str(), |(_, local)| local);
            if is_non_scalar_concept(local_name) {
                continue;
            }
            assert!(
                classify_item_name(field_name).is_some(),
                "taxonomy.json field {field_name} (from {full_key}) \
                 is not covered by classify_item_name"
            );
        }
    }

    // ── Namespace-aware regression tests (Phase 4 hardening) ──

    #[test]
    fn same_local_name_in_different_namespaces_does_not_collide() {
        // A recognized US-GAAP `Revenue` resolves to the revenue canonical.
        let us_gaap = (
            "http://fasb.org/us-gaap/2024".to_string(),
            "Revenue".to_string(),
        );
        assert_eq!(
            canonicalize_concept(&us_gaap).map(|c| c.item_name),
            Some("revenue".to_string())
        );
        // A company extension that reuses the local name `Revenue` must NOT be
        // collapsed onto the standard concept.
        let extension = (
            "https://filer.example.co.jp/taxonomy/2024".to_string(),
            "Revenue".to_string(),
        );
        assert_ne!(
            canonicalize_concept(&extension).map(|c| c.item_name),
            Some("revenue".to_string())
        );
        assert!(canonicalize_concept(&extension).is_none());
    }

    #[test]
    fn extension_namespace_is_classified_without_local_fallback() {
        // `Assets` maps to total_assets under any recognized EDINET namespace...
        assert_eq!(
            canonicalize_concept(&q("Assets")).map(|c| c.item_name),
            Some("total_assets".to_string())
        );
        // ...but an unrecognized namespace gets no local-name fallback, even
        // though the local name is a known alias. It is classified as an
        // extension boundary instead of Unknown.
        let ext = (
            "https://filer.example.co.jp/taxonomy/2024".to_string(),
            "Assets".to_string(),
        );
        assert!(canonicalize_concept(&ext).is_none());
        assert!(matches!(
            concept_status(&ext),
            TaxonomyConceptStatus::Extension
        ));
    }

    #[test]
    fn known_taxonomy_family_can_use_reviewed_family_alias() {
        // Each recognized EDINET family resolves its reviewed family-scoped
        // aliases. jppfs_cor:Assets -> total_assets.
        let jppfs = (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
            "Assets".to_string(),
        );
        assert_eq!(
            canonicalize_concept(&jppfs).map(|c| c.item_name),
            Some("total_assets".to_string())
        );
        // jpigp_cor:BondsAndBorrowingsLiabilitiesIFRS -> bonds_and_borrowings.
        let jpigp = (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2020-11-01/jpigp_cor".to_string(),
            "BondsAndBorrowingsLiabilitiesIFRS".to_string(),
        );
        assert_eq!(
            canonicalize_concept(&jpigp).map(|c| c.item_name),
            Some("bonds_and_borrowings".to_string())
        );
        // A bank's top-line tag (reviewed industry alias) resolves to revenue.
        let bnk = (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
            "OrdinaryIncomeBNK".to_string(),
        );
        assert_eq!(
            canonicalize_concept(&bnk).map(|c| c.item_name),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn exact_qname_override_beats_family_mapping() {
        // Layer 1 (exact QName) is consulted before the family/safe_local maps,
        // so a specific (namespace, local) pin overrides the default. Verified
        // directly against the resolver with a hand-built map set.
        let mut exact: HashMap<ConceptKey, CanonicalConcept> = HashMap::new();
        exact.insert(
            (
                "https://filer.example.co.jp/override/2024".to_string(),
                "Assets".to_string(),
            ),
            CanonicalConcept {
                item_name: "override_total_assets".to_string(),
                priority: 99,
                match_kind: MatchKind::ExactExpandedName,
            },
        );
        let maps = TaxonomyMaps {
            exact,
            family: HashMap::new(),
            safe_local: HashMap::new(),
        };
        let key = (
            "https://filer.example.co.jp/override/2024".to_string(),
            "Assets".to_string(),
        );
        let resolved = resolve_semantic(&key, &maps).unwrap();
        assert_eq!(resolved.item_name, "override_total_assets");
        assert_eq!(resolved.match_kind, MatchKind::ExactExpandedName);
    }

    #[test]
    fn identity_resolution_is_reported_separately() {
        // An identity concept in a recognized namespace is Identity (not
        // Semantic), and carries the Identity match kind.
        match concept_status(&q("AccountsPayableOther")) {
            TaxonomyConceptStatus::Identity(c) => {
                assert_eq!(c.match_kind, MatchKind::Identity);
            }
            other => panic!("expected Identity, got {other:?}"),
        }
    }

    #[test]
    fn real_official_unmapped_taxonomy_tags_are_identity_classified() {
        // Observed in the 10-year real-data audit. This is an exact identity
        // list entry, not a catch-all fallback for arbitrary unknown tags.
        let nuclear = q("ContributionPayableForNuclearReactorDecommissioningNCLELE");
        match concept_status(&nuclear) {
            TaxonomyConceptStatus::Identity(c) => {
                assert_eq!(
                    c.item_name,
                    "contribution_payable_for_nuclear_reactor_decommissioning_nclele"
                );
                assert_eq!(c.match_kind, MatchKind::Identity);
            }
            other => panic!("expected Identity, got {other:?}"),
        }

        assert!(matches!(
            concept_status(&q("DefinitelyUnknownConceptForStrictTaxonomy")),
            TaxonomyConceptStatus::Unknown
        ));
    }

    #[test]
    fn classify_namespace_uri_recognizes_edinet_and_standard_families() {
        use TaxonomyFamily::*;
        for (uri, expected) in [
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2019-02-28/jppfs_cor",
                EdinetJppfs,
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2020-11-01/jpigp_cor",
                EdinetJpigp,
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2024-11-01/jpcrp_cor",
                EdinetJpcrp,
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpdei/2024-11-01/jpdei_cor",
                EdinetJpDei,
            ),
            ("http://xbrl.fasb.org/us-gaap/2024", UsGaap),
            ("http://www.ifrs.org/taxonomy/2024", Ifrs),
        ] {
            assert_eq!(classify_namespace_uri(uri), Some(expected), "{uri}");
        }
        // Company extensions and arbitrary URIs are unrecognized.
        assert_eq!(
            classify_namespace_uri("https://filer.example.co.jp/x"),
            None
        );
        assert_eq!(classify_namespace_uri(""), None);
    }
}

