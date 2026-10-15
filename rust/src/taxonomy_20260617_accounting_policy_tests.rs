use crate::taxonomy::{canonicalize_concept, statement_for_concept};

fn jppfs(local_name: &str) -> (String, String) {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        local_name.to_string(),
    )
}

fn assert_pl_mapping(local_name: &str, expected_field: &str) {
    let concept = jppfs(local_name);
    let item_name = canonicalize_concept(&concept).unwrap().item_name;
    assert_eq!(item_name, expected_field, "{local_name} mapping changed");
    assert_eq!(statement_for_concept(&concept), Some("pl"));
}

#[test]
fn generic_profit_tags_and_industry_top_line_income_stay_separate() {
    for (local_name, expected_field) in [
        ("OperatingIncome", "operating_income"),
        ("OrdinaryIncome", "ordinary_income"),
        ("OrdinaryIncomeBNK", "revenue"),
        ("OrdinaryIncomeGAS", "revenue"),
        ("OrdinaryIncomeELE", "revenue"),
        ("OperatingIncomeINS", "revenue"),
    ] {
        assert_pl_mapping(local_name, expected_field);
    }
}
