use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // JP-GAAP bank top-line operating revenue. Unlike the RevOA component
        // aliases below, BNK is an industry top-line tag and should fold into
        // the normalized revenue bucket.
        "OperatingRevenueBNK" => "revenue",

        "HarborTransportationIncomeRevOA"
        | "InternationalTransportationIncomeRevOA"
        | "LandTransportationIncomeRevOA"
        | "StevedoringIncomeRevOA"
        | "WarehouseIncomeRevOA"
        | "OtherShippingBusinessRevenueRevenueShipping"
        | "ShipLeaseRevenueShipping"
        | "RevenueForIncidentalBusinessesGAS"
        | "RevenueFromInstallationWorkGAS"
        | "OtherElectricityRevenueORElectricELE"
        | "SoldPowerToOtherSuppliersORElectricELE"
        | "SoldPowerToOtherUtilitiesORElectricELE"
        | "SettlementRevenueAmongUtilitiesORElectricELE"
        | "TransmissionRevenueORElectricELE"
        | "BrokerageIncomeORCMD"
        | "CreditCardRevenueRevOA"
        | "FinancingRevenueRevOA"
        | "IncomeFromPartnershipManagementRevOA"
        | "IncidentalBusinessOperatingRevenueELE"
        | "OperatingRevenueGeneralPassengerAutomobileTrafficRevOA"
        | "OtherFinancialRevenueRevOA"
        | "PerItemRevenueRevOA"
        | "ResidentialORElectricELE"
        | "RevenueFromCreditGuaranteeRevOA"
        | "RevenueFromLoanedFacilitiesORElectricELE"
        | "RevenueFromOperationalInvestmentSecuritiesRevOA" => "other_operating_revenue",

        "DebenturesLiabilitiesBNK" => "bonds_payable",
        "LongTermDebtToSubsidiariesAndAffiliatesNCLELE" => "long_term_debt",
        "ShortTermDebtToSubsidiariesAndAffiliatesCLELE"
        | "ShortTermDebtToSubsidiariesAndAffiliatesCLGAS" => "short_term_debt",
        "IntangibleAssetsAffiliatedRWY"
        | "IntangibleAssetsNCAIncidentalELC"
        | "IntangibleAssetsNCARWY"
        | "IntangibleAssetsNCATelecommunications" => "intangible_fixed_assets",
        "ConstructionMaterialCA" | "RawMaterialsCAGAS" => "inventories",

        // Trade/customer receivables and construction-contract receivables are
        // operating receivables. Receivables from contracts with customers are
        // unconditional receivables; conditional contract assets are classified
        // separately below so they are not collapsed into receivables.
        "AccountReceivableMedicalIncomeCA"
        | "AccounAccountsReceivableFromCompletedConstructionContractstsReceivableFromCompletedConstructionContracts"
        | "AccountsReceivable"
        | "AccountsReceivableAccountsReceivableFromCompletedConstructionContractsAndOther"
        | "AccountsReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS"
        | "AccountsReceivableAdvisoryFeeCA"
        | "AccountsReceivableAssetsIFRS"
        | "AccountsReceivableAssetsINS"
        | "AccountsReceivableCA"
        | "AccountsReceivableCareBusiness"
        | "AccountsReceivableCareBusinessCA"
        | "AccountsReceivableCareBusinessCAAssets"
        | "AccountsReceivableCommissionCA"
        | "AccountsReceivableCompletedConstructionCA"
        | "AccountsReceivableCompletedOperation"
        | "AccountsReceivableCompletedOperationCA"
        | "AccountsReceivableCompletedSurveyCA"
        | "AccountsReceivableCompletedServiceContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableDevelopmentBusiness"
        | "AccountsReceivableDevelopmentBusinessAndOther"
        | "AccountsReceivableDueFromFranchisedStores"
        | "AccountsReceivableFromCompletedConstructionContracts"
        | "AccountsReceivableFromCompletedConstructionContractsAReceivableFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsAccountsReceivableRealEstateBusinessAndAccountsReceivableTradeCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOther"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherCNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherReceivablesFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableFromCompletedConstructionContractsCNS"
        | "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivableFromCompletedConstructionContractsReceivablesFromContractsWithCustomersCACNS"
        | "AccountsReceivableFromFranchisee"
        | "AccountsReceivableFromSalesOnSideLineBusiness"
        | "AccountsReceivableFromSideBusinessCA"
        | "AccountsReceivableFromSideLineBusinessCNS"
        | "AccountsReceivableFromSubsidiariesAndAffiliatesTrade"
        | "AccountsReceivableMedicalIncomeCA"
        | "AccountsReceivableRealEstateBusiness"
        | "AccountsReceivableRealEstateBusinessAndOther"
        | "AccountsReceivableRealEstateBusinessAndOtherCA"
        | "AccountsReceivableRealEstateBusinessCA"
        | "AccountsReceivableReceivablesArisingFromContractsWithCustomersCurrentAssets"
        | "AccountsReceivableRentalCA"
        | "AccountsReceivableRentalCAAssets"
        | "AccountsReceivableSideLineBusinessCA"
        | "AccountsReceivableTenantsCA"
        | "AccountsReceivableTrade1CA"
        | "AccountsReceivableTradeAccountsReceivableTrade"
        | "AccountsReceivableTradeAndAccountsReceivableFromCompletedConstructionContracts"
        | "AccountsReceivableTradeAndAccountsReceivableFromCompletedConstructionContractsCA"
        | "AccountsReceivableTradeCA"
        | "AccountsReceivableTradeCAINV"
        | "AccountsReceivableTradeNet"
        | "AccountsReceivableTradeReceivablesFromContractsWithCustomers"
        | "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA"
        | "AccountsReceivablerealEstateBusiness" => "accounts_receivable_trade",

        // Includes notes receivable as well as accounts receivable. Keep the
        // broader notes-and-accounts receivable scope rather than folding it into
        // pure accounts receivable.
        "AccountsReceivableFromCompletedConstructionContractsNotesReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS" => {
            "notes_and_accounts_receivable_trade"
        }

        // These labels explicitly combine receivables with contract assets. A
        // receivable is an unconditional right to consideration, while a contract
        // asset remains conditional on something other than the passage of time,
        // so the combined presentation must not be collapsed into either pure
        // receivables or pure contract assets.
        "AccountReceivableTradeAndContractAssetsCA"
        | "AccountsReccivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableAndContractAssets"
        | "AccountsReceivableAndContractAssetsCA"
        | "AccountsReceivableAndContractAssetsTrade"
        | "AccountsReceivableCompletedOperationAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssets"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS"
        | "AccountsReceivableFromCompletedConstructionContractsAndOtherContractAssetsCA"
        | "AccountsReceivableFromCompletedConstructionContractsContractAssetsCACNS"
        | "AccountsReceivableOtherAndContractAssetCA"
        | "AccountsReceivableTradeAndContractAssets"
        | "AccountsReceivableTradeAndContractAssetsAccountsReceivableTrade"
        | "AccountsReceivableTradeAndContractAssetsCA"
        | "AccountsReceivableTradeAndContractAssetsNet"
        | "AccountsReceivableTradeAndContractCA"
        | "AccountsReceivablesAndContractAssetsCA"
        | "AccountsreceivabletradeandcontractassetsCA" => "accounts_receivable_and_contract_assets",

        // These combined labels also include notes receivable. Keep the notes-
        // and-accounts scope rather than folding into either pure contract assets
        // or the narrower accounts-receivable-and-contract-assets bucket.
        "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssets"
        | "AccountsReceivableTradeAndContractAssetsNotesAndAccountsReceivableTradeAndContractAssetsCA" => {
            "notes_and_accounts_receivable_trade_and_contract_assets"
        }

        // Pure contract assets are conditional rights to consideration. Keep them
        // in a separate balance-sheet asset bucket.
        "ContractAssets" | "ContractAssetsCA" => "contract_assets",

        "AccountPayableTradeForFranchisedStoreCL"
        | "AccountsPayableForConstructionContracts"
        | "AccountsPayableForConstructionContracts2CNS"
        | "AccountsPayableForConstructionContractsAndOther"
        | "AccountsPayableForConstructionContractsAndOtherCL"
        | "AccountsPayableForConstructionContractsAndOtherCLCNS"
        | "AccountsPayableForConstructionContractsAndOtherCNS"
        | "AccountsPayableForConstructionContractsCL"
        | "AccountsPayableForConstructionContractsCNS"
        | "AccountsPayableForSideLineBusinessCNS"
        | "AccountsPayableFromSubsidiariesAndAffiliatesTradeCL"
        | "AccountsPayableOperatingSpecific"
        | "AccountsPayableOtherToSubsidiariesAndAssociatesTradeCL"
        | "AccountsPayableTradeAndAccountsPayableForConstructionContracts"
        | "AccountsPayableTradeAndAccountsPayableForConstructionContractsCL"
        | "AccountsPayableTradeForFranchisedStores"
        | "AccountsPayableTradeForFranchisedStoresCL"
        | "AccountsPayableTradeFromSubsidiariesAndAffiliates"
        | "AccountsPayableTradeLiabilitiesIFRS"
        | "AccountsPayableTradeToSubsidiariesAndAffiliatesCL" => "accounts_payable_trade",

        // This composite tag includes notes payable, so keep the broader
        // notes-and-accounts payable scope instead of folding it into pure trade
        // accounts payable.
        "AccountsPayableTradeNotesPayableAccountsPayableForConstructionContractsAndOtherCL" => {
            "notes_and_accounts_payable_trade"
        }

        "ACcountsPayableInstallmentPurchase"
        | "AFactoringAccountPayable"
        | "AFactoringAccountPayableCL"
        | "AccountPayableFactoring"
        | "AccountPayableForWorksInProgressAndOtherCL"
        | "AccountPaylableSubsidiariesAndAssociates"
        | "AccountPaylableSubsidiariesAndAssociatesCL"
        | "AccountsPayable"
        | "AccountsPayableAffiliatedCompanies"
        | "AccountsPayableAffiliatedCompaniesCL"
        | "AccountsPayableCL"
        | "AccountsPayableCLIFRS"
        | "AccountsPayableCLINV"
        | "AccountsPayableConsignment"
        | "AccountsPayableDevelopmentBusinessAndOther"
        | "AccountsPayableDevelopmentProjectsInProgressCNS"
        | "AccountsPayableDueToMemberMerchants"
        | "AccountsPayableFacilities"
        | "AccountsPayableFacility"
        | "AccountsPayableForAffiliatedCompanyCL"
        | "AccountsPayableForEquipmentCL"
        | "AccountsPayableForEquipmentInvestmentCL"
        | "AccountsPayableForEquipmentLiabilities"
        | "AccountsPayableForFacilities"
        | "AccountsPayableForFacilitiesvCL"
        | "AccountsPayableForFactoring"
        | "AccountsPayableForFactoringCL"
        | "AccountsPayableForFranchisee"
        | "AccountsPayableFormSubsideiariesAndAssociateOther"
        | "AccountsPayableFormSubsideiariesAndAssociateOtherCL"
        | "AccountsPayableFromSubsidiariesAndAffiliatesOther"
        | "AccountsPayableFromSubsidiariesAndAffiliatesOtherCL"
        | "AccountsPayableFromSubsidiariesAndAssociatesCL"
        | "AccountsPayableFromSubsidiariesAndAssociatesOther"
        | "AccountsPayableInstallmentPurchase"
        | "AccountsPayableInstallmentPurchaseCL"
        | "AccountsPayableOfBuyNoncurrentAssetsCL"
        | "AccountsPayableOfCollectionCL"
        | "AccountsPayableOnMarginCL"
        | "AccountsPayableOtherBusinessCL"
        | "AccountsPayableOtherCL"
        | "AccountsPayableOtherFromSubsidiariesAndAffiliatesCL"
        | "AccountsPayableOtherToAffiliatesCL"
        | "AccountsPayableOtherToSubsidiariesAndAffiliates"
        | "AccountsPayableOtherToSubsidiariesAndAffiliatesCL"
        | "AccountsPayableOtherToSubsidiariesAndAffiliatesCLCL"
        | "AccountsPayablePropertyPlantAndEquipmentCL"
        | "AccountsPayablePurchaseOfCredit"
        | "AccountsPayableRealEstate"
        | "AccountsPayableRealEstateAndOtherCL"
        | "AccountsPayableRealEstateCL"
        | "AccountsPayableRentalAssetsCL"
        | "AccountsPayableRentalAssetsCLLiabilities"
        | "AccountsPayableSurveyingCL"
        | "AccountsPayableSurveyingSpecific"
        | "AccountsPayableToSubsidiariesAndAffiliatesCL"
        | "AccountsPayableToSubsidiariesAndAssociates"
        | "AccountsPayableToSubsidiariesAndAssociatesCL" => "accounts_payable_current",
        _ => return None,
    };
    Some(field)
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    if !matches!(
        base::classify_namespace_uri(&concept.0),
        Some(TaxonomyFamily::EdinetJppfs)
    ) {
        return None;
    }
    let field = reviewed_jppfs_alias(&concept.1)?;
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
    reviewed_jppfs_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    match item_name {
        "accounts_receivable_and_contract_assets"
        | "contract_assets"
        | "notes_and_accounts_receivable_trade_and_contract_assets" => {
            Some(StatementCategory::BalanceSheet)
        }
        _ => base::classify_item_name(item_name),
    }
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "accounts_receivable_and_contract_assets"
        | "contract_assets"
        | "notes_and_accounts_receivable_trade_and_contract_assets" => Some("bs"),
        _ => base::statement_for_item_name(item_name),
    }
}

