#![allow(dead_code)]

use std::collections::HashMap;

type ConceptKey = (String, String);
type SharesFactsByPeriod = HashMap<String, HashMap<ConceptKey, Option<f64>>>;

fn shares_facts_for_period<'a>(
    shares_facts: &'a SharesFactsByPeriod,
    period: &str,
) -> Option<&'a HashMap<ConceptKey, Option<f64>>> {
    shares_facts.get(period)
}

fn shares_concept() -> ConceptKey {
    (
        "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
        "NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc".to_string(),
    )
}

#[test]
fn shares_outstanding_facts_are_period_scoped() {
    let concept = shares_concept();
    let mut by_period: SharesFactsByPeriod = HashMap::new();

    by_period.insert(
        "2024-03".to_string(),
        HashMap::from([(concept.clone(), Some(100.0))]),
    );
    by_period.insert(
        "2025-03".to_string(),
        HashMap::from([(concept.clone(), Some(125.0))]),
    );

    let current = shares_facts_for_period(&by_period, "2025-03").unwrap();
    assert_eq!(current.get(&concept), Some(&Some(125.0)));
    assert!(shares_facts_for_period(&by_period, "2023-03").is_none());
}
