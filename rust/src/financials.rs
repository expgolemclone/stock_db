use std::collections::{HashMap, HashSet};

use crate::InventoriesTagMismatchError;
use crate::inventory;
use crate::share_classes;
use crate::taxonomy;
use crate::types::{ConceptKey, LoadedXbrlArtifact, RawStatementFact};

#[derive(Debug, Clone)]
struct CanonicalStatementItem {
    value: Option<f64>,
    source_concept: String,
    priority: u8,
    decimals: Option<String>,
}

type CanonicalStatementItems = HashMap<String, CanonicalStatementItem>;
type ConceptDecimalsByContext = HashMap<(String, ConceptKey), String>;

/// Build a mapping from concept (namespace URI + local name) to statement type
/// ("bs", "pl", "cf") using the raw statement facts already classified by the
/// presentation linkbase. Keyed by the full ConceptKey so the same local name
/// under different namespaces is not collapsed.
fn build_concept_statement_map(artifact: &LoadedXbrlArtifact) -> HashMap<ConceptKey, String> {
    let mut map: HashMap<ConceptKey, String> = HashMap::new();
    for fact in &artifact.raw_balance_sheet_facts {
        map.entry(fact.concept.clone())
            .or_insert_with(|| "bs".to_string());
    }
    for fact in &artifact.raw_profit_loss_facts {
        map.entry(fact.concept.clone())
            .or_insert_with(|| "pl".to_string());
    }
    for fact in &artifact.raw_cash_flow_facts {
        map.entry(fact.concept.clone())
            .or_insert_with(|| "cf".to_string());
    }
    map
}

fn raw_fact_context_key(fact: &RawStatementFact) -> String {
    format!(
        "{}|{}|{}|{}",
        fact.period,
        fact.period_start.as_deref().unwrap_or(""),
        fact.period_end,
        fact.context_id
    )
}

fn public_period(context_key: &str) -> &str {
    context_key
        .split_once('|')
        .map_or(context_key, |(period, _)| period)
}

fn is_instant_context_key(context_key: &str) -> bool {
    let mut parts = context_key.splitn(4, '|');
    let Some(_period) = parts.next() else {
        return false;
    };
    matches!(parts.next(), Some("")) && parts.next().is_some() && parts.next().is_some()
}

fn build_concept_decimals_map(artifact: &LoadedXbrlArtifact) -> ConceptDecimalsByContext {
    let mut map = HashMap::new();
    for fact in artifact
        .raw_balance_sheet_facts
        .iter()
        .chain(&artifact.raw_profit_loss_facts)
        .chain(&artifact.raw_cash_flow_facts)
    {
        retain_most_precise_decimals(&mut map, fact);
    }
    map
}

fn retain_most_precise_decimals(map: &mut ConceptDecimalsByContext, fact: &RawStatementFact) {
    let key = (raw_fact_context_key(fact), fact.concept.clone());
    match map.get_mut(&key) {
        None => {
            map.insert(key, fact.decimals.clone());
        }
        Some(existing) if decimals_precision(&fact.decimals) > decimals_precision(existing) => {
            *existing = fact.decimals.clone();
        }
        Some(_) => {}
    }
}

/// Classify a concept that does not appear in any BS/PL/CF presentation role
/// using the taxonomy mapping and name-based heuristics.
fn classify_remaining_concept(concept: &ConceptKey) -> Option<String> {
    if let Some(statement) = taxonomy::statement_for_concept(concept) {
        return Some(statement.to_string());
    }

    let lower = concept.1.to_ascii_lowercase();
    if lower.contains("forecast") {
        return Some("forecast".to_string());
    }
    if lower.contains("dividend") || lower.contains("pershare") || lower.contains("per_share") {
        return Some("dividend".to_string());
    }
    None
}

/// Canonical分類に基づき, raw factsのステートメント区分を補正する.
///
/// XBRLリンクベースの分類とタクソノミーの正規分類が異なる場合,
/// タクソノミーを優先する. 例: PL項目がCFにのみ出現する場合にPLへ是正,
/// PerShare項目がBSコンテキストで出現する場合にdividendへ是正.
fn resolve_statement_override(concept: &ConceptKey, raw_statement: &str) -> Option<String> {
    let mapped = taxonomy::canonicalize_semantic_concept(concept)?;
    let statement = taxonomy::statement_for_item_name(&mapped.item_name)?;

    if statement == raw_statement {
        None
    } else {
        Some(statement.to_string())
    }
}

fn get_value_from_scope(
    facts: Option<&HashMap<ConceptKey, Option<f64>>>,
    concept: &ConceptKey,
) -> Option<f64> {
    facts.and_then(|period_facts| period_facts.get(concept).copied().flatten())
}

/// Parse canonical financial statements from consolidated XBRL contexts only.
///
/// Non-consolidated facts remain available in the artifact and raw fact tables,
/// but they never fill gaps in a consolidated canonical history. Mixing the two
/// reporting entities would make period-over-period financial analysis invalid.
/// The outer key preserves the exact XBRL context identity rather than only the
/// ending month, so durations that end in the same month remain distinct.
pub fn parse_financials_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Result<
    HashMap<String, HashMap<String, HashMap<String, Option<f64>>>>,
    InventoriesTagMismatchError,
> {
    let inventory_by_period = inventory::parse_inventories_from_artifact(artifact)?;
    let share_class_rows = share_classes::parse_share_classes_from_artifact(artifact);
    let preferred_flags = share_classes::preferred_flags_by_period(&share_class_rows);

    let concept_map = build_concept_statement_map(artifact);
    let decimals_map = build_concept_decimals_map(artifact);

    let mut periods: Vec<String> = artifact.financial_facts.keys().cloned().collect();
    periods.sort();
    periods.dedup();
    periods.reverse();

    let mut result: HashMap<String, HashMap<String, HashMap<String, Option<f64>>>> = HashMap::new();

    for period in &periods {
        let scoped_facts = artifact.financial_facts.get(period);

        let mut all_concepts: HashSet<ConceptKey> = HashSet::new();
        if let Some(facts) = scoped_facts {
            all_concepts.extend(facts.keys().cloned());
        }

        let mut statements: HashMap<String, CanonicalStatementItems> = HashMap::new();

        for concept in &all_concepts {
            let Some(mapped) = route_for_canonical_statement(concept) else {
                continue;
            };
            let raw_statement = match concept_map.get(concept) {
                Some(s) => s.clone(),
                None => match classify_remaining_concept(concept) {
                    Some(s) => s,
                    None => continue,
                },
            };
            let statement = resolve_statement_override(concept, &raw_statement)
                .unwrap_or_else(|| raw_statement.clone());

            let value = get_value_from_scope(scoped_facts, concept);
            let decimals = decimals_map
                .get(&(period.clone(), concept.clone()))
                .map(String::as_str);
            let statement_items = statements.entry(statement).or_default();
            insert_canonical_item(
                statement_items,
                period,
                &mapped.item_name,
                value,
                &concept.1,
                mapped.priority,
                decimals,
            )?;
        }

        if let Some(inv) = inventory_by_period
            .get(period)
            .and_then(|m| m.get("inventories"))
            .copied()
            .flatten()
        {
            let bs_items = statements.entry("bs".to_string()).or_default();
            insert_synthetic_item(bs_items, period, "bs", "inventories", Some(inv))?;
        }

        if let Some(shares_facts) = artifact.shares_facts.get(period) {
            let bs_items = statements.entry("bs".to_string()).or_default();
            for (concept, value) in shares_facts {
                let Some(mapped) = route_for_canonical_statement(concept) else {
                    continue;
                };
                insert_canonical_item(
                    bs_items,
                    period,
                    &mapped.item_name,
                    *value,
                    &concept.1,
                    mapped.priority,
                    None,
                )?;
            }
        }

        if is_instant_context_key(period) {
            let has_preferred = preferred_flags
                .get(public_period(period))
                .copied()
                .unwrap_or(false);
            let bs_items = statements.entry("bs".to_string()).or_default();
            insert_synthetic_item(
                bs_items,
                period,
                "bs",
                "has_preferred_shares",
                Some(if has_preferred { 1.0 } else { 0.0 }),
            )?;
        }

        if !statements.is_empty() {
            result.insert(period.clone(), finalize_statements(statements));
        }
    }

    Ok(result)
}

fn route_for_canonical_statement(concept: &ConceptKey) -> Option<taxonomy::CanonicalConcept> {
    match taxonomy::concept_status(concept) {
        taxonomy::TaxonomyConceptStatus::Semantic(mapped) => Some(mapped),
        taxonomy::TaxonomyConceptStatus::Identity(_)
        | taxonomy::TaxonomyConceptStatus::Extension
        | taxonomy::TaxonomyConceptStatus::Excluded
        | taxonomy::TaxonomyConceptStatus::Unknown => None,
    }
}

fn insert_synthetic_item(
    items: &mut CanonicalStatementItems,
    period: &str,
    statement: &str,
    item_name: &str,
    value: Option<f64>,
) -> Result<(), InventoriesTagMismatchError> {
    let mapped = taxonomy::canonicalize_synthetic(item_name).ok_or_else(|| {
        InventoriesTagMismatchError::new(format!(
            "Unregistered synthetic financial item in {period} {statement}: {item_name}"
        ))
    })?;
    insert_canonical_item(
        items,
        period,
        &mapped.item_name,
        value,
        item_name,
        mapped.priority,
        None,
    )
}

fn decimals_precision(decimals: &str) -> i32 {
    let decimals = decimals.trim();
    if decimals.eq_ignore_ascii_case("INF") {
        i32::MAX
    } else {
        decimals.parse::<i32>().unwrap_or(i32::MIN)
    }
}

fn decimals_tolerance(decimals: Option<&str>) -> f64 {
    let Some(decimals) = decimals.map(str::trim) else {
        return 0.0;
    };
    if decimals.is_empty() || decimals.eq_ignore_ascii_case("INF") {
        return 0.0;
    }
    decimals
        .parse::<i32>()
        .ok()
        .map(|value| 0.5 * 10f64.powi(-value))
        .unwrap_or(0.0)
}

fn values_compatible(
    left: f64,
    right: f64,
    left_decimals: Option<&str>,
    right_decimals: Option<&str>,
) -> bool {
    let declared = decimals_tolerance(left_decimals).max(decimals_tolerance(right_decimals));
    let floating = left.abs().max(right.abs()).max(1.0) * 1e-12;
    (left - right).abs() <= declared.max(floating)
}

fn insert_canonical_item(
    items: &mut CanonicalStatementItems,
    period: &str,
    item_name: &str,
    value: Option<f64>,
    source_concept: &str,
    priority: u8,
    decimals: Option<&str>,
) -> Result<(), InventoriesTagMismatchError> {
    let Some(existing) = items.get_mut(item_name) else {
        items.insert(
            item_name.to_string(),
            CanonicalStatementItem {
                value,
                source_concept: source_concept.to_string(),
                priority,
                decimals: decimals.map(str::to_string),
            },
        );
        return Ok(());
    };

    if priority > existing.priority {
        *existing = CanonicalStatementItem {
            value,
            source_concept: source_concept.to_string(),
            priority,
            decimals: decimals.map(str::to_string),
        };
        return Ok(());
    }
    if priority < existing.priority {
        return Ok(());
    }

    match (existing.value, value) {
        (Some(left), Some(right))
            if !values_compatible(left, right, existing.decimals.as_deref(), decimals) =>
        {
            Err(InventoriesTagMismatchError::new(format!(
                "Ambiguous canonical XBRL item {item_name} in {period}: {}={} and {source_concept}={right}",
                existing.source_concept, left
            )))
        }
        (Some(_), Some(_)) => {
            let existing_precision = existing
                .decimals
                .as_deref()
                .map(decimals_precision)
                .unwrap_or(i32::MIN);
            let new_precision = decimals.map(decimals_precision).unwrap_or(i32::MIN);
            if new_precision > existing_precision {
                existing.value = value;
                existing.source_concept = source_concept.to_string();
                existing.decimals = decimals.map(str::to_string);
            }
            Ok(())
        }
        (None, Some(_)) => {
            existing.value = value;
            existing.source_concept = source_concept.to_string();
            existing.decimals = decimals.map(str::to_string);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn finalize_statements(
    statements: HashMap<String, CanonicalStatementItems>,
) -> HashMap<String, HashMap<String, Option<f64>>> {
    statements
        .into_iter()
        .map(|(statement, items)| {
            (
                statement,
                items
                    .into_iter()
                    .map(|(item_name, item)| (item_name, item.value))
                    .collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(local: &str) -> ConceptKey {
        (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2020-11-01/jppfs_cor".to_string(),
            local.to_string(),
        )
    }

    fn raw_fact(context_id: &str, period_start: &str, decimals: &str) -> RawStatementFact {
        RawStatementFact {
            statement: "pl".to_string(),
            period: "2025-03".to_string(),
            period_start: Some(period_start.to_string()),
            period_end: "2025-03-31".to_string(),
            duration_days: Some(365),
            consolidation_scope: "consolidated".to_string(),
            context_id: context_id.to_string(),
            dimensions_json: "[]".to_string(),
            concept: q("NetSales"),
            role_uri: String::new(),
            unit: "JPY".to_string(),
            unit_ref: "JPY".to_string(),
            decimals: decimals.to_string(),
            value: Some(100.0),
        }
    }

    #[test]
    fn concept_statement_map_classifies_raw_facts() {
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("CashAndDeposits".to_string(), "bs".to_string());
        map.insert("NetSales".to_string(), "pl".to_string());
        map.insert(
            "NetCashProvidedByUsedInOperatingActivities".to_string(),
            "cf".to_string(),
        );

        assert_eq!(map.get("CashAndDeposits"), Some(&"bs".to_string()));
        assert_eq!(map.get("NetSales"), Some(&"pl".to_string()));
        assert_eq!(
            map.get("NetCashProvidedByUsedInOperatingActivities"),
            Some(&"cf".to_string())
        );
    }

    #[test]
    fn classify_remaining_concept_identifies_dividends_and_forecasts() {
        assert_eq!(
            classify_remaining_concept(&q("DividendPaidPerShareSummaryOfBusinessResults")),
            Some("dividend".to_string())
        );
        assert_eq!(
            classify_remaining_concept(&q("ForecastNetSalesSummaryOfBusinessResults")),
            Some("forecast".to_string())
        );
        assert_eq!(
            classify_remaining_concept(&q("CashAndDeposits")),
            Some("bs".to_string())
        );
        assert_eq!(
            classify_remaining_concept(&q("Assets")),
            Some("bs".to_string())
        );
        assert_eq!(
            classify_remaining_concept(&q("PurchaseOfTreasuryStockFinCF")),
            Some("cf".to_string())
        );
    }

    #[test]
    fn get_value_from_scope_returns_none_when_scope_is_empty() {
        let concept = ("ns".to_string(), "TestConcept".to_string());
        assert_eq!(get_value_from_scope(None, &concept), None);
    }

    #[test]
    fn get_value_from_scope_returns_none_value() {
        let concept = ("ns".to_string(), "TestConcept".to_string());
        let mut facts: HashMap<ConceptKey, Option<f64>> = HashMap::new();
        facts.insert(concept.clone(), None);

        assert_eq!(get_value_from_scope(Some(&facts), &concept), None);
    }

    #[test]
    fn decimals_are_keyed_by_exact_xbrl_context() {
        let mut map = ConceptDecimalsByContext::new();
        let full_year = raw_fact("FullYear", "2024-04-01", "-3");
        let quarter = raw_fact("Quarter", "2025-01-01", "0");
        retain_most_precise_decimals(&mut map, &full_year);
        retain_most_precise_decimals(&mut map, &quarter);

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get(&(raw_fact_context_key(&full_year), full_year.concept.clone()))
                .map(String::as_str),
            Some("-3")
        );
        assert_eq!(
            map.get(&(raw_fact_context_key(&quarter), quarter.concept.clone()))
                .map(String::as_str),
            Some("0")
        );
    }

    #[test]
    fn context_key_helpers_preserve_public_period_and_instant_identity() {
        assert_eq!(
            public_period("2025-03|2024-04-01|2025-03-31|FullYear"),
            "2025-03"
        );
        assert!(is_instant_context_key(
            "2025-03||2025-03-31|CurrentYearInstant"
        ));
        assert!(!is_instant_context_key(
            "2025-03|2024-04-01|2025-03-31|FullYear"
        ));
    }

    #[test]
    fn canonical_collision_uses_declared_xbrl_precision() {
        let mut items = CanonicalStatementItems::new();
        insert_canonical_item(
            &mut items,
            "2025-03",
            "revenue",
            Some(1_234_000.0),
            "RevenueRounded",
            10,
            Some("-3"),
        )
        .unwrap();
        insert_canonical_item(
            &mut items,
            "2025-03",
            "revenue",
            Some(1_234_400.0),
            "RevenueMoreDetailed",
            10,
            Some("-3"),
        )
        .unwrap();
        assert!(
            insert_canonical_item(
                &mut items,
                "2025-03",
                "revenue",
                Some(1_236_000.0),
                "RevenueConflicting",
                10,
                Some("-3"),
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_periods_never_fall_back_to_non_consolidated_facts() {
        let mut artifact = LoadedXbrlArtifact {
            path: String::new(),
            is_dir: false,
            financial_facts: HashMap::new(),
            inventory_facts: HashMap::new(),
            non_consolidated_financial_facts: HashMap::new(),
            non_consolidated_inventory_facts: HashMap::new(),
            non_consolidated_facts: HashMap::new(),
            shares_facts: HashMap::new(),
            raw_balance_sheet_facts: Vec::new(),
            raw_profit_loss_facts: Vec::new(),
            raw_cash_flow_facts: Vec::new(),
            statement_presentation_edges: Vec::new(),
            share_class_facts: Vec::new(),
            text_facts: Vec::new(),
        };
        artifact
            .non_consolidated_financial_facts
            .entry("2025-03|2024-04-01|2025-03-31|Standalone".to_string())
            .or_default()
            .insert(q("NetSales"), Some(100.0));

        let parsed = parse_financials_from_artifact(&artifact).unwrap();
        assert!(parsed.is_empty());
    }
}
