use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, TaxonomyFamily, canonicalize_synthetic, classify_namespace_uri};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

/// Reviewed EDINET J-GAAP amortization aliases observed in historical XBRL.
///
/// These aliases intentionally distinguish period profit-and-loss amortization,
/// operating-cash-flow non-cash add-backs, non-operating expenses, and
/// extraordinary losses.  The statement role suffix is accounting-significant:
/// an `OpeCF` amortization tag is a cash-flow adjustment, not a PL expense tag.
fn reviewed_jppfs_amortization_cash_flow_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Period amortization expenses. These are profit-and-loss expense tags,
        // not indirect-method cash-flow add-backs.
        "AmortizationExpensesOfGoodwillSGA"
        | "AmortizationOfCustomerRelatedAssetsSGA"
        | "AmortizationOfCustomerRelatedIntangibleAssetsSGA"
        | "AmortizationOfGoodwill"
        | "AmortizationOfGoodwillSGA"
        | "AmortizationOfLongTermPrepaidExpenses"
        | "AmortizationOfLongTermPrepaidExpensesSGA"
        | "AmortizationOfTrademarkRightSGA"
        | "AmortizationOfGuaranteeCommission"
        | "AmortizationOfGuaranteeDeposits" => "amortization",

        // Indirect-method operating cash-flow add-backs for amortization.
        // Keep these separate from period PL amortization expense.
        "AmortizationNetOfPremiumDiscountOnMarketableSecuritiesOpeCFIFRS"
        | "AmortizationOfBondIssuanceCost2OpeCF"
        | "AmortizationOfBondIssuanceCostOpeCF"
        | "AmortizationOfBondIssuanceExpensesOpeCF"
        | "AmortizationOfBusinessCommencementExpensesOpeCF"
        | "AmortizationOfContentsOpeCF"
        | "AmortizationOfDeferredAssetsUnderArticle113OfTheInsuranceBusinessLawOpeCFINS"
        | "AmortizationOfDeferredInsuranceAcquisitionCostsOpeCFIFRS"
        | "AmortizationOfDepositOpeCF"
        | "AmortizationOfDepositsOpeCF"
        | "AmortizationOfEquityInvestmentOpeCF"
        | "AmortizationOfGoodwillAndAmortizationOfNegativeGoodwillOpeCF"
        | "AmortizationOfGoodwillAndNegativeGoodwillOpeCF"
        | "AmortizationOfGoodwillAndNegativegoodwillOpeCF"
        | "AmortizationOfGoodwillELOpeCF"
        | "AmortizationOfGoodwillExtraordinaryLossOpeCF"
        | "AmortizationOfGoodwillNegativeGoodwillOpeCF"
        | "AmortizationOfGoodwillOpeCF"
        | "AmortizationOfGuaranteeDeposits1OpeCF"
        | "AmortizationOfGuaranteeDepositsOpeCF"
        | "AmortizationOfIdentifiableAssetsOpeCFIFRS"
        | "AmortizationOfInvestmentCorporationBondIssuanceCostsOpeCFINV"
        | "AmortizationOfInvestmentsInCapitalOpeCF"
        | "AmortizationOfLeaseAndGuaranteeDepositsOpeCF"
        | "AmortizationOfLeaseDepositsOpeCF"
        | "AmortizationOfLeaseholdAndGuaranteeDepositsOpeCF"
        | "AmortizationOfLeaseholdDepositsOpeCF"
        | "AmortizationOfLeaseholdRightOpeCFOpeCF"
        | "AmortizationOfLicenseContractDepositOpeCF"
        | "AmortizationOfLongTermPrepaidExpensesOpeCF"
        | "AmortizationOfLongtermPrepaidExpensesOpeCF"
        | "AmortizationOfNegativeGoodwillOpeCF"
        | "AmortizationOfNetRetirementBenefitObligationAtTransitionOpeCF"
        | "AmortizationOfOpeningExpensesOpeCF"
        | "AmortizationOfOrganizationExpensesOpeCFINV"
        | "AmortizationOfPrepaidExpensesOpeCF"
        | "AmortizationOfPriorServiceCostOpeCF"
        | "AmortizationOfPriorServiceCostsOpeCF"
        | "AmortizationOfRentalInventoryAssetsOpeCF"
        | "AmortizationOfRestrictedStockRemunerationOpeCF"
        | "AmortizationOfSecurityDepositAndSecurityDepositOpeCF"
        | "AmortizationOfSecurityDepositOpeCF"
        | "AmortizationOfSogtwareAndOthersOpeCF"
        | "AmortizationOfStockIssuanceCostOpeCF"
        | "AmortizationOfTademarkRightOpeCF"
        | "AmortizationOfTechnologyAssetsOpeCF"
        | "AmortizationOfTrademarkOpeCF"
        | "AmortizationOfTrademarkRightOpeCF"
        | "AmortizationOfTrademarksOpeCF"
        | "AmortizationOfUnrecognizedItemsOpeCF" => "amortization_cf_adjustment",

        // Non-operating amortization expense aliases. The NOE suffix carries the
        // statement role, so do not collapse these into ordinary amortization.
        "AmortizationNOE"
        | "AmortizationOfActuarialGainForEmployeeSRetirementBenefitsNOE"
        | "AmortizationOfBondIssuanceCostNOE"
        | "AmortizationOfBondIssuanceCostsEtcNOE"
        | "AmortizationOfBondIssuanceExpensesNOE"
        | "AmortizationOfBusinessCommencementExpensesNOE"
        | "AmortizationOfBusinessNOE"
        | "AmortizationOfCopyrightNOE"
        | "AmortizationOfDeferredAssetsNOE"
        | "AmortizationOfDeferredAssetsUnderArticle113OfInsuranceBusinessLawNOE"
        | "AmortizationOfDeferredOrganizationExpensesNOE"
        | "AmortizationOfDevelopmentExpensesNOE"
        | "AmortizationOfEquityInvestmentNOE"
        | "AmortizationOfGoodwillNOE"
        | "AmortizationOfInvestmentCorporationBondIssuanceCostsNOEINV"
        | "AmortizationOfIssuanceCostOfSubscriptionRightsToSharesNOE" => "non_operating_expenses",

        // Extraordinary amortization / goodwill-loss aliases. These belong to the
        // extraordinary-loss bucket, not to ordinary amortization expense.
        "AmortisationOfGoodwillEL"
        | "AmortizatinOfGoodwillEL"
        | "AmortizationGoodwillEL"
        | "AmortizationOfBusinessCommencementExpensesEL"
        | "AmortizationOfDeferredAssetsEL"
        | "AmortizationOfDepositsEL"
        | "AmortizationOfGoodWillEL"
        | "AmortizationOfGoodwill1EL"
        | "AmortizationOfGoodwillAndNegativeGoodwillEL"
        | "AmortizationOfGoodwillEL"
        | "AmortizationOfGoodwillExtraordinaryLoss"
        | "AmortizationOfGoodwillExtraordinaryLossEL"
        | "AmortizationOfGoodwillExtraordinaryLosses"
        | "AmortizationOfKeyMoneyEL"
        | "AmortizationOfLongTermPrepaidExpensesEL"
        | "AmortizationOfNegativeGoodwillEL"
        | "AmortizationOfPastServiceCostEL"
        | "AmortizationOfPastServiceCostsEL"
        | "AmortizationOfPrepaidExpensesEL"
        | "AmortizationOfPriorServiceCostEL"
        | "AmortizationOfPriorServiceCostsEL"
        | "AmortizationOrGoodwillEL"
        | "AmortrizationOfGoodwillEL"
        | "AmotizationOfGoodwillEL" => "extraordinary_loss",

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

    let field = reviewed_jppfs_amortization_cash_flow_alias(&concept.1)?;
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
    reviewed_jppfs_amortization_cash_flow_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

