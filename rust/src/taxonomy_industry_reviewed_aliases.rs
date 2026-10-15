use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_industry_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Industry-specific current customer receivables. These are presented as
        // current operating/customer receivables in legacy EDINET industry
        // taxonomies, so fold them into the trade/customer receivable bucket
        // rather than leaving them excluded or treating them as financing assets.
        "AccountsReceivableCARWY"
        | "AccountsReceivableCustomerCACMD"
        | "AccountsReceivableFormAgentsCAWAT"
        | "AccountsReceivableFromSubsidiariesAndAffiliatesTradeCAGAS"
        | "AccountsReceivableInstallmentSalesCALEA"
        | "AccountsReceivableInstallmentSalesCreditGuaranteeCA"
        | "AccountsReceivableShippingCAWAT"
        | "RailwayFaresReceivablesCARWY" => "accounts_receivable_trade",

        // Non-trade current receivables and accrued-income current assets. These
        // are asset-side claims due in the current period, but they do not carry
        // the trade/customer marker required for the narrower trade receivable
        // bucket. The singular AccountReceivable... variant is an older/issuer
        // spelling of the same monetized-receivable current asset line.
        "AccountReceivableMonetizationOfReceivableCA"
        | "AccountsReceivableDividendsCA"
        | "AccountsReceivableMonetizationOfReceivableCA"
        | "AccruedDividendCA"
        | "AccruedRentIncomeCA"
        | "AccruedRevenueCA"
        | "OtherAccountsReceivableCAELE"
        | "ShortTermReceivablesFromSubsidiariesAndAffiliatesCAELE"
        | "ShortTermReceivablesFromSubsidiariesAndAffiliatesCAGAS" => {
            "trade_and_other_current_receivables"
        }

        // Lease-industry customer loan receivables are current customer/financial
        // receivables, but the loan wording is broader than trade accounts
        // receivable. Preserve the non-trade scope by using the broader current
        // receivables bucket.
        "AccountsReceivableOperatingLoansCALEA"
        | "AccountsReceivableOtherLoansToCustomersCALEA" => "trade_and_other_current_receivables",

        // Current accounts payable variants from industry-specific tags. Keep
        // the current-liability payable scope, but do not narrow these to trade
        // payables because the labels do not consistently carry a trade marker.
        "AccountsPayableCreditGuaranteeCL"
        | "AccountsPayableShippingCLWAT"
        | "CurrentPortionOfLongTermAccountsPayableRailwayEquipmentCLRWY"
        | "CurrentPortionOfLongTermPayablesUnderFluidityLeaseReceivablesCLLEA" => {
            "accounts_payable_current"
        }

        // Customer advances, prepaid fares, and similar consideration received in
        // advance are current liabilities arising before the related service is
        // provided. Keep them separate from vendor payables.
        "AdvancesReceivedLeaseCLLEA"
        | "InterLineFaresReceivedCLRWY"
        | "OtherAdvancesCLELE"
        | "PrepaidFaresReceivedCLRWY" => "advances_received_current",

        // Industry-specific income-statement aliases whose labels preserve the
        // same statement subtotal semantics as the normalized field.
        "OperatingGrossProfitWAT" => "gross_profit",
        "OperatingIncomeRailwayRWY" => "operating_income",
        "GeneralAndAdministrativeExpensesGAS"
        | "GeneralAndAdministrativeExpensesOEElectricELE"
        | "GeneralAndAdministrativeExpensesOERWY"
        | "GeneralAndAdministrativeExpensesWAT"
        | "SellingGeneralAndAdministrativeExpensesGAS" => "selling_general_admin",

        // Investing cash-flow purchase of assets used by the lessor/lessor-like
        // business. The InvCF suffix keeps this out of operating cash flow.
        "PurchaseOfOwnUsedAssetsInvCFLEA" => "capex",

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

    let field = reviewed_jppfs_industry_alias(&concept.1)?;
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
    reviewed_jppfs_industry_alias(local_name).is_none() && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

