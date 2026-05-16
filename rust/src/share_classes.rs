use std::collections::HashMap;

use crate::types::{LoadedXbrlArtifact, ShareClassFact};

pub fn parse_share_classes_from_artifact(artifact: &LoadedXbrlArtifact) -> Vec<ShareClassFact> {
    let current_period = latest_financial_period(artifact);
    let mut by_key: HashMap<(String, String), ShareClassFact> = HashMap::new();

    for raw in &artifact.share_class_facts {
        let mut row = raw.clone();
        if row.source_kind == "classes_of_shares_axis" {
            if let Some(period) = current_period.as_ref() {
                row.period = period.clone();
            }
        }

        let key = (row.period.clone(), row.class_key.clone());
        let replace = by_key.get(&key).is_none_or(|existing| {
            row.fact_priority < existing.fact_priority
                || (row.fact_priority == existing.fact_priority
                    && row.source_kind < existing.source_kind)
        });
        if replace {
            by_key.insert(key, row);
        }
    }

    let mut rows: Vec<ShareClassFact> = by_key.into_values().collect();
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.class_name.cmp(&b.class_name))
            .then_with(|| a.class_key.cmp(&b.class_key))
    });
    rows
}

pub fn preferred_flags_by_period(rows: &[ShareClassFact]) -> HashMap<String, bool> {
    let mut flags: HashMap<String, bool> = HashMap::new();
    for row in rows {
        let flag = row.is_preferred && row.shares > 0.0;
        flags
            .entry(row.period.clone())
            .and_modify(|existing| *existing |= flag)
            .or_insert(flag);
    }
    flags
}

fn latest_financial_period(artifact: &LoadedXbrlArtifact) -> Option<String> {
    artifact
        .financial_facts
        .keys()
        .chain(artifact.non_consolidated_financial_facts.keys())
        .chain(artifact.inventory_facts.keys())
        .chain(artifact.non_consolidated_inventory_facts.keys())
        .max()
        .cloned()
}
