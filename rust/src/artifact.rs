use std::collections::HashMap;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use rayon::prelude::*;

use crate::types::{ConceptKey, ContextInfo, ContextMode, LoadedXbrlArtifact, UnitKind, UnitMap};
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
    let contexts = extract_contexts(content);
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
        vec![xml_util::read_file_content(artifact_path)
            .ok_or_else(|| format!("Cannot read file: {}", path))?]
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
            });
        }
        let fact_paths = xml_util::iter_fact_document_paths(artifact_path);
        let mut contents = Vec::new();
        for fp in &fact_paths {
            let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "xbrl" {
                if let Some(c) = xml_util::read_file_content(fp) {
                    contents.push(c);
                }
            } else if let Some(c) = xml_util::read_file_content(fp) {
                if xml_util::is_valid_xbrl_text(&c) {
                    contents.push(c);
                }
            }
        }
        contents
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

    for doc in &parsed_docs {
        for fact in &doc.inline_facts {
            let ctx = match fact.context_ref.as_ref().and_then(|id| all_contexts.get(id)) {
                Some(c) => c,
                None => continue,
            };
            if all_units.get(&fact.unit_ref).copied().unwrap_or(UnitKind::Other) == UnitKind::Other {
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
            let unit_kind = all_units.get(&fact.unit_ref).copied().unwrap_or(UnitKind::Other);
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
            )?;
        }

        for fact in &doc.instance_facts {
            let ctx = match fact.context_ref.as_ref().and_then(|id| all_contexts.get(id)) {
                Some(c) => c,
                None => continue,
            };
            if all_units.get(&fact.unit_ref).copied().unwrap_or(UnitKind::Other) == UnitKind::Other {
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
            let unit_kind = all_units.get(&fact.unit_ref).copied().unwrap_or(UnitKind::Other);
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
                            "Conflicting values for inventory concept {} in {}: {} vs {}",
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
) -> Result<(), String> {
    let period = match &ctx.period {
        Some(p) => p.clone(),
        None => return Ok(()),
    };

    if unit_kind == UnitKind::Shares {
        if xml_util::should_use_context(ctx, ContextMode::Instant) {
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

// ── XML extraction passes ──

fn qname_str(q: QName<'_>) -> String {
    String::from_utf8_lossy(q.as_ref()).to_string()
}

fn attr_str(attrs: &mut quick_xml::events::attributes::Attributes, target_local: &str) -> Option<String> {
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

/// Extract all context definitions using event-based parsing.
fn extract_contexts(content: &str) -> HashMap<String, ContextInfo> {
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
                            let mut dimension_name = String::new();
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    let k = xml_util::local_name(
                                        &String::from_utf8_lossy(attr.key.as_ref()).to_string(),
                                    );
                                    if k == "dimension" {
                                        dimension_name = xml_util::local_name(
                                            &String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                }
                            }
                            // Read the text content for member name
                            let mut member_buf = Vec::new();
                            loop {
                                match reader.read_event_into(&mut member_buf) {
                                    Ok(Event::Text(t)) => {
                                        let member_name = xml_util::local_name(
                                            &t.unescape().unwrap_or_default().trim().to_string(),
                                        );
                                        if member_name == "NonConsolidatedMember"
                                            || (dimension_name == "ConsolidatedOrNonConsolidatedAxis"
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
                // Determine unit kind from measure text inside the unit element
                let mut kind = UnitKind::Other;
                let mut inner_buf = Vec::new();
                let mut inner_depth: usize = 0;
                loop {
                    match reader.read_event_into(&mut inner_buf) {
                        Ok(Event::Text(t)) => {
                            let text = t.unescape().unwrap_or_default();
                            let measure = xml_util::local_name(text.trim());
                            if measure == "JPY" {
                                kind = UnitKind::JPY;
                            } else if measure == "shares" {
                                kind = UnitKind::Shares;
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

fn parse_inline_fact_empty(
    attrs: &mut quick_xml::events::attributes::Attributes,
) -> InlineFact {
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
