use std::collections::HashMap;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::types::{CalculationEdge, ConceptKey, LoadedXbrlArtifact};
use crate::xml_util;

use crate::InventoriesTagMismatchError;

/// Parse inventories from a loaded XBRL artifact.
///
/// Follows the same priority chain as the Python parser:
///   direct total → calculation linkbase → presentation linkbase → component sum
pub fn parse_inventories_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Result<HashMap<String, HashMap<String, Option<f64>>>, InventoriesTagMismatchError> {
    let artifact_path = Path::new(&artifact.path);

    let (calc_graphs, pres_graphs) = if artifact.is_dir {
        let (by_path, by_basename) = build_concept_lookup(artifact_path);
        let calc = build_calculation_graphs(artifact_path, &by_path, &by_basename);
        let pres = build_presentation_graphs(artifact_path, &by_path, &by_basename);
        (calc, pres)
    } else {
        (HashMap::new(), HashMap::new())
    };

    let mut result = parse_inventories_from_facts(
        &artifact.inventory_facts,
        &calc_graphs,
        &pres_graphs,
    )?;
    let non_consolidated_result = parse_inventories_from_facts(
        &artifact.non_consolidated_inventory_facts,
        &calc_graphs,
        &pres_graphs,
    )?;

    for (period, items) in non_consolidated_result {
        result.entry(period).or_insert(items);
    }

    Ok(result)
}

fn parse_inventories_from_facts(
    facts_by_period: &HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    calc_graphs: &CalculationGraphs,
    pres_graphs: &PresentationGraphs,
) -> Result<HashMap<String, HashMap<String, Option<f64>>>, InventoriesTagMismatchError> {
    let mut periods: Vec<&String> = facts_by_period.keys().collect();
    periods.sort();
    periods.reverse();

    let mut result: HashMap<String, HashMap<String, Option<f64>>> = HashMap::new();
    for period in periods {
        let period_facts = match facts_by_period.get(period) {
            Some(f) => f,
            None => continue,
        };

        // 1. Direct total
        let direct = matching_concepts_unique(period_facts, xml_util::INVENTORY_TOTAL_TAGS, "direct inventory", period)?;
        if let Some(v) = direct {
            result.insert(period.clone(), HashMap::from([("inventories".to_string(), Some(v))]));
            continue;
        }

        // 2. Calculation linkbase
        let calc = calculation_candidates_unique(period_facts, &calc_graphs, period)?;
        if let Some(v) = calc {
            result.insert(period.clone(), HashMap::from([("inventories".to_string(), Some(v))]));
            continue;
        }

        // 3. Presentation linkbase
        let pres = presentation_candidates_unique(period_facts, &pres_graphs, period)?;
        if let Some(v) = pres {
            result.insert(period.clone(), HashMap::from([("inventories".to_string(), Some(v))]));
            continue;
        }

        // 4. Component sum
        let comp = component_sum(period_facts);
        if comp != 0.0 {
            result.insert(period.clone(), HashMap::from([("inventories".to_string(), Some(comp))]));
            continue;
        }

        // 5. Check for unknown inventory-like tags
        let unknown = unknown_inventory_like_tags(period_facts);
        if !unknown.is_empty() {
            let mut tags: Vec<String> = unknown.into_iter().collect();
            tags.sort();
            return Err(InventoriesTagMismatchError::new(format!(
                "Unknown inventory-like XBRL tags: {}",
                tags.join(", ")
            )));
        }
    }

    Ok(result)
}

// ── Concept lookup (XSD parsing) ──

type LookupByPath = HashMap<(PathBuf, String), ConceptKey>;
type LookupByBasename = HashMap<(String, String), ConceptKey>;

fn build_concept_lookup(artifact_root: &Path) -> (LookupByPath, LookupByBasename) {
    let mut by_path: LookupByPath = HashMap::new();
    let mut by_basename: LookupByBasename = HashMap::new();

    let mut xsd_paths: Vec<PathBuf> = Vec::new();
    collect_files_recursive(artifact_root, "xsd", &mut xsd_paths);
    xsd_paths.sort();

    for xsd_path in &xsd_paths {
        let content = match std::fs::read_to_string(xsd_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let target_ns = extract_target_namespace(&content);
        if target_ns.is_empty() {
            continue;
        }
        extract_xsd_elements(&content, xsd_path, &target_ns, &mut by_path, &mut by_basename);
    }

    (by_path, by_basename)
}

fn extract_target_namespace(content: &str) -> String {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local == "schema" {
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            if xml_util::local_name(&k) == "targetNamespace" {
                                return String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                }
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    String::new()
}

fn extract_xsd_elements(
    content: &str,
    xsd_path: &Path,
    target_ns: &str,
    by_path: &mut LookupByPath,
    by_basename: &mut LookupByBasename,
) {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local != "element" {
                    continue;
                }
                let mut name = String::new();
                let mut element_id = String::new();
                for attr in e.attributes() {
                    if let Ok(attr) = attr {
                        let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let v = String::from_utf8_lossy(&attr.value).to_string();
                        match xml_util::local_name(&k).as_str() {
                            "name" => name = v,
                            "id" => element_id = v,
                            _ => {}
                        }
                    }
                }
                if name.is_empty() {
                    continue;
                }
                let concept = (target_ns.to_string(), name);
                if !element_id.is_empty() {
                    let key = (xsd_path.to_path_buf().canonicalize().unwrap_or_else(|_| xsd_path.to_path_buf()), element_id.clone());
                    by_path.insert(key, concept.clone());
                    by_basename.insert((xsd_path.file_name().unwrap_or_default().to_string_lossy().to_string(), element_id), concept);
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

// ── Linkbase graph building ──

fn collect_files_recursive(root: &Path, ext: &str, result: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, ext, result);
            } else if path.extension().is_some_and(|e| e == ext) {
                result.push(path);
            }
        }
    }
}

fn resolve_locator_concept(
    href: &str,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
) -> Option<ConceptKey> {
    let (href_path, fragment) = if let Some(idx) = href.rfind('#') {
        (&href[..idx], &href[idx + 1..])
    } else {
        return None;
    };
    if fragment.is_empty() {
        return None;
    }

    if !href_path.is_empty() {
        // Check if it's a URL with scheme
        if href_path.contains("://") {
            let basename = Path::new(href_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            // URL-decode
            let decoded = urldecode(&basename);
            return by_basename
                .get(&(decoded, fragment.to_string()))
                .cloned();
        }
        let resolved = base_path
            .parent()
            .unwrap_or(base_path)
            .join(urldecode(href_path));
        let canonical = resolved.canonicalize().unwrap_or(resolved);
        if let Some(concept) = by_path.get(&(canonical.clone(), fragment.to_string())) {
            return Some(concept.clone());
        }
        let basename = canonical.file_name().unwrap_or_default().to_string_lossy().to_string();
        return by_basename
            .get(&(basename, fragment.to_string()))
            .cloned();
    }

    let canonical = base_path.canonicalize().unwrap_or_else(|_| base_path.to_path_buf());
    by_path.get(&(canonical, fragment.to_string())).cloned()
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_digit(h), hex_digit(l)) {
                    result.push(char::from(hv << 4 | lv));
                    continue;
                }
            }
            result.push('%');
        } else {
            result.push(char::from(b));
        }
    }
    result
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub type CalculationGraphs = HashMap<String, HashMap<ConceptKey, Vec<CalculationEdge>>>;
pub type PresentationGraphs = HashMap<String, HashMap<ConceptKey, std::collections::HashSet<ConceptKey>>>;

fn build_calculation_graphs(
    artifact_root: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
) -> CalculationGraphs {
    let mut graphs: CalculationGraphs = HashMap::new();
    let mut cal_paths: Vec<PathBuf> = Vec::new();
    collect_files_recursive(artifact_root, "xml", &mut cal_paths);
    cal_paths.retain(|p| p.file_name().is_some_and(|n| n.to_string_lossy().ends_with("_cal.xml")));
    cal_paths.sort();

    for cal_path in &cal_paths {
        let content = match std::fs::read_to_string(cal_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        parse_calculation_linkbase(&content, cal_path, by_path, by_basename, &mut graphs);
    }
    graphs
}

fn parse_calculation_linkbase(
    content: &str,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    graphs: &mut CalculationGraphs,
) {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut current_role = String::new();
    let mut locators: HashMap<String, ConceptKey> = HashMap::new();
    let mut in_calc_link = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);

                if local == "calculationLink" {
                    in_calc_link = true;
                    locators.clear();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("role") {
                                current_role = v;
                            }
                        }
                    }
                } else if in_calc_link && local == "loc" {
                    let mut label = String::new();
                    let mut href = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("label") {
                                label = v;
                            } else if k.ends_with("href") {
                                href = v;
                            }
                        }
                    }
                    if !label.is_empty() && !href.is_empty() {
                        if let Some(concept) =
                            resolve_locator_concept(&href, base_path, by_path, by_basename)
                        {
                            locators.insert(label, concept);
                        }
                    }
                } else if in_calc_link && local == "calculationArc" {
                    let mut from_label = String::new();
                    let mut to_label = String::new();
                    let mut weight = 1.0;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("from") {
                                from_label = v;
                            } else if k.ends_with("to") {
                                to_label = v;
                            } else if xml_util::local_name(&k) == "weight" {
                                weight = v.parse().unwrap_or(1.0);
                            }
                        }
                    }
                    if let (Some(from), Some(to)) =
                        (locators.get(&from_label), locators.get(&to_label))
                    {
                        graphs
                            .entry(current_role.clone())
                            .or_default()
                            .entry(from.clone())
                            .or_default()
                            .push(CalculationEdge {
                                child: to.clone(),
                                weight,
                            });
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local == "calculationLink" {
                    in_calc_link = false;
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);

                if in_calc_link && local == "loc" {
                    let mut label = String::new();
                    let mut href = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("label") {
                                label = v;
                            } else if k.ends_with("href") {
                                href = v;
                            }
                        }
                    }
                    if !label.is_empty() && !href.is_empty() {
                        if let Some(concept) =
                            resolve_locator_concept(&href, base_path, by_path, by_basename)
                        {
                            locators.insert(label, concept);
                        }
                    }
                } else if in_calc_link && local == "calculationArc" {
                    let mut from_label = String::new();
                    let mut to_label = String::new();
                    let mut weight = 1.0;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("from") {
                                from_label = v;
                            } else if k.ends_with("to") {
                                to_label = v;
                            } else if xml_util::local_name(&k) == "weight" {
                                weight = v.parse().unwrap_or(1.0);
                            }
                        }
                    }
                    if let (Some(from), Some(to)) =
                        (locators.get(&from_label), locators.get(&to_label))
                    {
                        graphs
                            .entry(current_role.clone())
                            .or_default()
                            .entry(from.clone())
                            .or_default()
                            .push(CalculationEdge {
                                child: to.clone(),
                                weight,
                            });
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

fn build_presentation_graphs(
    artifact_root: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
) -> PresentationGraphs {
    let mut graphs: PresentationGraphs = HashMap::new();
    let mut pre_paths: Vec<PathBuf> = Vec::new();
    collect_files_recursive(artifact_root, "xml", &mut pre_paths);
    pre_paths.retain(|p| p.file_name().is_some_and(|n| n.to_string_lossy().ends_with("_pre.xml")));
    pre_paths.sort();

    for pre_path in &pre_paths {
        let content = match std::fs::read_to_string(pre_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        parse_presentation_linkbase(&content, pre_path, by_path, by_basename, &mut graphs);
    }
    graphs
}

fn parse_presentation_linkbase(
    content: &str,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    graphs: &mut PresentationGraphs,
) {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut current_role = String::new();
    let mut locators: HashMap<String, ConceptKey> = HashMap::new();
    let mut in_pres_link = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);

                if local == "presentationLink" {
                    in_pres_link = true;
                    locators.clear();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("role") {
                                current_role = v;
                            }
                        }
                    }
                } else if in_pres_link && local == "loc" {
                    let mut label = String::new();
                    let mut href = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("label") {
                                label = v;
                            } else if k.ends_with("href") {
                                href = v;
                            }
                        }
                    }
                    if !label.is_empty() && !href.is_empty() {
                        if let Some(concept) =
                            resolve_locator_concept(&href, base_path, by_path, by_basename)
                        {
                            locators.insert(label, concept);
                        }
                    }
                } else if in_pres_link && local == "presentationArc" {
                    let mut from_label = String::new();
                    let mut to_label = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("from") {
                                from_label = v;
                            } else if k.ends_with("to") {
                                to_label = v;
                            }
                        }
                    }
                    if let (Some(from), Some(to)) =
                        (locators.get(&from_label), locators.get(&to_label))
                    {
                        graphs
                            .entry(current_role.clone())
                            .or_default()
                            .entry(from.clone())
                            .or_default()
                            .insert(to.clone());
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local == "presentationLink" {
                    in_pres_link = false;
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);

                if in_pres_link && local == "loc" {
                    let mut label = String::new();
                    let mut href = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("label") {
                                label = v;
                            } else if k.ends_with("href") {
                                href = v;
                            }
                        }
                    }
                    if !label.is_empty() && !href.is_empty() {
                        if let Some(concept) =
                            resolve_locator_concept(&href, base_path, by_path, by_basename)
                        {
                            locators.insert(label, concept);
                        }
                    }
                } else if in_pres_link && local == "presentationArc" {
                    let mut from_label = String::new();
                    let mut to_label = String::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let v = String::from_utf8_lossy(&attr.value).to_string();
                            if k.ends_with("from") {
                                from_label = v;
                            } else if k.ends_with("to") {
                                to_label = v;
                            }
                        }
                    }
                    if let (Some(from), Some(to)) =
                        (locators.get(&from_label), locators.get(&to_label))
                    {
                        graphs
                            .entry(current_role.clone())
                            .or_default()
                            .entry(from.clone())
                            .or_default()
                            .insert(to.clone());
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

// ── Inventory aggregation helpers ──

fn matching_concepts_unique(
    facts: &HashMap<ConceptKey, Option<f64>>,
    names: &[&str],
    label: &str,
    period: &str,
) -> Result<Option<f64>, InventoriesTagMismatchError> {
    let values: Vec<f64> = facts
        .iter()
        .filter(|(concept, value)| names.contains(&concept.1.as_str()) && value.is_some())
        .map(|(_, v)| v.unwrap())
        .collect();

    if values.is_empty() {
        return Ok(None);
    }
    let mut unique: Vec<f64> = values.clone();
    unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    if unique.len() > 1 {
        return Err(InventoriesTagMismatchError::new(format!(
            "Conflicting {} totals in {}: {:?}",
            label, period, unique
        )));
    }
    Ok(Some(unique[0]))
}

fn calculation_candidates_unique(
    facts: &HashMap<ConceptKey, Option<f64>>,
    graphs: &CalculationGraphs,
    period: &str,
) -> Result<Option<f64>, InventoriesTagMismatchError> {
    let candidates = calculation_candidates(facts, graphs);
    unique_candidate("calculation inventory", period, &candidates)
}

fn calculation_candidates(
    facts: &HashMap<ConceptKey, Option<f64>>,
    graphs: &CalculationGraphs,
) -> Vec<f64> {
    let mut candidates = Vec::new();

    for graph in graphs.values() {
        let all_concepts = all_graph_concepts_calculation(graph);
        let roots: Vec<&ConceptKey> = all_concepts
            .iter()
            .filter(|c| xml_util::is_inventory_total(&c.1))
            .collect();

        let mut memo: HashMap<ConceptKey, Option<f64>> = HashMap::new();
        for root in roots {
            if let Some(v) = evaluate_calculation(graph, root, facts, &mut memo, &mut std::collections::HashSet::new()) {
                candidates.push(v);
            }
        }
    }
    candidates
}

fn all_graph_concepts_calculation(
    graph: &HashMap<ConceptKey, Vec<CalculationEdge>>,
) -> std::collections::HashSet<ConceptKey> {
    let mut concepts: std::collections::HashSet<ConceptKey> = std::collections::HashSet::new();
    for (parent, edges) in graph {
        concepts.insert(parent.clone());
        for edge in edges {
            concepts.insert(edge.child.clone());
        }
    }
    concepts
}

fn evaluate_calculation(
    graph: &HashMap<ConceptKey, Vec<CalculationEdge>>,
    concept: &ConceptKey,
    facts: &HashMap<ConceptKey, Option<f64>>,
    memo: &mut HashMap<ConceptKey, Option<f64>>,
    visiting: &mut std::collections::HashSet<ConceptKey>,
) -> Option<f64> {
    if let Some(v) = memo.get(concept) {
        return *v;
    }
    if visiting.contains(concept) {
        return None;
    }
    visiting.insert(concept.clone());

    let mut child_values: Vec<f64> = Vec::new();
    for edge in graph.get(concept).into_iter().flatten() {
        if let Some(child_val) =
            evaluate_calculation(graph, &edge.child, facts, memo, visiting)
        {
            child_values.push(edge.weight * child_val);
        }
    }
    visiting.remove(concept);

    let result = if !child_values.is_empty() {
        Some(child_values.iter().sum())
    } else {
        facts.get(concept).copied().flatten()
    };
    memo.insert(concept.clone(), result);
    result
}

fn presentation_candidates_unique(
    facts: &HashMap<ConceptKey, Option<f64>>,
    graphs: &PresentationGraphs,
    period: &str,
) -> Result<Option<f64>, InventoriesTagMismatchError> {
    let candidates = presentation_candidates(facts, graphs);
    unique_candidate("presentation inventory", period, &candidates)
}

fn presentation_candidates(
    facts: &HashMap<ConceptKey, Option<f64>>,
    graphs: &PresentationGraphs,
) -> Vec<f64> {
    let mut candidates = Vec::new();

    for graph in graphs.values() {
        let all_concepts = all_graph_concepts_presentation(graph);
        let roots: Vec<&ConceptKey> = all_concepts
            .iter()
            .filter(|c| xml_util::is_inventory_total(&c.1))
            .collect();

        for root in roots {
            if let Some(v) = presentation_sum(graph, root, facts) {
                candidates.push(v);
            }
        }
    }
    candidates
}

fn all_graph_concepts_presentation(
    graph: &HashMap<ConceptKey, std::collections::HashSet<ConceptKey>>,
) -> std::collections::HashSet<ConceptKey> {
    let mut concepts: std::collections::HashSet<ConceptKey> = std::collections::HashSet::new();
    for (parent, children) in graph {
        concepts.insert(parent.clone());
        for child in children {
            concepts.insert(child.clone());
        }
    }
    concepts
}

fn presentation_sum(
    graph: &HashMap<ConceptKey, std::collections::HashSet<ConceptKey>>,
    root: &ConceptKey,
    facts: &HashMap<ConceptKey, Option<f64>>,
) -> Option<f64> {
    let mut desc_cache: HashMap<ConceptKey, std::collections::HashSet<ConceptKey>> = HashMap::new();
    let mut closure = std::collections::HashSet::new();
    closure.insert(root.clone());
    if let Some(desc) = reachable_descendants(graph, root, &mut desc_cache) {
        closure.extend(desc.iter().cloned());
    }

    let factful: std::collections::HashSet<ConceptKey> = closure
        .iter()
        .filter(|c| facts.get(*c).is_some_and(|v| v.is_some()))
        .cloned()
        .collect();

    if factful.is_empty() {
        return None;
    }

    let mut non_deepest: std::collections::HashSet<ConceptKey> = std::collections::HashSet::new();
    for concept in &factful {
        if let Some(desc) = reachable_descendants(graph, concept, &mut desc_cache) {
            for d in desc {
                if factful.contains(&d) {
                    non_deepest.insert(concept.clone());
                    break;
                }
            }
        }
    }

    let deepest = factful.difference(&non_deepest);
    let sum: f64 = deepest
        .filter_map(|c| facts.get(c).copied().flatten())
        .sum();
    Some(sum)
}

fn reachable_descendants(
    graph: &HashMap<ConceptKey, std::collections::HashSet<ConceptKey>>,
    concept: &ConceptKey,
    memo: &mut HashMap<ConceptKey, std::collections::HashSet<ConceptKey>>,
) -> Option<std::collections::HashSet<ConceptKey>> {
    if let Some(desc) = memo.get(concept) {
        return Some(desc.clone());
    }
    let mut descendants: std::collections::HashSet<ConceptKey> = std::collections::HashSet::new();
    if let Some(children) = graph.get(concept) {
        for child in children {
            descendants.insert(child.clone());
            if let Some(child_desc) = reachable_descendants(graph, child, memo) {
                descendants.extend(child_desc);
            }
        }
    }
    memo.insert(concept.clone(), descendants.clone());
    Some(descendants)
}

fn unique_candidate(
    label: &str,
    period: &str,
    values: &[f64],
) -> Result<Option<f64>, InventoriesTagMismatchError> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut unique: Vec<f64> = values.to_vec();
    unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    if unique.len() > 1 {
        return Err(InventoriesTagMismatchError::new(format!(
            "Conflicting {} totals in {}: {:?}",
            label, period, unique
        )));
    }
    Ok(Some(unique[0]))
}

fn component_sum(facts: &HashMap<ConceptKey, Option<f64>>) -> f64 {
    facts
        .iter()
        .filter(|(concept, value)| {
            xml_util::is_inventory_component(&concept.1) && value.is_some()
        })
        .map(|(_, v)| v.unwrap())
        .sum()
}

fn unknown_inventory_like_tags(facts: &HashMap<ConceptKey, Option<f64>>) -> std::collections::HashSet<String> {
    facts
        .iter()
        .filter(|(concept, value)| {
            value.is_some()
                && xml_util::is_inventory_like(&concept.1)
                && !xml_util::is_ignored_inventory_candidate(&concept.1)
                && !xml_util::is_inventory_total(&concept.1)
                && !xml_util::is_inventory_component(&concept.1)
        })
        .map(|(concept, _)| concept.1.clone())
        .collect()
}
