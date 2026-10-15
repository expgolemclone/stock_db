use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

struct Alias {
    prefix: &'static str,
    local_name: &'static str,
    field: &'static str,
    category: &'static str,
    standard: &'static str,
    label_jp: &'static str,
    label_en: &'static str,
}

const REVIEWED_ALIASES: &[Alias] = &[
    // IFRS top-line revenue aliases.
    Alias {
        prefix: "jpigp_cor",
        local_name: "NetRevenueIFRS",
        field: "revenue",
        category: "financial_statement_ifrs",
        standard: "ifrs",
        label_jp: "純収益",
        label_en: "Net revenue",
    },
    Alias {
        prefix: "jpigp_cor",
        local_name: "ATMOperationBusinessRevenueIFRS",
        field: "revenue",
        category: "financial_statement_ifrs",
        standard: "ifrs",
        label_jp: "ATM運営業務収益",
        label_en: "ATM operation business revenue",
    },
    // JP-GAAP industry top-line revenue aliases that are semantically the same
    // top-line revenue bucket used by existing NetSales, OperatingRevenue, and
    // financial-industry revenue aliases.
    Alias {
        prefix: "jppfs_cor",
        local_name: "ElectricUtilityOperatingRevenueELE",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "電気事業営業収益",
        label_en: "Electric utility operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OperatingRevenueRailwayRWY",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "鉄道事業営業収益",
        label_en: "Operating revenue, railway",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "GasSalesGAS",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "ガス売上",
        label_en: "Gas sales",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "SalesFromGasBusinessGAS",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "ガス事業売上高",
        label_en: "Sales from gas business",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FreightRevenueShipping",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "運賃収益",
        label_en: "Freight revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FreightAndOtherRevenueShipping",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "運賃及びその他収益",
        label_en: "Freight and other revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ShippingBusinessRevenueWAT",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "海運業収益",
        label_en: "Shipping business revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "LeaseSalesSalesLEA",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "リース売上高",
        label_en: "Lease sales",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InstallmentSalesSalesLEA",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "割賦売上高",
        label_en: "Installment sales",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FinanceRevenueSalesLEA",
        field: "revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "金融収益",
        label_en: "Finance revenue",
    },
    // Other operating revenue aliases. These are operating-revenue components,
    // not total top-line revenue.
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherOperatingRevenueSPF",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他の営業収益",
        label_en: "Other operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "MiscellaneousOperatingRevenueGAS",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "雑営業収益",
        label_en: "Miscellaneous operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherMiscellaneousOperatingRevenueGAS",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他の雑営業収益",
        label_en: "Other miscellaneous operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherBusinessOperatingRevenueELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他事業営業収益",
        label_en: "Other business operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OperatingRevenueOtherBusinessesORIncidentalELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他事業営業収益",
        label_en: "Operating revenue from other businesses",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OperatingRevenueGasSupplyBusinessORIncidentalELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "ガス供給事業営業収益",
        label_en: "Operating revenue from gas supply business",
    },
    // Reviewed operating-revenue component aliases.  These are revenue streams
    // inside the operating-activities/revenue roles, but they are not evidence
    // that the concept is the issuer's total top-line revenue.
    Alias {
        prefix: "jppfs_cor",
        local_name: "HarborTransportationIncomeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "港湾運送収益",
        label_en: "Harbor transportation income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InternationalTransportationIncomeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "国際運送収益",
        label_en: "International transportation income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "LandTransportationIncomeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "陸上運送収益",
        label_en: "Land transportation income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "StevedoringIncomeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "荷役収益",
        label_en: "Stevedoring income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "WarehouseIncomeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "倉庫収益",
        label_en: "Warehouse income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherShippingBusinessRevenueRevenueShipping",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他海運業収益",
        label_en: "Other shipping business revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ShipLeaseRevenueShipping",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "船舶貸渡収益",
        label_en: "Ship lease revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RevenueForIncidentalBusinessesGAS",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "附帯事業収益",
        label_en: "Revenue for incidental businesses",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RevenueFromInstallationWorkGAS",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "工事収益",
        label_en: "Revenue from installation work",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherElectricityRevenueORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他電気事業収益",
        label_en: "Other electricity revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "SoldPowerToOtherSuppliersORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "他事業者への販売電力料",
        label_en: "Power sold to other suppliers",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "SoldPowerToOtherUtilitiesORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "他電力会社への販売電力料",
        label_en: "Power sold to other utilities",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "SettlementRevenueAmongUtilitiesORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "電力会社間精算収益",
        label_en: "Settlement revenue among utilities",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "TransmissionRevenueORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "送電収益",
        label_en: "Transmission revenue",
    },
    // Newly reviewed operating-revenue components from the unsupported-detail
    // bucket.  These are recurring operating revenue streams, but each is a
    // component line, not the issuer's total top-line revenue.
    Alias {
        prefix: "jppfs_cor",
        local_name: "BrokerageIncomeORCMD",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "受入手数料",
        label_en: "Brokerage income",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "CreditCardRevenueRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "クレジットカード収益",
        label_en: "Credit card revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FinancingRevenueRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "金融収益",
        label_en: "Financing revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IncomeFromPartnershipManagementRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "匿名組合等管理収益",
        label_en: "Income from partnership management",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IncidentalBusinessOperatingRevenueELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "附帯事業営業収益",
        label_en: "Incidental business operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OperatingRevenueGeneralPassengerAutomobileTrafficRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "一般旅客自動車運送事業営業収益",
        label_en: "Operating revenue, general passenger automobile traffic",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "OtherFinancialRevenueRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "その他金融収益",
        label_en: "Other financial revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "PerItemRevenueRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "品目別収益",
        label_en: "Per-item revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ResidentialORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "電灯料",
        label_en: "Residential electricity revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RevenueFromCreditGuaranteeRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "信用保証収益",
        label_en: "Revenue from credit guarantee",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RevenueFromLoanedFacilitiesORElectricELE",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "貸設備収益",
        label_en: "Revenue from loaned facilities",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RevenueFromOperationalInvestmentSecuritiesRevOA",
        field: "other_operating_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "営業投資有価証券収益",
        label_en: "Revenue from operational investment securities",
    },
    // Cost, depreciation, interest, and non-operating aliases.
    Alias {
        prefix: "jppfs_cor",
        local_name: "CostOfSalesCOSExpOA",
        field: "cost_of_revenue",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売上原価",
        label_en: "Cost of sales",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationCOS",
        field: "depreciation",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "減価償却費",
        label_en: "Depreciation",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationOEOILTelecommunications",
        field: "depreciation",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "減価償却費",
        label_en: "Depreciation",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationOERWY",
        field: "depreciation",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "減価償却費",
        label_en: "Depreciation",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationOESPF",
        field: "depreciation",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "減価償却費",
        label_en: "Depreciation",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationShipsShipExpensesShipping",
        field: "depreciation",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "船舶減価償却費",
        label_en: "Depreciation of ships",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationOfLeasedAssetsOpeCFLEA",
        field: "depreciation_cf_adjustment",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "リース資産減価償却費",
        label_en: "Depreciation of leased assets",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "DepreciationOfRealEstateForRentOpeCFINS",
        field: "depreciation_cf_adjustment",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "賃貸不動産減価償却費",
        label_en: "Depreciation of real estate for rent",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestExpensesOESPF",
        field: "interest_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "支払利息",
        label_en: "Interest expenses",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnBondsOESPF",
        field: "interest_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "社債利息",
        label_en: "Interest on bonds",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnCommercialPapersCOSExpOA",
        field: "interest_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "コマーシャル・ペーパー利息",
        label_en: "Interest on commercial papers",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnDebenturesBNK",
        field: "interest_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "債券利息",
        label_en: "Interest on debentures",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnShortTermBondsNOE",
        field: "interest_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "短期社債利息",
        label_en: "Interest on short-term bonds",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnDepositsORSPF",
        field: "interest_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "預金利息",
        label_en: "Interest on deposits",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnLoansORSPF",
        field: "interest_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "貸付金利息",
        label_en: "Interest on loans",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "InterestOnOperatingLoansSPF",
        field: "interest_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "営業貸付金利息",
        label_en: "Interest on operating loans",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "NonOperatingRevenueNORELE",
        field: "non_operating_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "営業外収益",
        label_en: "Non-operating revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FinancialRevenueNORELE",
        field: "non_operating_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "財務収益",
        label_en: "Financial revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "MiscellaneousRevenueNORELE",
        field: "non_operating_income",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "雑収益",
        label_en: "Miscellaneous revenue",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "NonOperatingExpensesNOEELE",
        field: "non_operating_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "営業外費用",
        label_en: "Non-operating expenses",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FinancialExpensesNOEELE",
        field: "non_operating_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "財務費用",
        label_en: "Financial expenses",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "FinancialExpensesSPF",
        field: "non_operating_expenses",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "財務費用",
        label_en: "Financial expenses",
    },
    // Payables alias with wording variation.
    Alias {
        prefix: "jppfs_cor",
        local_name: "NotesAndOperatingAccountsPayableTrade",
        field: "notes_and_accounts_payable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "支払手形及び営業未払金",
        label_en: "Notes and operating accounts payable, trade",
    },
    // Reviewed balance-sheet aliases whose local names are variants of the same
    // receivable/payable line item.  Do not include contract-asset hybrids here:
    // receivables and contract assets differ in enforceability and timing.
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsPayableTradeForFranchisedStores",
        field: "accounts_payable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "加盟店買掛金",
        label_en: "Accounts payable, trade for franchised stores",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsPayableTradeForFranchisedStoresCL",
        field: "accounts_payable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "加盟店買掛金",
        label_en: "Accounts payable, trade for franchised stores",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsPayableTradeAndAccountsPayableForConstructionContracts",
        field: "accounts_payable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "買掛金及び工事未払金",
        label_en: "Accounts payable, trade and for construction contracts",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL",
        field: "accounts_payable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "買掛金及び工事未払金",
        label_en: "Accounts payable, trade and for construction contracts",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsReceivableTrade1CA",
        field: "accounts_receivable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売掛金",
        label_en: "Accounts receivable, trade",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsReceivableTradeAccountsReceivableTrade",
        field: "accounts_receivable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売掛金",
        label_en: "Accounts receivable, trade",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsReceivableTradeCA",
        field: "accounts_receivable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売掛金",
        label_en: "Accounts receivable, trade",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsReceivableTradeReceivablesFromContractsWithCustomers",
        field: "accounts_receivable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売掛金及び顧客との契約から生じた債権",
        label_en: "Accounts receivable, trade and receivables from contracts with customers",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA",
        field: "accounts_receivable_trade",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "売掛金及び顧客との契約から生じた債権",
        label_en: "Accounts receivable, trade and receivables from contracts with customers",
    },
    // Reviewed balance-sheet aliases from industry and old-taxonomy tags.
    Alias {
        prefix: "jppfs_cor",
        local_name: "DebenturesLiabilitiesBNK",
        field: "bonds_payable",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "債券",
        label_en: "Debentures liabilities",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "LongTermDebtToSubsidiariesAndAffiliatesNCLELE",
        field: "long_term_debt",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "関係会社長期債務",
        label_en: "Long-term debt to subsidiaries and affiliates",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ShortTermDebtToSubsidiariesAndAffiliatesCLELE",
        field: "short_term_debt",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "関係会社短期債務",
        label_en: "Short-term debt to subsidiaries and affiliates",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ShortTermDebtToSubsidiariesAndAffiliatesCLGAS",
        field: "short_term_debt",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "関係会社短期債務",
        label_en: "Short-term debt to subsidiaries and affiliates",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IntangibleAssetsAffiliatedRWY",
        field: "intangible_fixed_assets",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "無形固定資産",
        label_en: "Intangible assets, affiliated railway",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IntangibleAssetsNCAIncidentalELC",
        field: "intangible_fixed_assets",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "無形固定資産",
        label_en: "Intangible assets, incidental business",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IntangibleAssetsNCARWY",
        field: "intangible_fixed_assets",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "無形固定資産",
        label_en: "Intangible assets, railway",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "IntangibleAssetsNCATelecommunications",
        field: "intangible_fixed_assets",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "無形固定資産",
        label_en: "Intangible assets, telecommunications",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "ConstructionMaterialCA",
        field: "inventories",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "工事材料",
        label_en: "Construction material",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "RawMaterialsCAGAS",
        field: "inventories",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "原材料",
        label_en: "Raw materials",
    },
    // Reviewed investing/financing cash-flow aliases.
    Alias {
        prefix: "jpigp_cor",
        local_name: "AdditionsToIntangibleAssetsInvCFIFRS",
        field: "intangible_asset_purchases",
        category: "financial_statement_ifrs",
        standard: "ifrs",
        label_jp: "無形資産の取得による支出",
        label_en: "Additions to intangible assets",
    },
    Alias {
        prefix: "jppfs_cor",
        local_name: "AcquisitionOfTreasurySharesViaTrustFinCF",
        field: "treasury_stock_purchase",
        category: "financial_statement",
        standard: "jp_gaap",
        label_jp: "信託を通じた自己株式の取得",
        label_en: "Acquisition of treasury shares via trust",
    },
];

fn main() {
    println!("cargo:rerun-if-changed=../src/stock_db/storage/taxonomy.json");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let taxonomy_path = Path::new(&manifest_dir).join("../src/stock_db/storage/taxonomy.json");
    let before = fs::read_to_string(&taxonomy_path).expect("read taxonomy.json");
    let mut taxonomy: Value = serde_json::from_str(&before).expect("taxonomy.json is valid JSON");

    apply_reviewed_aliases(&mut taxonomy);
    ensure_alias_canonical_fields(&mut taxonomy);

    let after = format!(
        "{}\n",
        serde_json::to_string_pretty(&taxonomy).expect("serialize taxonomy.json")
    );
    if before != after {
        fs::write(&taxonomy_path, after).expect("write taxonomy.json with reviewed aliases");
    }
}

fn apply_reviewed_aliases(taxonomy: &mut Value) {
    let reviewed_locals = REVIEWED_ALIASES
        .iter()
        .map(|alias| alias.local_name)
        .collect::<HashSet<_>>();

    if let Some(groups) = taxonomy
        .get_mut("excluded_concepts")
        .and_then(Value::as_object_mut)
    {
        for concepts in groups.values_mut().filter_map(Value::as_array_mut) {
            concepts.retain(|concept| {
                concept
                    .as_str()
                    .is_none_or(|local_name| !reviewed_locals.contains(local_name))
            });
        }
    }

    let field_map = taxonomy
        .get_mut("field_map")
        .and_then(Value::as_object_mut)
        .expect("taxonomy.json field_map must be an object");
    for alias in REVIEWED_ALIASES {
        let element_id = format!("{}:{}", alias.prefix, alias.local_name);
        field_map.entry(element_id).or_insert_with(|| {
            json!({
                "field": alias.field,
                "category": alias.category,
                "standard": alias.standard,
                "label_jp": alias.label_jp,
                "label_en": alias.label_en,
                "matched_by": "reviewed_alias"
            })
        });
    }

    let reverse_map = taxonomy
        .get_mut("reverse_map")
        .and_then(Value::as_object_mut)
        .expect("taxonomy.json reverse_map must be an object");
    for alias in REVIEWED_ALIASES {
        let element_id = format!("{}:{}", alias.prefix, alias.local_name);
        let reverse_entries = reverse_map
            .entry(alias.field.to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("reverse_map values must be arrays");
        let already_present = reverse_entries.iter().any(|entry| {
            entry
                .get("element_id")
                .and_then(Value::as_str)
                .is_some_and(|existing| existing == element_id)
        });
        if !already_present {
            reverse_entries.push(json!({
                "element_id": element_id,
                "category": alias.category,
                "standard": alias.standard
            }));
        }
    }
}

fn ensure_alias_canonical_fields(taxonomy: &mut Value) {
    let canonical_fields = taxonomy
        .get_mut("canonical_fields")
        .and_then(Value::as_object_mut)
        .expect("taxonomy.json canonical_fields must be an object");

    ensure_canonical_field(
        canonical_fields,
        "accounts_payable_trade",
        "営業買掛金",
        "Accounts payable, trade",
        "balance_sheet",
        "営業取引から生じる買掛債務. 工事未払金など業種別の営業債務を含み得る.",
    );
    ensure_canonical_field(
        canonical_fields,
        "accounts_receivable_trade",
        "営業売掛金",
        "Accounts receivable, trade",
        "balance_sheet",
        "営業取引から生じる売掛債権. 顧客との契約から生じた債権として表示されるものを含み得る.",
    );
    ensure_canonical_field(
        canonical_fields,
        "other_operating_revenue",
        "その他営業収益",
        "Other operating revenue",
        "profit_and_loss",
        "主たる営業活動に付随するその他の営業収益.",
    );
    ensure_canonical_field(
        canonical_fields,
        "non_operating_income",
        "営業外収益",
        "Non-operating income",
        "profit_and_loss",
        "営業活動以外から生じる収益項目.",
    );
    ensure_canonical_field(
        canonical_fields,
        "non_operating_expenses",
        "営業外費用",
        "Non-operating expenses",
        "profit_and_loss",
        "営業活動以外から生じる費用項目.",
    );
    ensure_canonical_field(
        canonical_fields,
        "depreciation_cf_adjustment",
        "減価償却費調整額",
        "Depreciation cash-flow adjustment",
        "cash_flow",
        "営業活動キャッシュ・フローで加減算される減価償却費調整額.",
    );
    ensure_canonical_field(
        canonical_fields,
        "notes_and_accounts_payable_trade",
        "支払手形及び買掛金",
        "Notes and accounts payable, trade",
        "balance_sheet",
        "営業取引から生じる支払手形及び買掛金.",
    );
    ensure_canonical_field(
        canonical_fields,
        "intangible_asset_purchases",
        "無形資産の取得による支出",
        "Additions to intangible assets",
        "cash_flow",
        "投資活動キャッシュ・フローで表示される無形資産の取得支出.",
    );
    ensure_canonical_field(
        canonical_fields,
        "treasury_stock_purchase",
        "自己株式の取得",
        "Purchase of treasury stock",
        "cash_flow",
        "財務活動キャッシュ・フローで表示される自己株式の取得支出.",
    );

    let field_map_count = taxonomy
        .get("field_map")
        .and_then(Value::as_object)
        .map_or(0, |field_map| field_map.len());
    let reverse_map_count = taxonomy
        .get("reverse_map")
        .and_then(Value::as_object)
        .map_or(0, |reverse_map| reverse_map.len());
    let canonical_fields_count = taxonomy
        .get("canonical_fields")
        .and_then(Value::as_object)
        .map_or(0, |fields| fields.len());

    let meta = taxonomy
        .get_mut("_meta")
        .and_then(Value::as_object_mut)
        .expect("taxonomy.json _meta must be an object");
    meta.insert("field_map_count".to_string(), json!(field_map_count));
    meta.insert("reverse_map_count".to_string(), json!(reverse_map_count));
    meta.insert(
        "canonical_fields_count".to_string(),
        json!(canonical_fields_count),
    );
}

fn ensure_canonical_field(
    canonical_fields: &mut Map<String, Value>,
    field: &str,
    label_jp: &str,
    label_en: &str,
    statement: &str,
    description_jp: &str,
) {
    canonical_fields
        .entry(field.to_string())
        .or_insert_with(|| {
            json!({
                "label_jp": label_jp,
                "label_en": label_en,
                "category": "financial_statement",
                "statement": statement,
                "standards": ["jp_gaap"],
                "description_jp": description_jp,
                "standard_notes": {
                    "jp_gaap": "レビュー済みtaxonomy aliasとして正規化する."
                },
                "sources": ["FSA EDINET Taxonomy 2026"]
            })
        });
}
