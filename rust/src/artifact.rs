use std::collections::HashMap;
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use rayon::prelude::*;

use crate::types::{
    ConceptKey, ContextInfo, ContextMode, ExplicitMember, LoadedXbrlArtifact, ShareClassFact,
    UnitKind, UnitMap,
};
use crate::xml_util;

/// Result of parsing a single XBRL document.
struct ParsedDocument {
    nsmap: HashMap<String, String>,
    contexts: HashMap<String, ContextInfo>,
    units: UnitMap,
    inline_facts: Vec<InlineFact>,
    instance_facts: Vec<InstanceFact>,
}

fn parse_document_parts(content: &str) -> ParsedDocument {
    let nsmap = extract_nsmap(content);
    let contexts = extract_contexts(content, &nsmap);
    let units = extract_units(content);
    let inline_facts = extract_inline_facts(content);
    let instance_facts = extract_instance_facts(content, &nsmap);
    ParsedDocument {
        nsmap,
        contexts,
        units,
        inline_facts,
        instance_facts,
    }
}

/// Load an XBRL artifact from a file path or directory.
pub fn load_xbrl_artifact(path: &str) -> Result<LoadedXbrlArtifact, String> {
    let artifact_path = Path::new(path);
    let is_dir = artifact_path.is_dir();

    let file_contents: Vec<String> = if artifact_path.is_file() {
        let ext = artifact_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.to_lowercase() != "xbrl" {
            return Err(format!("Not an XBRL file: {}", path));
        }
        vec![
            xml_util::read_file_content(artifact_path)
                .ok_or_else(|| format!("Cannot read file: {}", path))?,
        ]
    } else if artifact_path.is_dir() {
        let xbrl_dir = artifact_path.join("XBRL");
        if !xbrl_dir.is_dir() {
            // Legacy flat directory — no XBRL subdir, return empty
            return Ok(LoadedXbrlArtifact {
                path: path.to_string(),
                is_dir: true,
                financial_facts: HashMap::new(),
                inventory_facts: HashMap::new(),
                non_consolidated_financial_facts: HashMap::new(),
                non_consolidated_inventory_facts: HashMap::new(),
                non_consolidated_facts: HashMap::new(),
                shares_facts: HashMap::new(),
                share_class_facts: Vec::new(),
            });
        }
        // Prefer PublicDoc/*.xbrl as canonical fact document
        let public_doc = xbrl_dir.join("PublicDoc");
        let mut public_xbrl: Vec<String> = Vec::new();
        if public_doc.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&public_doc) {
                for entry in entries.flatten() {
                    let fp = entry.path();
                    if fp.extension().is_some_and(|e| e == "xbrl") {
                        if let Some(c) = xml_util::read_file_content(&fp) {
                            public_xbrl.push(c);
                        }
                    }
                }
            }
        }
        if !public_xbrl.is_empty() {
            public_xbrl
        } else {
            // Fallback: scan all fact documents with iXBRL validation
            let fact_paths = xml_util::iter_fact_document_paths(artifact_path);
            let mut contents = Vec::new();
            for fp in &fact_paths {
                let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "xbrl" {
                    if let Some(c) = xml_util::read_file_content(fp) {
                        contents.push(c);
                    }
                } else if let Some(c) = xml_util::read_file_content(fp)
                    && xml_util::is_valid_xbrl_text(&c)
                {
                    contents.push(c);
                }
            }
            contents
        }
    } else {
        return Err(format!("Path does not exist: {}", path));
    };

    if file_contents.is_empty() {
        return Err(format!("No parseable XBRL documents found in: {}", path));
    }

    // Parse all documents in parallel, then merge sequentially
    let parsed_docs: Vec<ParsedDocument> = file_contents
        .par_iter()
        .map(|content| parse_document_parts(content))
        .collect();

    let mut all_contexts: HashMap<String, ContextInfo> = HashMap::new();
    let mut all_units: UnitMap = HashMap::new();

    for doc in &parsed_docs {
        all_contexts.extend(doc.contexts.iter().map(|(k, v)| (k.clone(), v.clone())));
        all_units.extend(doc.units.iter().map(|(k, v)| (k.clone(), *v)));
    }

    // Store facts into buckets (sequential — shared HashMap writes)
    let mut financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut non_consolidated_financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut non_consolidated_inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut non_consolidated_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut shares_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut share_class_facts: Vec<ShareClassFact> = Vec::new();
    let label_map = load_label_map(artifact_path);

    for doc in &parsed_docs {
        for fact in &doc.inline_facts {
            let ctx = match fact
                .context_ref
                .as_ref()
                .and_then(|id| all_contexts.get(id))
            {
                Some(c) => c,
                None => continue,
            };
            if all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other)
                == UnitKind::Other
            {
                continue;
            }
            let concept = match xml_util::resolve_qname(&fact.name, &doc.nsmap, "") {
                Some(c) => c,
                None => continue,
            };
            let value = if fact.is_nil {
                None
            } else {
                xml_util::parse_xbrl_value(&fact.text_value, "", &fact.scale, &fact.sign)
            };
            let unit_kind = all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other);
            store_fact_buckets(
                ctx,
                concept,
                value,
                unit_kind,
                &mut financial_facts,
                &mut inventory_facts,
                &mut non_consolidated_financial_facts,
                &mut non_consolidated_inventory_facts,
                &mut non_consolidated_facts,
                &mut shares_facts,
                &mut share_class_facts,
                &label_map,
            )?;
        }

        for fact in &doc.instance_facts {
            let ctx = match fact
                .context_ref
                .as_ref()
                .and_then(|id| all_contexts.get(id))
            {
                Some(c) => c,
                None => continue,
            };
            if all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other)
                == UnitKind::Other
            {
                continue;
            }
            let concept = match xml_util::resolve_qname(&fact.tag, &doc.nsmap, "") {
                Some(c) => c,
                None => continue,
            };
            let value = if fact.is_nil {
                None
            } else {
                xml_util::parse_xbrl_value(
                    &fact.text_value,
                    &fact.decimals,
                    &fact.scale,
                    &fact.sign,
                )
            };
            let unit_kind = all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other);
            store_fact_buckets(
                ctx,
                concept,
                value,
                unit_kind,
                &mut financial_facts,
                &mut inventory_facts,
                &mut non_consolidated_financial_facts,
                &mut non_consolidated_inventory_facts,
                &mut non_consolidated_facts,
                &mut shares_facts,
                &mut share_class_facts,
                &label_map,
            )?;
        }
    }

    Ok(LoadedXbrlArtifact {
        path: path.to_string(),
        is_dir,
        financial_facts,
        inventory_facts,
        non_consolidated_financial_facts,
        non_consolidated_inventory_facts,
        non_consolidated_facts,
        shares_facts,
        share_class_facts,
    })
}

// ── Data structures ──

struct InlineFact {
    name: String,
    context_ref: Option<String>,
    unit_ref: String,
    scale: String,
    sign: String,
    is_nil: bool,
    text_value: String,
}

struct InstanceFact {
    tag: String,
    context_ref: Option<String>,
    unit_ref: String,
    decimals: String,
    scale: String,
    sign: String,
    is_nil: bool,
    text_value: String,
}

// ── Bucket storage ──

fn store_concept_value(
    bucket: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    period: &str,
    concept: ConceptKey,
    value: Option<f64>,
) -> Result<(), String> {
    let by_period = bucket.entry(period.to_string()).or_default();
    match by_period.get(&concept) {
        None => {
            by_period.insert(concept, value);
        }
        Some(existing) => {
            if let Some(e) = existing {
                if let Some(v) = value {
                    if (*e - v).abs() > f64::EPSILON {
                        return Err(format!(
                            "Conflicting values for XBRL concept {} in {}: {} vs {}",
                            concept.1, period, e, v
                        ));
                    }
                }
            } else {
                by_period.insert(concept, value);
            }
        }
    }
    Ok(())
}

fn store_fact_buckets(
    ctx: &ContextInfo,
    concept: ConceptKey,
    value: Option<f64>,
    unit_kind: UnitKind,
    financial_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    inventory_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_financial_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_inventory_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    shares_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    share_class_facts: &mut Vec<ShareClassFact>,
    label_map: &HashMap<String, String>,
) -> Result<(), String> {
    let period = match &ctx.period {
        Some(p) => p.clone(),
        None => return Ok(()),
    };

    if unit_kind == UnitKind::Shares {
        if let Some(row) = build_share_class_fact(ctx, &concept, value, label_map) {
            share_class_facts.push(row);
        }
        if xml_util::is_relevant_shares_fact(&concept.1)
            && xml_util::should_use_context(ctx, ContextMode::Instant)
        {
            store_concept_value(shares_facts, &period, concept, value)?;
        }
        return Ok(());
    }

    if xml_util::should_use_context(ctx, ContextMode::Financial) {
        store_concept_value(financial_facts, &period, concept.clone(), value)?;
    }
    if xml_util::should_use_context(ctx, ContextMode::Instant) {
        store_concept_value(inventory_facts, &period, concept.clone(), value)?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedFinancial) {
        store_concept_value(
            non_consolidated_financial_facts,
            &period,
            concept.clone(),
            value,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedInstant) {
        store_concept_value(
            non_consolidated_inventory_facts,
            &period,
            concept.clone(),
            value,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedDividend) {
        store_concept_value(non_consolidated_facts, &period, concept, value)?;
    }
    Ok(())
}

fn build_share_class_fact(
    ctx: &ContextInfo,
    concept: &ConceptKey,
    value: Option<f64>,
    label_map: &HashMap<String, String>,
) -> Option<ShareClassFact> {
    let shares = value?;
    let period = ctx.period.clone()?;

    if let Some(priority) = xml_util::issued_shares_fact_priority(&concept.1) {
        if let Some(member) = share_class_member(ctx) {
            if xml_util::is_total_share_class_member(&member.member.1) {
                return None;
            }
            let label = label_map.get(&member.member.1).map(String::as_str);
            let class_name = xml_util::share_class_name_from_member(&member.member.1, label);
            let is_preferred = xml_util::is_preferred_share_class(&member.member.1, &class_name);
            return Some(ShareClassFact {
                period,
                class_key: xml_util::concept_key_string(&member.member),
                class_name,
                shares,
                is_preferred,
                source_kind: "classes_of_shares_axis".to_string(),
                fact_priority: priority,
            });
        }
    }

    let label = label_map.get(&concept.1).map(String::as_str);
    if xml_util::is_class_specific_issued_shares_concept(&concept.1, label) {
        let class_name = xml_util::share_class_name_from_concept(&concept.1, label);
        let is_preferred = xml_util::is_preferred_share_class(&concept.1, &class_name);
        return Some(ShareClassFact {
            period,
            class_key: xml_util::concept_key_string(concept),
            class_name,
            shares,
            is_preferred,
            source_kind: "class_specific_concept".to_string(),
            fact_priority: 0,
        });
    }

    None
}

fn share_class_member(ctx: &ContextInfo) -> Option<&ExplicitMember> {
    ctx.explicit_members
        .iter()
        .find(|member| member.dimension.1 == "ClassesOfSharesAxis")
}

// ── XML extraction passes ──

fn qname_str(q: QName<'_>) -> String {
    String::from_utf8_lossy(q.as_ref()).to_string()
}

fn attr_str(
    attrs: &mut quick_xml::events::attributes::Attributes,
    target_local: &str,
) -> Option<String> {
    for attr in attrs {
        if let Ok(attr) = attr {
            let key = qname_str(attr.key);
            if xml_util::local_name(&key) == target_local {
                return Some(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
    }
    None
}

/// Extract namespace map from root element.
fn extract_nsmap(content: &str) -> HashMap<String, String> {
    let mut nsmap: HashMap<String, String> = HashMap::new();
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                for attr in e.attributes() {
                    if let Ok(attr) = attr {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if let Some(prefix) = key.strip_prefix("xmlns:") {
                            nsmap.insert(prefix.to_string(), val);
                        } else if key == "xmlns" {
                            nsmap.insert(String::new(), val);
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
    nsmap
}

#[derive(Clone)]
struct LabelResource {
    text: String,
    role: String,
    lang: String,
}

fn load_label_map(artifact_path: &Path) -> HashMap<String, String> {
    let mut paths = Vec::new();
    if artifact_path.is_file() {
        if let Some(parent) = artifact_path.parent() {
            collect_label_paths(parent, &mut paths);
        }
    } else {
        let public_doc = artifact_path.join("XBRL").join("PublicDoc");
        if public_doc.is_dir() {
            collect_label_paths(&public_doc, &mut paths);
        }
    }

    let mut labels: HashMap<String, String> = HashMap::new();
    for path in paths {
        if let Some(content) = xml_util::read_file_content(&path) {
            for (key, value) in extract_label_map(&content) {
                let replace = labels.get(&key).is_none_or(|existing| {
                    !contains_japanese(existing) && contains_japanese(&value)
                });
                if replace {
                    labels.insert(key, value);
                }
            }
        }
    }
    labels
}

fn collect_label_paths(dir: &Path, result: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_label_paths(&path, result);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("_lab") && name.ends_with(".xml"))
            {
                result.push(path);
            }
        }
    }
}

fn extract_label_map(content: &str) -> HashMap<String, String> {
    let mut locs: HashMap<String, String> = HashMap::new();
    let mut resources: HashMap<String, LabelResource> = HashMap::new();
    let mut arcs: Vec<(String, String)> = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local == "label" {
                    let attrs = collect_attrs(&mut e.attributes());
                    let text = read_text_until_end(&mut reader, "label");
                    if let Some(label_id) = attrs.get("label") {
                        resources.insert(
                            label_id.clone(),
                            LabelResource {
                                text,
                                role: attrs.get("role").cloned().unwrap_or_default(),
                                lang: attrs.get("lang").cloned().unwrap_or_default(),
                            },
                        );
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                let attrs = collect_attrs(&mut e.attributes());
                if local == "loc" {
                    if let (Some(label_id), Some(href)) = (attrs.get("label"), attrs.get("href")) {
                        if let Some((_, fragment)) = href.rsplit_once('#') {
                            locs.insert(label_id.clone(), fragment.to_string());
                        }
                    }
                } else if local == "labelArc" {
                    if let (Some(from), Some(to)) = (attrs.get("from"), attrs.get("to")) {
                        arcs.push((from.clone(), to.clone()));
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    let mut scored: HashMap<String, (u8, String)> = HashMap::new();
    for (from, to) in arcs {
        let Some(concept) = locs.get(&from) else {
            continue;
        };
        let Some(resource) = resources.get(&to) else {
            continue;
        };
        let score = label_score(resource);
        let replace = scored
            .get(concept)
            .is_none_or(|(existing_score, _)| score < *existing_score);
        if replace {
            scored.insert(concept.clone(), (score, resource.text.clone()));
        }
    }

    let mut labels = HashMap::new();
    for (concept, (_, text)) in scored {
        if let Some((_, local)) = concept.rsplit_once('_') {
            labels.insert(local.to_string(), text.clone());
        }
        labels.insert(concept, text);
    }
    labels
}

fn label_score(resource: &LabelResource) -> u8 {
    let is_standard = resource.role.ends_with("/label");
    let is_ja = resource.lang == "ja";
    match (is_ja, is_standard) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
}

fn contains_japanese(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c))
}

/// Extract all context definitions using event-based parsing.
fn extract_contexts(
    content: &str,
    nsmap: &HashMap<String, String>,
) -> HashMap<String, ContextInfo> {
    let mut contexts: HashMap<String, ContextInfo> = HashMap::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    // State for context parsing
    let mut in_context = false;
    let mut context_id = String::new();
    let mut instant_text: Option<String> = None;
    let mut end_date_text: Option<String> = None;
    let mut is_instant = false;
    let mut has_dimensions = false;
    let mut is_non_consolidated = false;
    let mut dimension_count: usize = 0;
    let mut explicit_members: Vec<ExplicitMember> = Vec::new();
    let mut text_buf = String::new();
    let mut collect_text_for: Option<String> = None; // local name we're collecting text for

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);

                if local == "context" && !in_context {
                    in_context = true;
                    context_id = String::new();
                    instant_text = None;
                    end_date_text = None;
                    is_instant = false;
                    has_dimensions = false;
                    is_non_consolidated = false;
                    dimension_count = 0;
                    explicit_members.clear();
                    text_buf.clear();
                    collect_text_for = None;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let k = xml_util::local_name(
                                &String::from_utf8_lossy(attr.key.as_ref()).to_string(),
                            );
                            if k == "id" {
                                context_id = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                } else if in_context {
                    match local.as_str() {
                        "instant" => {
                            collect_text_for = Some("instant".to_string());
                            text_buf.clear();
                        }
                        "endDate" => {
                            collect_text_for = Some("endDate".to_string());
                            text_buf.clear();
                        }
                        "explicitMember" => {
                            has_dimensions = true;
                            dimension_count += 1;
                            let mut dimension_qname = String::new();
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    let k = xml_util::local_name(
                                        &String::from_utf8_lossy(attr.key.as_ref()).to_string(),
                                    );
                                    if k == "dimension" {
                                        dimension_qname =
                                            String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                }
                            }
                            // Read the text content for member name
                            let mut member_qname = String::new();
                            let mut member_buf = Vec::new();
                            loop {
                                match reader.read_event_into(&mut member_buf) {
                                    Ok(Event::Text(t)) => {
                                        member_qname =
                                            t.unescape().unwrap_or_default().trim().to_string();
                                        let dimension_name = xml_util::local_name(&dimension_qname);
                                        let member_name = xml_util::local_name(&member_qname);
                                        if member_name == "NonConsolidatedMember"
                                            || (dimension_name
                                                == "ConsolidatedOrNonConsolidatedAxis"
                                                && member_name == "NonConsolidatedMember")
                                        {
                                            is_non_consolidated = true;
                                        }
                                    }
                                    Ok(Event::End(_)) => break,
                                    Ok(Event::Eof) => break,
                                    _ => {}
                                }
                                member_buf.clear();
                            }
                            if let (Some(dimension), Some(member)) = (
                                xml_util::resolve_qname(&dimension_qname, nsmap, ""),
                                xml_util::resolve_qname(&member_qname, nsmap, ""),
                            ) {
                                explicit_members.push(ExplicitMember { dimension, member });
                            }
                        }
                        "typedMember" => {
                            has_dimensions = true;
                            dimension_count += 1;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);
                if local == "explicitMember" && in_context {
                    has_dimensions = true;
                    dimension_count += 1;
                }
            }
            Ok(Event::Text(t)) => {
                if collect_text_for.is_some() {
                    text_buf.push_str(&t.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local == "context" && in_context {
                    // Finalize context
                    if let Some(ref target) = collect_text_for {
                        match target.as_str() {
                            "instant" => {
                                let val = text_buf.trim().to_string();
                                if !val.is_empty() {
                                    instant_text = Some(val);
                                    is_instant = true;
                                }
                            }
                            "endDate" => {
                                let val = text_buf.trim().to_string();
                                if !val.is_empty() {
                                    end_date_text = Some(val);
                                }
                            }
                            _ => {}
                        }
                    }
                    collect_text_for = None;

                    let period = xml_util::context_period_from_instant(instant_text.as_deref())
                        .or_else(|| {
                            end_date_text.as_ref().and_then(|d| {
                                if d.len() >= 7 {
                                    Some(d[..7].to_string())
                                } else {
                                    None
                                }
                            })
                        });

                    if !context_id.is_empty() {
                        contexts.insert(
                            context_id.clone(),
                            ContextInfo {
                                period,
                                _instant: instant_text.clone(),
                                is_instant,
                                has_dimensions,
                                is_non_consolidated,
                                dimension_count,
                                explicit_members: explicit_members.clone(),
                            },
                        );
                    }
                    in_context = false;
                } else if in_context {
                    if let Some(ref target) = collect_text_for {
                        if local == *target {
                            match target.as_str() {
                                "instant" => {
                                    let val = text_buf.trim().to_string();
                                    if !val.is_empty() {
                                        instant_text = Some(val);
                                        is_instant = true;
                                    }
                                }
                                "endDate" => {
                                    let val = text_buf.trim().to_string();
                                    if !val.is_empty() {
                                        end_date_text = Some(val);
                                    }
                                }
                                _ => {}
                            }
                            collect_text_for = None;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    contexts
}

/// Extract all unit definitions using event-based parsing.
fn extract_units(content: &str) -> UnitMap {
    let mut units: UnitMap = HashMap::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local != "unit" {
                    continue;
                }
                let unit_id = attr_str(&mut e.attributes(), "id");
                if unit_id.is_none() {
                    // Still need to consume the element
                    if matches!(reader.read_event_into(&mut buf), Ok(Event::Start(_))) {
                        read_text_until_end(&mut reader, "unit");
                    }
                    continue;
                }
                // Determine unit kind from measure text inside the unit element.
                // JPY-per-share units contain both JPY and shares; those are
                // financial per-share values, not pure share counts.
                let mut inner_buf = Vec::new();
                let mut inner_depth: usize = 0;
                let mut has_jpy = false;
                let mut has_shares = false;
                loop {
                    match reader.read_event_into(&mut inner_buf) {
                        Ok(Event::Text(t)) => {
                            let text = t.unescape().unwrap_or_default();
                            let measure = xml_util::local_name(text.trim());
                            if measure == "JPY" {
                                has_jpy = true;
                            } else if measure == "shares" {
                                has_shares = true;
                            }
                        }
                        Ok(Event::Start(_)) => inner_depth += 1,
                        Ok(Event::End(ee)) => {
                            if inner_depth == 0
                                && xml_util::local_name(&qname_str(ee.name())) == "unit"
                            {
                                break;
                            }
                            inner_depth = inner_depth.saturating_sub(1);
                        }
                        Ok(Event::Eof) => break,
                        _ => {}
                    }
                    inner_buf.clear();
                }
                let kind = if has_jpy {
                    UnitKind::JPY
                } else if has_shares {
                    UnitKind::Shares
                } else {
                    UnitKind::Other
                };
                units.insert(unit_id.unwrap(), kind);
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    units
}

/// Extract inline XBRL nonfraction facts.
fn extract_inline_facts(content: &str) -> Vec<InlineFact> {
    let mut facts: Vec<InlineFact> = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local != "nonfraction" {
                    continue;
                }
                facts.push(parse_inline_fact_start(&mut e.attributes(), &mut reader));
            }
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local != "nonfraction" {
                    continue;
                }
                facts.push(parse_inline_fact_empty(&mut e.attributes()));
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    facts
}

/// Collect all attributes from an element into a HashMap (local_name → value).
fn collect_attrs(attrs: &mut quick_xml::events::attributes::Attributes) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for attr in attrs {
        if let Ok(attr) = attr {
            let k = xml_util::local_name(&String::from_utf8_lossy(attr.key.as_ref()).to_string());
            let v = String::from_utf8_lossy(&attr.value).to_string();
            map.insert(k.to_lowercase(), v);
        }
    }
    map
}

fn parse_inline_fact_start(
    attrs: &mut quick_xml::events::attributes::Attributes,
    reader: &mut Reader<&[u8]>,
) -> InlineFact {
    let a = collect_attrs(attrs);
    let name = a.get("name").cloned().unwrap_or_default();
    let context_ref = a.get("contextref").cloned();
    let unit_ref = a.get("unitref").cloned().unwrap_or_default();
    let scale = a.get("scale").cloned().unwrap_or_default();
    let sign = a.get("sign").cloned().unwrap_or_default();
    let is_nil = a.get("nil").is_some_and(|v| v.to_lowercase() == "true");

    let text_value = if is_nil {
        String::new()
    } else {
        read_text_until_end(reader, "nonfraction")
    };

    InlineFact {
        name,
        context_ref,
        unit_ref,
        scale,
        sign,
        is_nil,
        text_value,
    }
}

fn parse_inline_fact_empty(attrs: &mut quick_xml::events::attributes::Attributes) -> InlineFact {
    let a = collect_attrs(attrs);
    let name = a.get("name").cloned().unwrap_or_default();
    let context_ref = a.get("contextref").cloned();
    let unit_ref = a.get("unitref").cloned().unwrap_or_default();
    let scale = a.get("scale").cloned().unwrap_or_default();
    let sign = a.get("sign").cloned().unwrap_or_default();
    let is_nil = a.get("nil").is_some_and(|v| v.to_lowercase() == "true");

    InlineFact {
        name,
        context_ref,
        unit_ref,
        scale,
        sign,
        is_nil,
        text_value: String::new(),
    }
}

fn read_text_until_end(reader: &mut Reader<&[u8]>, end_tag: &str) -> String {
    let mut text = String::new();
    let mut depth: usize = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                text.push_str(&t.unescape().unwrap_or_default());
            }
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(e)) => {
                if depth == 0 && xml_util::local_name(&qname_str(e.name())) == end_tag {
                    break;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    text.trim().to_string()
}

/// Extract instance document facts.
fn extract_instance_facts(content: &str, nsmap: &HashMap<String, String>) -> Vec<InstanceFact> {
    let mut facts: Vec<InstanceFact> = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);
                let ns = xml_util::namespace_uri(&tag);

                // Skip infrastructure elements
                if local == "context"
                    || local == "unit"
                    || local == "nonfraction"
                    || local == "schema"
                    || local == "linkbase"
                    || local == "calculationLink"
                    || local == "presentationLink"
                    || local == "loc"
                    || local == "calculationArc"
                    || local == "presentationArc"
                    || local == "references"
                    || local == "hidden"
                    || xml_util::IGNORED_FACT_NAMESPACES.contains(&ns)
                {
                    // Skip but still consume the element
                    if local != "context" && local != "unit" && local != "nonfraction" {
                        read_text_until_end(&mut reader, &local);
                    }
                    continue;
                }

                let context_ref = attr_str(&mut e.attributes(), "contextRef");
                if context_ref.is_none() {
                    continue;
                }

                let unit_ref = attr_str(&mut e.attributes(), "unitRef").unwrap_or_default();
                let decimals = attr_str(&mut e.attributes(), "decimals").unwrap_or_default();
                let scale = attr_str(&mut e.attributes(), "scale").unwrap_or_default();
                let sign = attr_str(&mut e.attributes(), "sign").unwrap_or_default();
                let nil_val = attr_str(&mut e.attributes(), "nil").unwrap_or_default();
                let is_nil = nil_val.to_lowercase() == "true";

                let text_value = if is_nil {
                    read_text_until_end(&mut reader, &local);
                    String::new()
                } else {
                    read_text_until_end(&mut reader, &local)
                };

                facts.push(InstanceFact {
                    tag,
                    context_ref,
                    unit_ref,
                    decimals,
                    scale,
                    sign,
                    is_nil,
                    text_value,
                });
            }
            Ok(Event::Empty(e)) => {
                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);
                let ns = xml_util::namespace_uri(&tag);

                if local == "context"
                    || local == "unit"
                    || local == "nonfraction"
                    || local == "loc"
                    || xml_util::IGNORED_FACT_NAMESPACES.contains(&ns)
                {
                    continue;
                }

                let attrs = collect_attrs(&mut e.attributes());
                let context_ref = attrs.get("contextref").cloned();
                if context_ref.is_none() {
                    continue;
                }

                let unit_ref = attrs.get("unitref").cloned().unwrap_or_default();
                let decimals = attrs.get("decimals").cloned().unwrap_or_default();
                let scale = attrs.get("scale").cloned().unwrap_or_default();
                let sign = attrs.get("sign").cloned().unwrap_or_default();
                let is_nil = attrs.get("nil").is_some_and(|v| v.to_lowercase() == "true");

                facts.push(InstanceFact {
                    tag,
                    context_ref,
                    unit_ref,
                    decimals,
                    scale,
                    sign,
                    is_nil,
                    text_value: String::new(),
                });
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    // Suppress unused variable warning
    let _ = nsmap;
    facts
}
