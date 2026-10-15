#![allow(dead_code, unused_imports)]

#[path = "../src/types.rs"]
mod types;

#[path = "../src/taxonomy_jppfs_pl_suffix_batch3_20260617_reviewed_aliases.rs"]
mod taxonomy;

use taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, canonicalize_semantic_concept,
    classify_item_name, concept_status, statement_for_concept,
};

fn jppfs(local: &str) -> types::ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2026-11-01/jppfs_cor".to_string(),
        local.to_string(),
    )
}

fn extension(local: &str) -> types::ConceptKey {
    (
        "https://issuer.example.jp/taxonomy/2026".to_string(),
        local.to_string(),
    )
}

#[test]
fn accounting_factcheck_uses_active_taxonomy_stack() {
    for (local_name, expected_field, expected_statement) in [
        // Current active top layer: taxonomy_jppfs_pl_suffix_batch3_20260617.
        ("AdjustmentReceivedNOI", "non_operating_income", "pl"),
        ("AcquisitionExpenseNOE", "non_operating_expenses", "pl"),
        (
            "AdjustmentForTransferPricingTaxationEI",
            "extraordinary_income",
            "pl",
        ),
        ("AcquisitionRelatedCostsEL", "extraordinary_loss", "pl"),
        // Compensation/commission layer under batch3.
        ("CommissionForInsuranceNOI", "non_operating_income", "pl"),
        ("CommissionFeesNOE", "non_operating_expenses", "pl"),
        ("CompensationForDamageEI", "extraordinary_income", "pl"),
        ("CompensationForDamageEL", "extraordinary_loss", "pl"),
        // Earlier PL suffix layers.
        (
            "ContractCancellationFeeIncomeNOI",
            "non_operating_income",
            "pl",
        ),
        (
            "ContractBusinessRelatedCostsNOE",
            "non_operating_expenses",
            "pl",
        ),
        ("CashBackIncomeNOI", "non_operating_income", "pl"),
        // Balance-sheet aliases from older working-capital layers must remain BS.
        (
            "AccountReceivableMedicalIncomeCA",
            "accounts_receivable_trade",
            "bs",
        ),
        (
            "AccountsPayableOtherAndAccruedExpenses",
            "accounts_payable_other_and_accrued_expenses",
            "bs",
        ),
    ] {
        let concept = jppfs(local_name);
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve in the active taxonomy stack"));

        assert_eq!(resolved.item_name, expected_field);
        assert_eq!(statement_for_concept(&concept), Some(expected_statement));
        assert!(matches!(
            concept_status(&concept),
            TaxonomyConceptStatus::Semantic(_)
        ));
    }
}

#[test]
fn pl_suffix_aliases_are_profit_and_loss_and_namespace_gated() {
    for local_name in [
        "AdjustmentReceivedNOI",
        "AcquisitionExpenseNOE",
        "AdjustmentForTransferPricingTaxationEI",
        "AcquisitionRelatedCostsEL",
        "CommissionForInsuranceNOI",
        "CommissionFeesNOE",
        "CompensationForDamageEI",
        "CompensationForDamageEL",
        "CashBackIncomeNOI",
        "ContractCancellationFeeIncomeNOI",
    ] {
        let concept = jppfs(local_name);
        let resolved = canonicalize_concept(&concept)
            .unwrap_or_else(|| panic!("{local_name} should resolve as reviewed"));
        assert_eq!(
            classify_item_name(&resolved.item_name),
            Some(StatementCategory::ProfitAndLoss)
        );
        assert!(canonicalize_concept(&extension(local_name)).is_none());
        assert!(matches!(
            concept_status(&extension(local_name)),
            TaxonomyConceptStatus::Extension
        ));
    }
}

#[test]
fn cash_flow_suffix_siblings_are_not_collapsed_to_pl_suffix_fields() {
    for (local_name, rejected_field) in [
        ("AdjustmentReceivedOpeCF", "non_operating_income"),
        ("AdvisoryFeeOpeCF", "non_operating_expenses"),
        (
            "BusinessIntegrationRelatedExpensesOpeCF",
            "extraordinary_loss",
        ),
        ("CommissionForCommitmentLineOpeCF", "non_operating_expenses"),
        ("CompensationForDamageIncomeOpeCF", "non_operating_income"),
        ("CompanyNameChangeCostOpeCF", "extraordinary_loss"),
        ("CashlessReturnIncomeOpeCF", "non_operating_income"),
        (
            "ConstructionRelatedCompensationForDamagesOpeCF",
            "extraordinary_loss",
        ),
        ("ContractCancellationRevenueOpeCF", "extraordinary_income"),
    ] {
        let resolved = canonicalize_semantic_concept(&jppfs(local_name));
        assert_ne!(
            resolved
                .as_ref()
                .map(|canonical| canonical.item_name.as_str()),
            Some(rejected_field),
            "{local_name} should not resolve to PL-only alias {rejected_field}"
        );
    }
}
