use crate::taxonomy::{
    StatementCategory, TaxonomyConceptStatus, canonicalize_concept, classify_item_name,
    concept_status, raw_statement_for_concept,
};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn assert_reviewed_balance_sheet_alias(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let canonical = canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("{local_name} should resolve as a reviewed alias"));

    assert_eq!(canonical.item_name, expected_field);
    assert_eq!(
        classify_item_name(&canonical.item_name),
        Some(StatementCategory::BalanceSheet)
    );
    assert_eq!(raw_statement_for_concept(&concept), Some("bs"));
    assert!(matches!(
        concept_status(&concept),
        TaxonomyConceptStatus::Semantic(canonical) if canonical.item_name == expected_field
    ));
}

#[test]
fn payable_receivable_contract_and_advance_aliases_resolve_to_reviewed_fields() {
    for (local_name, expected_field) in [
        (
            "ACcountsPayableInstallmentPurchase",
            "accounts_payable_current",
        ),
        (
            "AccountsPayableFromSubsidiariesAndAffiliatesOther",
            "accounts_payable_current",
        ),
        ("AccountsReceivableAssetsIFRS", "accounts_receivable_trade"),
        (
            "AccountsReceivableMedicalIncomeCA",
            "accounts_receivable_trade",
        ),
        (
            "AccountsReceivableFromCompletedConstructionContractsAndOtherCA",
            "trade_and_other_current_receivables",
        ),
        (
            "AccountsReceivableTradeReceivablesFromContractsWithCustomersCA",
            "accounts_receivable_trade",
        ),
        (
            "AccountsReceivableFromCompletedConstructionContractsCNSReceivablesFromContractsWithCustomersCA",
            "accounts_receivable_trade",
        ),
        (
            "AccountsReceivableFromCompletedConstructionContractsNotesReceivableAccountsReceivableFromCompletedConstructionContractsAndOtherCNS",
            "notes_and_accounts_receivable_trade",
        ),
        (
            "AccountsReceivableAndContractAssetsCA",
            "accounts_receivable_and_contract_assets",
        ),
        (
            "AccountsReceivableFromCompletedConstructionContractsAndContractAssetsCNS",
            "accounts_receivable_and_contract_assets",
        ),
        (
            "AdvancesReceivedOnUncompletedContracts",
            "advances_received_current",
        ),
        (
            "AdvancesReceivedOnUmcompletedConstructionContractsAndOtherCNS",
            "advances_received_current",
        ),
    ] {
        assert_reviewed_balance_sheet_alias(local_name, expected_field);
    }
}

#[test]
fn unsafe_bulk_candidates_are_checked_by_behavior_not_source_text() {
    let unsafe_targets = [
        "trade_and_other_current_receivables",
        "accounts_receivable_trade",
        "accounts_receivable_and_contract_assets",
        "tangible_fixed_assets",
        "cost_of_revenue",
    ];

    for local_name in [
        // IFRS finance lease receivables are not generic current receivables.
        "FinanceLeaseReceivablesAssetsIFRS",
        // This electric-power tag combines PPE and intangible assets.
        "PlantAndEquipmentAndIntangibleAssetsELE",
        // Reprocessing transfer cost is a cash-flow / nuclear-fuel-specific cost.
        "TransferCostOfReprocessingOfIrradiatedNuclearFuelOpeCFELE",
    ] {
        let concept = jppfs(local_name);
        if let TaxonomyConceptStatus::Semantic(canonical) = concept_status(&concept) {
            assert!(
                !unsafe_targets.contains(&canonical.item_name.as_str()),
                "{local_name} must not be semantically folded into {canonical:?}"
            );
        }
    }
}
