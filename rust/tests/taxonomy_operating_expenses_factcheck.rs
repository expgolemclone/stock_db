#![allow(dead_code, unused_imports)]

#[path = "../src/types.rs"]
mod types;

#[path = "../src/taxonomy_operating_expenses_reviewed_aliases.rs"]
mod taxonomy;

use taxonomy::{
    StatementCategory, canonicalize_concept, classify_item_name, statement_for_concept,
};

fn jppfs(local: &str) -> types::ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2026-11-01/jppfs_cor".to_string(),
        local.to_string(),
    )
}

fn item_name(concept: types::ConceptKey) -> String {
    canonicalize_concept(&concept)
        .unwrap_or_else(|| panic!("missing canonical mapping for {:?}", concept))
        .item_name
}

#[test]
fn operating_expense_aliases_are_pl_expenses_not_cost_of_revenue() {
    for local_name in [
        "OperatingExpensesTotalRWY",
        "OperatingExpensesTransportationOERWY",
        "ShippingBusinessExpensesAndOtherOperatingExpensesWAT",
        "TransportationOperatingExpensesOEAutomobile",
        "WarehouseBusinessExpensesOtherBusinessRevenueWAT",
    ] {
        let concept = jppfs(local_name);
        let resolved = item_name(concept.clone());

        assert_eq!(resolved, "operating_expenses");
        assert_eq!(statement_for_concept(&concept), Some("pl"));
        assert_eq!(
            classify_item_name(&resolved),
            Some(StatementCategory::ProfitAndLoss)
        );
        assert_ne!(resolved, "cost_of_revenue");
        assert_ne!(resolved, "selling_general_admin");
        assert_ne!(resolved, "operating_expenses_ins");
    }
}
