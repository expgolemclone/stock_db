use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyConceptStatus, canonicalize_synthetic};
#[cfg(test)]
use base::StatementCategory;


use crate::types::ConceptKey;

fn reviewed_jppfs_non_trade_current_receivable_alias(local_name: &str) -> Option<&'static str> {
    let field = match local_name {
        // Current income and monetized receivables are asset-side claims due in
        // the current period, but they do not carry the trade/customer marker
        // required for the narrower trade receivable bucket.
        "AccountsReceivableDividendsCA"
        | "AccountReceivableMonetizationOfReceivableCA"
        | "AccountsReceivableMonetizationOfReceivableCA"
        | "AccruedDividendCA"
        | "AccruedRentIncomeCA"
        | "AccruedRevenueCA" => "trade_and_other_current_receivables",

        // Lease, rental, operating-loan, subcontractor, and other receivable tags
        // are current receivable assets, but they are not necessarily
        // unconditional trade accounts receivable. Keep them in the broader
        // current-receivables bucket so statement totals are not overstated as
        // trade receivables.
        "AccountReceivableSubcontractorOtherCA"
        | "AccountsReceivableForRentCA"
        | "AccountsReceivableLeaseCA"
        | "AccountsReceivableLeaseCALEA"
        | "AccountsReceivableLeasingCA"
        | "AccountsReceivableOperatingLoansCASPF"
        | "AccountsReceivableOther"
        | "AccountsReceivableOtherBusinessCA"
        | "AccountsReceivableOtherBusinessCAWAT"
        | "AccountsReceivableOtherCA"
        | "AccountsReceivableOtherFromFactor"
        | "AccountsReceivableOtherFromSubsidiariesAndAffiliates"
        | "AccountsReceivableOtherFromSubsidiariesAndAffiliatesIOA"
        | "AccountsReceivableOtherNetCA"
        | "AccountsReceivableOtherOfStorageAgencyCA"
        | "AccountsReceivablePaidProvisionCA"
        | "AccountsReceivableSupplyOfMaterialsWithPaymentArrangementCA"
        | "AccountsReceivableRentalCA"
        | "AccountsReceivableRentalCAAssets" => "trade_and_other_current_receivables",

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

    let field = reviewed_jppfs_non_trade_current_receivable_alias(&concept.1)?;
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
    reviewed_jppfs_non_trade_current_receivable_alias(local_name).is_none()
        && base::is_excluded_concept(local_name)
}

#[cfg(test)]
pub fn classify_item_name(item_name: &str) -> Option<StatementCategory> {
    base::classify_item_name(item_name)
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    base::statement_for_item_name(item_name)
}

