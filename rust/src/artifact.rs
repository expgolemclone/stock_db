use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::NaiveDate;
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use rayon::prelude::*;

use crate::linkbase;
use crate::taxonomy;
use crate::types::{
    ConceptKey, ContextInfo, ContextMode, ExplicitMember, LoadedXbrlArtifact, RawStatementFact,
    ShareClassFact, StatementPresentationEdge, TextFact, TextTable, UnitKind, UnitMap,
};
use crate::xml_util;

struct ParsedDocument {
    nsmap: HashMap<String, String>,
    contexts: HashMap<String, ContextInfo>,
    units: UnitMap,
    inline_facts: Vec<InlineFact>,
    instance_facts: Vec<InstanceFact>,
}

struct FactDocument {
    content: String,
    has_inline_facts: bool,
    include_inline_numeric_facts: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FactBucketKind {
    Financial,
    Inventory,
    NonConsolidatedFinancial,
    NonConsolidatedInventory,
    NonConsolidatedDividend,
    Shares,
}

type ConceptDecimalsByContext = HashMap<(FactBucketKind, String, ConceptKey), String>;

fn parse_document_parts(
    content: &str,
    has_inline_facts: bool,
    include_inline_numeric_facts: bool,
) -> Result<ParsedDocument, String> {
    let mut parsed = scan_document_parts(content)?;
    parsed.inline_facts = if has_inline_facts {
        extract_inline_facts(content, include_inline_numeric_facts)?
    } else {
        Vec::new()
    };
    Ok(parsed)
}

pub fn load_xbrl_artifact(path: &str) -> Result<LoadedXbrlArtifact, String> {
    let artifact_path = Path::new(path);
    let is_dir = artifact_path.is_dir();

    let fact_documents: Vec<FactDocument> = if artifact_path.is_file() {
        let ext = artifact_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.to_lowercase() != "xbrl" {
            return Err(format!("Not an XBRL file: {}", path));
        }
        vec![FactDocument {
            content: xml_util::read_file_content(artifact_path)
                .ok_or_else(|| format!("Cannot read file: {}", path))?,
            has_inline_facts: false,
            include_inline_numeric_facts: false,
        }]
    } else if artifact_path.is_dir() {
        let xbrl_dir = artifact_path.join("XBRL");
        if !xbrl_dir.is_dir() {
            return Ok(LoadedXbrlArtifact {
                path: path.to_string(),
                is_dir: true,
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
            });
        }
        let public_doc = xbrl_dir.join("PublicDoc");
        let mut public_xbrl: Vec<FactDocument> = Vec::new();
        let mut public_inline: Vec<String> = Vec::new();
        if public_doc.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&public_doc) {
                for entry in entries.flatten() {
                    let fp = entry.path();
                    let ext = fp
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if ext == "xbrl" {
                        if let Some(c) = xml_util::read_file_content(&fp) {
                            public_xbrl.push(FactDocument {
                                content: c,
                                has_inline_facts: false,
                                include_inline_numeric_facts: false,
                            });
                        }
                    } else if matches!(ext.as_str(), "xhtml" | "html" | "htm")
                        && let Some(c) = xml_util::read_file_content(&fp)
                        && xml_util::has_inline_xbrl_fact(&c)
                    {
                        public_inline.push(c);
                    }
                }
            }
        }
        if !public_xbrl.is_empty() {
            public_xbrl.extend(public_inline.into_iter().map(|content| FactDocument {
                content,
                has_inline_facts: true,
                include_inline_numeric_facts: false,
            }));
            public_xbrl
        } else if !public_inline.is_empty() {
            public_inline
                .into_iter()
                .map(|content| FactDocument {
                    content,
                    has_inline_facts: true,
                    include_inline_numeric_facts: true,
                })
                .collect()
        } else {
            let fact_paths = xml_util::iter_fact_document_paths(artifact_path);
            let mut contents = Vec::new();
            for fp in &fact_paths {
                let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "xbrl" {
                    if let Some(c) = xml_util::read_file_content(fp) {
                        contents.push(FactDocument {
                            content: c,
                            has_inline_facts: false,
                            include_inline_numeric_facts: false,
                        });
                    }
                } else if let Some(c) = xml_util::read_file_content(fp)
                    && xml_util::is_valid_xbrl_text(&c)
                {
                    contents.push(FactDocument {
                        content: c,
                        has_inline_facts: true,
                        include_inline_numeric_facts: true,
                    });
                }
            }
            contents
        }
    } else {
        return Err(format!("Path does not exist: {}", path));
    };

    if fact_documents.is_empty() {
        return Err(format!("No parseable XBRL documents found in: {}", path));
    }

    let parsed_docs: Vec<ParsedDocument> = fact_documents
        .par_iter()
        .enumerate()
        .map(|(index, doc)| {
            parse_document_parts(
                &doc.content,
                doc.has_inline_facts,
                doc.include_inline_numeric_facts,
            )
            .map_err(|error| format!("XBRL document {}: {error}", index + 1))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut all_contexts: HashMap<String, ContextInfo> = HashMap::new();
    let mut all_units: UnitMap = HashMap::new();
    let mut prefix_namespaces: HashMap<String, String> = HashMap::new();

    for doc in &parsed_docs {
        all_contexts.extend(doc.contexts.iter().map(|(k, v)| (k.clone(), v.clone())));
        all_units.extend(doc.units.iter().map(|(k, v)| (k.clone(), *v)));
        for (prefix, namespace) in &doc.nsmap {
            prefix_namespaces
                .entry(prefix.clone())
                .or_insert_with(|| namespace.clone());
        }
    }

    let mut financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut non_consolidated_financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut non_consolidated_inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut non_consolidated_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> =
        HashMap::new();
    let mut shares_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>> = HashMap::new();
    let mut concept_decimals_by_context: ConceptDecimalsByContext = HashMap::new();
    let mut raw_balance_sheet_facts: Vec<RawStatementFact> = Vec::new();
    let mut raw_profit_loss_facts: Vec<RawStatementFact> = Vec::new();
    let mut raw_cash_flow_facts: Vec<RawStatementFact> = Vec::new();
    let mut share_class_facts: Vec<ShareClassFact> = Vec::new();
    let mut text_facts: Vec<TextFact> = Vec::new();
    let label_map = load_label_map(artifact_path)?;
    let statement_roles = if is_dir {
        build_statement_roles_by_concept(artifact_path, &prefix_namespaces)
    } else {
        HashMap::new()
    };
    let statement_presentation_edges = if is_dir {
        build_statement_presentation_edges(artifact_path, &prefix_namespaces, &label_map)
    } else {
        Vec::new()
    };

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
            let concept = match xml_util::resolve_qname(&fact.name, &doc.nsmap, "") {
                Some(c) => c,
                None => continue,
            };
            if is_ixbrl_fact_element_concept(&concept.1) {
                continue;
            }
            if let Some(row) = build_text_fact(
                ctx,
                &concept,
                &fact.text_value,
                &fact.text_tables,
                fact.is_nil,
            ) {
                text_facts.push(row);
            }
            let unit_kind = all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other);
            if unit_kind == UnitKind::Other {
                continue;
            }
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
            store_fact_buckets(
                ctx,
                fact.context_ref.as_deref().unwrap_or(""),
                concept,
                value,
                unit_kind,
                &fact.unit_ref,
                &fact.decimals,
                &mut financial_facts,
                &mut inventory_facts,
                &mut non_consolidated_financial_facts,
                &mut non_consolidated_inventory_facts,
                &mut non_consolidated_facts,
                &mut shares_facts,
                &mut concept_decimals_by_context,
                &statement_roles,
                &mut raw_balance_sheet_facts,
                &mut raw_profit_loss_facts,
                &mut raw_cash_flow_facts,
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
            let concept = match xml_util::resolve_qname(&fact.tag, &doc.nsmap, "") {
                Some(c) => c,
                None => continue,
            };
            if is_ixbrl_fact_element_concept(&concept.1) {
                continue;
            }
            if let Some(row) = build_text_fact(
                ctx,
                &concept,
                &fact.text_value,
                &fact.text_tables,
                fact.is_nil,
            ) {
                text_facts.push(row);
            }
            let unit_kind = all_units
                .get(&fact.unit_ref)
                .copied()
                .unwrap_or(UnitKind::Other);
            if unit_kind == UnitKind::Other {
                continue;
            }
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
            store_fact_buckets(
                ctx,
                fact.context_ref.as_deref().unwrap_or(""),
                concept,
                value,
                unit_kind,
                &fact.unit_ref,
                &fact.decimals,
                &mut financial_facts,
                &mut inventory_facts,
                &mut non_consolidated_financial_facts,
                &mut non_consolidated_inventory_facts,
                &mut non_consolidated_facts,
                &mut shares_facts,
                &mut concept_decimals_by_context,
                &statement_roles,
                &mut raw_balance_sheet_facts,
                &mut raw_profit_loss_facts,
                &mut raw_cash_flow_facts,
                &mut share_class_facts,
                &label_map,
            )?;
        }
    }
    let raw_balance_sheet_facts = dedupe_raw_statement_facts(raw_balance_sheet_facts)?;
    let raw_profit_loss_facts = dedupe_raw_statement_facts(raw_profit_loss_facts)?;
    let raw_cash_flow_facts = dedupe_raw_statement_facts(raw_cash_flow_facts)?;

    Ok(LoadedXbrlArtifact {
        path: path.to_string(),
        is_dir,
        financial_facts,
        inventory_facts,
        non_consolidated_financial_facts,
        non_consolidated_inventory_facts,
        non_consolidated_facts,
        shares_facts,
        raw_balance_sheet_facts,
        raw_profit_loss_facts,
        raw_cash_flow_facts,
        statement_presentation_edges,
        share_class_facts,
        text_facts,
    })
}

#[derive(Debug, Default)]
struct StatementRoles {
    balance_sheet: Vec<String>,
    profit_loss: Vec<String>,
    cash_flow: Vec<String>,
}

fn build_statement_roles_by_concept(
    artifact_path: &Path,
    prefix_namespaces: &HashMap<String, String>,
) -> HashMap<ConceptKey, StatementRoles> {
    let (by_path, by_basename) = linkbase::build_concept_lookup(artifact_path);
    let graphs = linkbase::build_presentation_graphs(
        artifact_path,
        &by_path,
        &by_basename,
        prefix_namespaces,
    );
    let mut result: HashMap<ConceptKey, StatementRoles> = HashMap::new();

    for (role_uri, graph) in graphs {
        let Some(statement) = classify_statement_role(&role_uri) else {
            continue;
        };
        for concept in presentation_graph_concepts(&graph) {
            let roles = result.entry(concept).or_default();
            match statement {
                "bs" => push_unique_role(&mut roles.balance_sheet, &role_uri),
                "pl" => push_unique_role(&mut roles.profit_loss, &role_uri),
                "cf" => push_unique_role(&mut roles.cash_flow, &role_uri),
                _ => {}
            }
        }
    }

    for roles in result.values_mut() {
        roles.balance_sheet.sort();
        roles.profit_loss.sort();
        roles.cash_flow.sort();
    }
    result
}

fn build_statement_presentation_edges(
    artifact_path: &Path,
    prefix_namespaces: &HashMap<String, String>,
    label_map: &HashMap<String, String>,
) -> Vec<StatementPresentationEdge> {
    let (by_path, by_basename) = linkbase::build_concept_lookup(artifact_path);
    let graphs = linkbase::build_presentation_graphs(
        artifact_path,
        &by_path,
        &by_basename,
        prefix_namespaces,
    );
    let mut result: Vec<StatementPresentationEdge> = Vec::new();

    for (role_uri, graph) in graphs {
        let Some(statement) = classify_statement_role(&role_uri) else {
            continue;
        };
        for (parent, children) in graph {
            for edge in children {
                result.push(StatementPresentationEdge {
                    statement: statement.to_string(),
                    role_uri: role_uri.clone(),
                    parent_label: label_for_concept(&parent, label_map),
                    parent: parent.clone(),
                    child_label: label_for_concept(&edge.child, label_map),
                    child: edge.child,
                    child_order: edge.order,
                });
            }
        }
    }

    result.sort_by(|left, right| {
        left.statement
            .cmp(&right.statement)
            .then_with(|| left.role_uri.cmp(&right.role_uri))
            .then_with(|| left.parent.1.cmp(&right.parent.1))
            .then_with(|| {
                left.child_order
                    .partial_cmp(&right.child_order)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.child.1.cmp(&right.child.1))
    });
    result
}

fn classify_statement_role(role_uri: &str) -> Option<&'static str> {
    let normalized = role_uri.to_ascii_lowercase();
    if normalized.contains("notes") {
        return None;
    }
    if normalized.contains("balancesheet") || normalized.contains("statementoffinancialposition") {
        return Some("bs");
    }
    if normalized.contains("statementofcashflows") {
        return Some("cf");
    }
    if normalized.contains("comprehensive") {
        return None;
    }
    if normalized.contains("statementofincome") || normalized.contains("statementofprofitorloss") {
        return Some("pl");
    }
    None
}

fn presentation_graph_concepts(
    graph: &HashMap<ConceptKey, Vec<linkbase::PresentationArc>>,
) -> HashSet<ConceptKey> {
    let mut concepts = HashSet::new();
    for (parent, children) in graph {
        concepts.insert(parent.clone());
        concepts.extend(children.iter().map(|edge| edge.child.clone()));
    }
    concepts
}

fn push_unique_role(roles: &mut Vec<String>, role_uri: &str) {
    if !roles.iter().any(|existing| existing == role_uri) {
        roles.push(role_uri.to_string());
    }
}

struct InlineFact {
    name: String,
    context_ref: Option<String>,
    unit_ref: String,
    decimals: String,
    scale: String,
    sign: String,
    is_nil: bool,
    text_value: String,
    text_tables: Vec<TextTable>,
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
    text_tables: Vec<TextTable>,
}

struct TextBlockContent {
    text: String,
    tables: Vec<TextTable>,
}

fn decimals_tolerance(decimals: &str) -> f64 {
    let decimals = decimals.trim();
    if decimals.is_empty() || decimals.eq_ignore_ascii_case("INF") {
        return 0.0;
    }
    decimals
        .parse::<i32>()
        .ok()
        .map(|value| 0.5 * 10f64.powi(-value))
        .unwrap_or(0.0)
}

fn decimals_precision(decimals: &str) -> i32 {
    let decimals = decimals.trim();
    if decimals.eq_ignore_ascii_case("INF") {
        i32::MAX
    } else {
        decimals.parse::<i32>().unwrap_or(i32::MIN)
    }
}

fn values_compatible(left: f64, right: f64, left_decimals: &str, right_decimals: &str) -> bool {
    let declared = decimals_tolerance(left_decimals).max(decimals_tolerance(right_decimals));
    let floating = f64::EPSILON * left.abs().max(right.abs()).max(1.0) * 8.0;
    (left - right).abs() <= declared.max(floating)
}

fn context_period_key(ctx: &ContextInfo, context_id: &str) -> Option<String> {
    let period = ctx.period.as_deref()?;
    let period_end = ctx.period_end.as_deref()?;
    Some(format!(
        "{period}|{}|{period_end}|{context_id}",
        ctx.period_start.as_deref().unwrap_or("")
    ))
}

fn store_concept_value(
    bucket: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    bucket_kind: FactBucketKind,
    decimals_by_context: &mut ConceptDecimalsByContext,
    period: &str,
    concept: ConceptKey,
    value: Option<f64>,
    decimals: &str,
) -> Result<(), String> {
    let decimals_key = (bucket_kind, period.to_string(), concept.clone());
    let existing_decimals = decimals_by_context
        .get(&decimals_key)
        .cloned()
        .unwrap_or_default();
    let by_period = bucket.entry(period.to_string()).or_default();
    match by_period.get_mut(&concept) {
        None => {
            by_period.insert(concept, value);
            decimals_by_context.insert(decimals_key, decimals.to_string());
        }
        Some(existing) => match (*existing, value) {
            (Some(left), Some(right)) => {
                if !values_compatible(left, right, &existing_decimals, decimals) {
                    return Err(format!(
                        "Conflicting values for XBRL concept {} in {}: {} vs {}",
                        concept.1, period, left, right
                    ));
                }
                if decimals_precision(decimals) > decimals_precision(&existing_decimals) {
                    *existing = Some(right);
                    decimals_by_context.insert(decimals_key, decimals.to_string());
                }
            }
            (None, Some(right)) => {
                *existing = Some(right);
                decimals_by_context.insert(decimals_key, decimals.to_string());
            }
            (None, None)
                if decimals_precision(decimals) > decimals_precision(&existing_decimals) =>
            {
                decimals_by_context.insert(decimals_key, decimals.to_string());
            }
            _ => {}
        },
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn store_fact_buckets(
    ctx: &ContextInfo,
    context_id: &str,
    concept: ConceptKey,
    value: Option<f64>,
    unit_kind: UnitKind,
    unit_ref: &str,
    decimals: &str,
    financial_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    inventory_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_financial_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_inventory_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    non_consolidated_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    shares_facts: &mut HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    decimals_by_context: &mut ConceptDecimalsByContext,
    statement_roles: &HashMap<ConceptKey, StatementRoles>,
    raw_balance_sheet_facts: &mut Vec<RawStatementFact>,
    raw_profit_loss_facts: &mut Vec<RawStatementFact>,
    raw_cash_flow_facts: &mut Vec<RawStatementFact>,
    share_class_facts: &mut Vec<ShareClassFact>,
    label_map: &HashMap<String, String>,
) -> Result<(), String> {
    let period = match &ctx.period {
        Some(p) => p.clone(),
        None => return Ok(()),
    };
    let context_period = match context_period_key(ctx, context_id) {
        Some(period) => period,
        None => return Ok(()),
    };

    if unit_kind == UnitKind::Shares {
        if let Some(row) = build_share_class_fact(ctx, &concept, value, label_map) {
            share_class_facts.push(row);
        }
        if xml_util::is_relevant_shares_fact(&concept.1)
            && xml_util::should_use_context(ctx, ContextMode::Instant)
        {
            store_concept_value(
                shares_facts,
                FactBucketKind::Shares,
                decimals_by_context,
                &context_period,
                concept,
                value,
                decimals,
            )?;
        }
        return Ok(());
    }

    if matches!(unit_kind, UnitKind::JPY | UnitKind::JPYPerShare) {
        store_raw_statement_facts(
            ctx,
            context_id,
            &concept,
            value,
            unit_kind,
            unit_ref,
            decimals,
            statement_roles.get(&concept),
            raw_balance_sheet_facts,
            raw_profit_loss_facts,
            raw_cash_flow_facts,
        );
    }

    if xml_util::should_use_context(ctx, ContextMode::Financial) {
        store_concept_value(
            financial_facts,
            FactBucketKind::Financial,
            decimals_by_context,
            &context_period,
            concept.clone(),
            value,
            decimals,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::Instant) {
        store_concept_value(
            inventory_facts,
            FactBucketKind::Inventory,
            decimals_by_context,
            &context_period,
            concept.clone(),
            value,
            decimals,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedFinancial) {
        store_concept_value(
            non_consolidated_financial_facts,
            FactBucketKind::NonConsolidatedFinancial,
            decimals_by_context,
            &context_period,
            concept.clone(),
            value,
            decimals,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedInstant) {
        store_concept_value(
            non_consolidated_inventory_facts,
            FactBucketKind::NonConsolidatedInventory,
            decimals_by_context,
            &context_period,
            concept.clone(),
            value,
            decimals,
        )?;
    }
    if xml_util::should_use_context(ctx, ContextMode::NonConsolidatedDividend) {
        store_concept_value(
            non_consolidated_facts,
            FactBucketKind::NonConsolidatedDividend,
            decimals_by_context,
            &context_period,
            concept,
            value,
            decimals,
        )?;
    }
    let _ = period;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn store_raw_statement_facts(
    ctx: &ContextInfo,
    context_id: &str,
    concept: &ConceptKey,
    value: Option<f64>,
    unit_kind: UnitKind,
    unit_ref: &str,
    decimals: &str,
    roles: Option<&StatementRoles>,
    raw_balance_sheet_facts: &mut Vec<RawStatementFact>,
    raw_profit_loss_facts: &mut Vec<RawStatementFact>,
    raw_cash_flow_facts: &mut Vec<RawStatementFact>,
) {
    let Some(period) = &ctx.period else {
        return;
    };
    let dimensions_json = context_dimensions_json(ctx);
    let consolidation_scope = if ctx.is_non_consolidated {
        "non_consolidated"
    } else {
        "consolidated"
    };

    let mut pushed = false;
    if let Some(roles) = roles {
        for role_uri in &roles.balance_sheet {
            raw_balance_sheet_facts.push(raw_statement_fact(
                "bs", ctx, period, consolidation_scope, context_id, &dimensions_json, concept,
                role_uri, unit_kind, unit_ref, decimals, value,
            ));
            pushed = true;
        }
        for role_uri in &roles.profit_loss {
            raw_profit_loss_facts.push(raw_statement_fact(
                "pl", ctx, period, consolidation_scope, context_id, &dimensions_json, concept,
                role_uri, unit_kind, unit_ref, decimals, value,
            ));
            pushed = true;
        }
        for role_uri in &roles.cash_flow {
            raw_cash_flow_facts.push(raw_statement_fact(
                "cf", ctx, period, consolidation_scope, context_id, &dimensions_json, concept,
                role_uri, unit_kind, unit_ref, decimals, value,
            ));
            pushed = true;
        }
    }

    if !pushed
        && (xml_util::should_use_context(ctx, ContextMode::Instant)
            || xml_util::should_use_context(ctx, ContextMode::NonConsolidatedInstant))
    {
        raw_balance_sheet_facts.push(raw_statement_fact(
            "bs", ctx, period, consolidation_scope, context_id, &dimensions_json, concept, "",
            unit_kind, unit_ref, decimals, value,
        ));
    }

    if !pushed
        && !ctx.is_instant
        && let Some(statement) = taxonomy::raw_statement_for_concept(concept)
    {
        let target = match statement {
            "bs" => raw_balance_sheet_facts,
            "pl" => raw_profit_loss_facts,
            "cf" => raw_cash_flow_facts,
            _ => return,
        };
        target.push(raw_statement_fact(
            statement, ctx, period, consolidation_scope, context_id, &dimensions_json, concept, "",
            unit_kind, unit_ref, decimals, value,
        ));
    }
}

fn context_duration_days(ctx: &ContextInfo) -> Option<i64> {
    let start = NaiveDate::parse_from_str(ctx.period_start.as_deref()?, "%Y-%m-%d").ok()?;
    let end = NaiveDate::parse_from_str(ctx.period_end.as_deref()?, "%Y-%m-%d").ok()?;
    Some((end - start).num_days() + 1)
}

#[allow(clippy::too_many_arguments)]
fn raw_statement_fact(
    statement: &str,
    ctx: &ContextInfo,
    period: &str,
    consolidation_scope: &str,
    context_id: &str,
    dimensions_json: &str,
    concept: &ConceptKey,
    role_uri: &str,
    unit_kind: UnitKind,
    unit_ref: &str,
    decimals: &str,
    value: Option<f64>,
) -> RawStatementFact {
    let period_end = ctx
        .period_end
        .clone()
        .expect("context with a period key must preserve its full period end date");
    RawStatementFact {
        statement: statement.to_string(),
        period: period.to_string(),
        period_start: ctx.period_start.clone(),
        period_end,
        duration_days: context_duration_days(ctx),
        consolidation_scope: consolidation_scope.to_string(),
        context_id: context_id.to_string(),
        dimensions_json: dimensions_json.to_string(),
        concept: concept.clone(),
        role_uri: role_uri.to_string(),
        unit: unit_kind.as_str().to_string(),
        unit_ref: unit_ref.to_string(),
        decimals: decimals.to_string(),
        value,
    }
}

fn dedupe_raw_statement_facts(
    facts: Vec<RawStatementFact>,
) -> Result<Vec<RawStatementFact>, String> {
    let mut by_key: HashMap<
        (String, String, String, String, String, String, ConceptKey),
        RawStatementFact,
    > = HashMap::new();
    for fact in facts {
        let key = (
            fact.statement.clone(),
            fact.period.clone(),
            fact.consolidation_scope.clone(),
            fact.context_id.clone(),
            fact.dimensions_json.clone(),
            fact.role_uri.clone(),
            fact.concept.clone(),
        );
        match by_key.get_mut(&key) {
            None => {
                by_key.insert(key, fact);
            }
            Some(existing) => {
                if existing.unit != fact.unit || existing.unit_ref != fact.unit_ref {
                    return Err(format!(
                        "Conflicting raw units for {} in {} {}: {} ({}) vs {} ({})",
                        existing.concept.1,
                        existing.period,
                        existing.context_id,
                        existing.unit,
                        existing.unit_ref,
                        fact.unit,
                        fact.unit_ref
                    ));
                }
                match (existing.value, fact.value) {
                    (Some(left), Some(right))
                        if !values_compatible(left, right, &existing.decimals, &fact.decimals) =>
                    {
                        return Err(format!(
                            "Conflicting raw {} values for {} in {} {} {} {}: {} vs {}",
                            existing.statement,
                            existing.concept.1,
                            existing.period,
                            existing.consolidation_scope,
                            existing.context_id,
                            existing.role_uri,
                            left,
                            right
                        ));
                    }
                    (Some(_), Some(_))
                        if decimals_precision(&fact.decimals)
                            > decimals_precision(&existing.decimals) =>
                    {
                        *existing = fact;
                    }
                    (None, Some(_)) => {
                        *existing = fact;
                    }
                    _ => {}
                }
            }
        }
    }
    let mut rows: Vec<RawStatementFact> = by_key.into_values().collect();
    rows.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| b.period.cmp(&a.period))
            .then_with(|| a.consolidation_scope.cmp(&b.consolidation_scope))
            .then_with(|| a.context_id.cmp(&b.context_id))
            .then_with(|| a.dimensions_json.cmp(&b.dimensions_json))
            .then_with(|| a.role_uri.cmp(&b.role_uri))
            .then_with(|| a.concept.0.cmp(&b.concept.0))
            .then_with(|| a.concept.1.cmp(&b.concept.1))
    });
    Ok(rows)
}

fn context_dimensions_json(ctx: &ContextInfo) -> String {
    let mut dimensions: Vec<(String, String)> = ctx
        .explicit_members
        .iter()
        .map(|member| {
            (
                xml_util::concept_key_string(&member.dimension),
                xml_util::concept_key_string(&member.member),
            )
        })
        .collect();
    dimensions.sort();
    if dimensions.is_empty() {
        return "[]".to_string();
    }
    let parts: Vec<String> = dimensions
        .into_iter()
        .map(|(dimension, member)| {
            format!(
                "{{\"dimension\":\"{}\",\"member\":\"{}\"}}",
                json_escape(&dimension),
                json_escape(&member)
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
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

fn build_text_fact(
    ctx: &ContextInfo,
    concept: &ConceptKey,
    text_value: &str,
    text_tables: &[TextTable],
    is_nil: bool,
) -> Option<TextFact> {
    if is_nil || text_value.trim().is_empty() {
        return None;
    }
    let period = ctx.period.clone()?;
    let text = xml_util::plain_text_from_xbrl_text(text_value);
    if !xml_util::is_relevant_potential_equity_text_fact(&concept.1, &text) {
        return None;
    }
    Some(TextFact {
        period,
        source_concept: xml_util::concept_key_string(concept),
        text,
        tables: text_tables.to_vec(),
    })
}

fn qname_str(q: QName<'_>) -> String {
    String::from_utf8_lossy(q.as_ref()).to_string()
}

fn attr_str(
    attrs: &mut quick_xml::events::attributes::Attributes,
    target_local: &str,
) -> Result<Option<String>, String> {
    for attr in attrs {
        let attr = attr.map_err(|error| format!("invalid XBRL attribute: {error}"))?;
        let key = qname_str(attr.key);
        if xml_util::local_name(&key) == target_local {
            return Ok(Some(String::from_utf8_lossy(&attr.value).to_string()));
        }
    }
    Ok(None)
}

fn scan_document_parts(content: &str) -> Result<ParsedDocument, String> {
    let mut nsmap: HashMap<String, String> = HashMap::new();
    let mut contexts: HashMap<String, ContextInfo> = HashMap::new();
    let mut units: UnitMap = HashMap::new();
    let mut instance_facts = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut saw_root = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if !saw_root {
                    collect_root_namespaces(&mut e.attributes(), &mut nsmap)?;
                    saw_root = true;
                }

                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);
                if local == "context" {
                    if let Some((id, ctx)) =
                        parse_context_start(&mut e.attributes(), &mut reader, &nsmap)?
                    {
                        contexts.insert(id, ctx);
                    }
                    continue;
                }
                if local == "unit" {
                    if let Some((id, kind)) = parse_unit_start(&mut e.attributes(), &mut reader)? {
                        units.insert(id, kind);
                    }
                    continue;
                }

                let ns = xml_util::namespace_uri(&tag);
                if is_instance_infrastructure(&local, ns) {
                    if !is_ixbrl_nonfraction(&local) {
                        let _ = read_text_until_end(&mut reader, &local)?;
                    }
                    continue;
                }

                if let Some(fact) =
                    parse_instance_fact_start(tag, &mut e.attributes(), &mut reader)?
                {
                    instance_facts.push(fact);
                }
            }
            Ok(Event::Empty(e)) => {
                if !saw_root {
                    collect_root_namespaces(&mut e.attributes(), &mut nsmap)?;
                    saw_root = true;
                }

                let tag = qname_str(e.name());
                let local = xml_util::local_name(&tag);
                if local == "unit" {
                    if let Some(id) = attr_str(&mut e.attributes(), "id")? {
                        units.insert(id, UnitKind::Other);
                    }
                    continue;
                }

                let ns = xml_util::namespace_uri(&tag);
                if is_instance_infrastructure(&local, ns) {
                    continue;
                }

                if let Some(fact) = parse_instance_fact_empty(tag, &mut e.attributes())? {
                    instance_facts.push(fact);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL XML: {error}")),
        }
        buf.clear();
    }

    if !saw_root {
        return Err("XBRL document has no root element".to_string());
    }

    Ok(ParsedDocument {
        nsmap,
        contexts,
        units,
        inline_facts: Vec::new(),
        instance_facts,
    })
}

fn collect_root_namespaces(
    attrs: &mut quick_xml::events::attributes::Attributes,
    nsmap: &mut HashMap<String, String>,
) -> Result<(), String> {
    for attr in attrs {
        let attr = attr.map_err(|error| format!("invalid XBRL namespace attribute: {error}"))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let val = String::from_utf8_lossy(&attr.value).to_string();
        if let Some(prefix) = key.strip_prefix("xmlns:") {
            nsmap.insert(prefix.to_string(), val);
        } else if key == "xmlns" {
            nsmap.insert(String::new(), val);
        }
    }
    Ok(())
}

fn parse_context_start(
    attrs: &mut quick_xml::events::attributes::Attributes,
    reader: &mut Reader<&[u8]>,
    nsmap: &HashMap<String, String>,
) -> Result<Option<(String, ContextInfo)>, String> {
    let context_id = attr_str(attrs, "id")?.unwrap_or_default();
    let mut instant_text: Option<String> = None;
    let mut start_date_text: Option<String> = None;
    let mut end_date_text: Option<String> = None;
    let mut is_instant = false;
    let mut has_dimensions = false;
    let mut is_non_consolidated = false;
    let mut dimension_count: usize = 0;
    let mut explicit_members: Vec<ExplicitMember> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.as_str() {
                    "instant" => {
                        let value = read_text_until_end(reader, "instant")?;
                        if !value.is_empty() {
                            instant_text = Some(value);
                            is_instant = true;
                        }
                    }
                    "startDate" => {
                        let value = read_text_until_end(reader, "startDate")?;
                        if !value.is_empty() {
                            start_date_text = Some(value);
                        }
                    }
                    "endDate" => {
                        let value = read_text_until_end(reader, "endDate")?;
                        if !value.is_empty() {
                            end_date_text = Some(value);
                        }
                    }
                    "explicitMember" => {
                        has_dimensions = true;
                        dimension_count += 1;
                        let dimension_qname =
                            attr_str(&mut e.attributes(), "dimension")?.unwrap_or_default();
                        let member_qname = read_text_until_end(reader, "explicitMember")?;
                        let dimension_name = xml_util::local_name(&dimension_qname);
                        let member_name = xml_util::local_name(&member_qname);
                        if member_name == "NonConsolidatedMember"
                            || (dimension_name == "ConsolidatedOrNonConsolidatedAxis"
                                && member_name == "NonConsolidatedMember")
                        {
                            is_non_consolidated = true;
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
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if local == "explicitMember" {
                    has_dimensions = true;
                    dimension_count += 1;
                }
            }
            Ok(Event::End(e)) => {
                if xml_util::local_name(&qname_str(e.name())) == "context" {
                    break;
                }
            }
            Ok(Event::Eof) => {
                return Err("unexpected EOF while reading XBRL context".to_string());
            }
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL context XML: {error}")),
        }
        buf.clear();
    }

    if context_id.is_empty() {
        return Ok(None);
    }

    let period_end = instant_text.clone().or(end_date_text.clone());
    let period = period_end.as_ref().and_then(|date| {
        (date.len() >= 7).then(|| date[..7].to_string())
    });

    Ok(Some((
        context_id,
        ContextInfo {
            period,
            period_start: start_date_text,
            period_end,
            _instant: instant_text,
            is_instant,
            has_dimensions,
            is_non_consolidated,
            dimension_count,
            explicit_members,
        },
    )))
}

fn parse_unit_start(
    attrs: &mut quick_xml::events::attributes::Attributes,
    reader: &mut Reader<&[u8]>,
) -> Result<Option<(String, UnitKind)>, String> {
    let unit_id = attr_str(attrs, "id")?;
    let mut buf = Vec::new();
    let mut inner_depth: usize = 0;
    let mut has_jpy = false;
    let mut has_shares = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                let text = t
                    .unescape()
                    .map_err(|error| format!("invalid XBRL unit text: {error}"))?;
                let measure = xml_util::local_name(text.trim());
                if measure == "JPY" {
                    has_jpy = true;
                } else if measure == "shares" {
                    has_shares = true;
                }
            }
            Ok(Event::Start(_)) => inner_depth += 1,
            Ok(Event::End(e)) => {
                if inner_depth == 0 && xml_util::local_name(&qname_str(e.name())) == "unit" {
                    break;
                }
                inner_depth = inner_depth.saturating_sub(1);
            }
            Ok(Event::Eof) => return Err("unexpected EOF while reading XBRL unit".to_string()),
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL unit XML: {error}")),
        }
        buf.clear();
    }

    let Some(unit_id) = unit_id else {
        return Ok(None);
    };
    let kind = match (has_jpy, has_shares) {
        (true, true) => UnitKind::JPYPerShare,
        (true, false) => UnitKind::JPY,
        (false, true) => UnitKind::Shares,
        (false, false) => UnitKind::Other,
    };
    Ok(Some((unit_id, kind)))
}

fn is_instance_infrastructure(local: &str, ns: &str) -> bool {
    is_ixbrl_nonfraction(local)
        || is_ixbrl_nonnumeric(local)
        || is_ixbrl_ignored_inline_element(local)
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
}

fn is_ixbrl_ignored_inline_element(local: &str) -> bool {
    matches!(
        local.to_ascii_lowercase().as_str(),
        "continuation" | "exclude" | "footnote"
    )
}

fn is_ixbrl_fact_element_concept(local: &str) -> bool {
    is_ixbrl_nonfraction(local)
        || is_ixbrl_nonnumeric(local)
        || is_ixbrl_ignored_inline_element(local)
}

fn parse_instance_fact_start(
    tag: String,
    attrs: &mut quick_xml::events::attributes::Attributes,
    reader: &mut Reader<&[u8]>,
) -> Result<Option<InstanceFact>, String> {
    let attrs = collect_attrs(attrs)?;
    let Some(context_ref) = attrs.get("contextref").cloned() else {
        return Ok(None);
    };
    let unit_ref = attrs.get("unitref").cloned().unwrap_or_default();
    let decimals = attrs.get("decimals").cloned().unwrap_or_default();
    let scale = attrs.get("scale").cloned().unwrap_or_default();
    let sign = attrs.get("sign").cloned().unwrap_or_default();
    let is_nil = attrs.get("nil").is_some_and(|v| v.to_lowercase() == "true");
    let local = xml_util::local_name(&tag);
    let content = if is_nil {
        let _ = read_text_block_until_end(reader, &local)?;
        TextBlockContent {
            text: String::new(),
            tables: Vec::new(),
        }
    } else {
        read_text_block_until_end(reader, &local)?
    };

    Ok(Some(InstanceFact {
        tag,
        context_ref: Some(context_ref),
        unit_ref,
        decimals,
        scale,
        sign,
        is_nil,
        text_value: content.text,
        text_tables: content.tables,
    }))
}

fn parse_instance_fact_empty(
    tag: String,
    attrs: &mut quick_xml::events::attributes::Attributes,
) -> Result<Option<InstanceFact>, String> {
    let attrs = collect_attrs(attrs)?;
    let Some(context_ref) = attrs.get("contextref").cloned() else {
        return Ok(None);
    };
    Ok(Some(InstanceFact {
        tag,
        context_ref: Some(context_ref),
        unit_ref: attrs.get("unitref").cloned().unwrap_or_default(),
        decimals: attrs.get("decimals").cloned().unwrap_or_default(),
        scale: attrs.get("scale").cloned().unwrap_or_default(),
        sign: attrs.get("sign").cloned().unwrap_or_default(),
        is_nil: attrs.get("nil").is_some_and(|v| v.to_lowercase() == "true"),
        text_value: String::new(),
        text_tables: Vec::new(),
    }))
}

#[derive(Clone)]
struct LabelResource {
    text: String,
    role: String,
    lang: String,
}

fn load_label_map(artifact_path: &Path) -> Result<HashMap<String, String>, String> {
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
            let path_labels = extract_label_map(&content)
                .map_err(|error| format!("label file {}: {error}", path.display()))?;
            for (key, value) in path_labels {
                let replace = labels.get(&key).is_none_or(|existing| {
                    !contains_japanese(existing) && contains_japanese(&value)
                });
                if replace {
                    labels.insert(key, value);
                }
            }
        }
    }
    Ok(labels)
}

fn label_for_concept(concept: &ConceptKey, label_map: &HashMap<String, String>) -> String {
    label_map
        .get(&concept.1)
        .or_else(|| label_map.get(&format!("{}_{}", namespace_prefix(&concept.0), concept.1)))
        .cloned()
        .unwrap_or_else(|| concept.1.clone())
}

fn namespace_prefix(namespace: &str) -> &str {
    namespace
        .rsplit('/')
        .next()
        .and_then(|last| last.split('_').next())
        .unwrap_or("")
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

fn extract_label_map(content: &str) -> Result<HashMap<String, String>, String> {
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
                    let attrs = collect_attrs(&mut e.attributes())?;
                    let text = read_text_until_end(&mut reader, "label")?;
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
                let attrs = collect_attrs(&mut e.attributes())?;
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
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL label XML: {error}")),
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
    Ok(labels)
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

fn extract_inline_facts(content: &str, include_numeric: bool) -> Result<Vec<InlineFact>, String> {
    let mut facts: Vec<InlineFact> = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if !is_inline_fact_element(&local, include_numeric) {
                    continue;
                }
                facts.push(parse_inline_fact_start(
                    &mut e.attributes(),
                    &mut reader,
                    &local,
                )?);
            }
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                if !is_inline_fact_element(&local, include_numeric) {
                    continue;
                }
                facts.push(parse_inline_fact_empty(&mut e.attributes())?);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("invalid inline XBRL XML: {error}")),
        }
        buf.clear();
    }
    Ok(facts)
}

fn is_inline_fact_element(local: &str, include_numeric: bool) -> bool {
    is_ixbrl_nonnumeric(local) || (include_numeric && is_ixbrl_nonfraction(local))
}

fn is_ixbrl_nonfraction(local: &str) -> bool {
    local.eq_ignore_ascii_case("nonfraction")
}

fn is_ixbrl_nonnumeric(local: &str) -> bool {
    local.eq_ignore_ascii_case("nonnumeric")
}

fn collect_attrs(
    attrs: &mut quick_xml::events::attributes::Attributes,
) -> Result<HashMap<String, String>, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for attr in attrs {
        let attr = attr.map_err(|error| format!("invalid XBRL attribute: {error}"))?;
        let k = xml_util::local_name(&String::from_utf8_lossy(attr.key.as_ref()).to_string());
        let v = String::from_utf8_lossy(&attr.value).to_string();
        map.insert(k.to_lowercase(), v);
    }
    Ok(map)
}

fn parse_inline_fact_start(
    attrs: &mut quick_xml::events::attributes::Attributes,
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
) -> Result<InlineFact, String> {
    let a = collect_attrs(attrs)?;
    let name = a.get("name").cloned().unwrap_or_default();
    let context_ref = a.get("contextref").cloned();
    let unit_ref = a.get("unitref").cloned().unwrap_or_default();
    let decimals = a.get("decimals").cloned().unwrap_or_default();
    let scale = a.get("scale").cloned().unwrap_or_default();
    let sign = a.get("sign").cloned().unwrap_or_default();
    let is_nil = a.get("nil").is_some_and(|v| v.to_lowercase() == "true");

    let content = if is_nil {
        TextBlockContent {
            text: String::new(),
            tables: Vec::new(),
        }
    } else {
        read_text_block_until_end(reader, end_tag)?
    };

    Ok(InlineFact {
        name,
        context_ref,
        unit_ref,
        decimals,
        scale,
        sign,
        is_nil,
        text_value: content.text,
        text_tables: content.tables,
    })
}

fn parse_inline_fact_empty(
    attrs: &mut quick_xml::events::attributes::Attributes,
) -> Result<InlineFact, String> {
    let a = collect_attrs(attrs)?;
    let name = a.get("name").cloned().unwrap_or_default();
    let context_ref = a.get("contextref").cloned();
    let unit_ref = a.get("unitref").cloned().unwrap_or_default();
    let decimals = a.get("decimals").cloned().unwrap_or_default();
    let scale = a.get("scale").cloned().unwrap_or_default();
    let sign = a.get("sign").cloned().unwrap_or_default();
    let is_nil = a.get("nil").is_some_and(|v| v.to_lowercase() == "true");

    Ok(InlineFact {
        name,
        context_ref,
        unit_ref,
        decimals,
        scale,
        sign,
        is_nil,
        text_value: String::new(),
        text_tables: Vec::new(),
    })
}

fn read_text_block_until_end(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
) -> Result<TextBlockContent, String> {
    let mut text = String::new();
    let mut tables = Vec::new();
    let mut current_table: Option<Vec<Vec<String>>> = None;
    let mut current_row: Option<Vec<String>> = None;
    let mut current_cell: Option<String> = None;
    let mut depth: usize = 0;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                let chunk = t
                    .unescape()
                    .map_err(|error| format!("invalid XBRL text block: {error}"))?;
                append_text_chunk(&mut text, &chunk);
                if let Some(cell) = &mut current_cell {
                    append_text_chunk(cell, &chunk);
                }
            }
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "table" => current_table = Some(Vec::new()),
                    "tr" if current_table.is_some() => current_row = Some(Vec::new()),
                    "td" | "th" if current_row.is_some() => current_cell = Some(String::new()),
                    "br" => {
                        append_text_chunk(&mut text, " ");
                        if let Some(cell) = &mut current_cell {
                            append_text_chunk(cell, " ");
                        }
                    }
                    _ => {}
                }
                depth += 1;
            }
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "td" | "th" if current_row.is_some() => {
                        if let Some(row) = &mut current_row {
                            row.push(String::new());
                        }
                    }
                    "br" => {
                        append_text_chunk(&mut text, " ");
                        if let Some(cell) = &mut current_cell {
                            append_text_chunk(cell, " ");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "td" | "th" => {
                        if let (Some(row), Some(cell)) = (&mut current_row, current_cell.take()) {
                            row.push(compact_inline_text(&cell));
                        }
                    }
                    "tr" => {
                        if let (Some(table), Some(row)) = (&mut current_table, current_row.take()) {
                            if row.iter().any(|cell| !cell.is_empty()) {
                                table.push(row);
                            }
                        }
                    }
                    "table" => {
                        if let Some(table) = current_table.take() {
                            if !table.is_empty() {
                                tables.push(TextTable { rows: table });
                            }
                        }
                    }
                    _ => {}
                }
                if depth == 0 && local == end_tag {
                    break;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => {
                return Err(format!("unexpected EOF while reading XBRL {end_tag}"));
            }
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL text block XML: {error}")),
        }
        buf.clear();
    }

    let text = text.trim().to_string();
    if tables.is_empty() && text.contains("<table") {
        tables = extract_tables_from_html_fragment(&text);
    }
    Ok(TextBlockContent { text, tables })
}

fn read_text_until_end(reader: &mut Reader<&[u8]>, end_tag: &str) -> Result<String, String> {
    let mut text = String::new();
    let mut depth: usize = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                text.push_str(
                    &t.unescape()
                        .map_err(|error| format!("invalid XBRL text: {error}"))?,
                );
            }
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(e)) => {
                if depth == 0 && xml_util::local_name(&qname_str(e.name())) == end_tag {
                    break;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => {
                return Err(format!("unexpected EOF while reading XBRL {end_tag}"));
            }
            Ok(_) => {}
            Err(error) => return Err(format!("invalid XBRL text XML: {error}")),
        }
        buf.clear();
    }
    Ok(text.trim().to_string())
}

fn extract_tables_from_html_fragment(raw: &str) -> Vec<TextTable> {
    let wrapped = format!("<root>{raw}</root>");
    let mut reader = Reader::from_str(&wrapped);
    reader.config_mut().trim_text(true);
    let mut tables = Vec::new();
    let mut current_table: Option<Vec<Vec<String>>> = None;
    let mut current_row: Option<Vec<String>> = None;
    let mut current_cell: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                if let Some(cell) = &mut current_cell {
                    append_text_chunk(cell, &t.unescape().unwrap_or_default());
                }
            }
            Ok(Event::Start(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "table" => current_table = Some(Vec::new()),
                    "tr" if current_table.is_some() => current_row = Some(Vec::new()),
                    "td" | "th" if current_row.is_some() => current_cell = Some(String::new()),
                    "br" => {
                        if let Some(cell) = &mut current_cell {
                            append_text_chunk(cell, " ");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "td" | "th" if current_row.is_some() => {
                        if let Some(row) = &mut current_row {
                            row.push(String::new());
                        }
                    }
                    "br" => {
                        if let Some(cell) = &mut current_cell {
                            append_text_chunk(cell, " ");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&qname_str(e.name()));
                match local.to_ascii_lowercase().as_str() {
                    "td" | "th" => {
                        if let (Some(row), Some(cell)) = (&mut current_row, current_cell.take()) {
                            row.push(compact_inline_text(&cell));
                        }
                    }
                    "tr" => {
                        if let (Some(table), Some(row)) = (&mut current_table, current_row.take()) {
                            if row.iter().any(|cell| !cell.is_empty()) {
                                table.push(row);
                            }
                        }
                    }
                    "table" => {
                        if let Some(table) = current_table.take() {
                            if !table.is_empty() {
                                tables.push(TextTable { rows: table });
                            }
                        }
                    }
                    "root" => break,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    tables
}

fn append_text_chunk(output: &mut String, chunk: &str) {
    if chunk.trim().is_empty() {
        return;
    }
    if !output.is_empty() && !output.chars().last().is_some_and(|ch| ch.is_whitespace()) {
        output.push(' ');
    }
    output.push_str(chunk.trim());
}

fn compact_inline_text(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(prefix: &str, extension: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "stock_db_{prefix}_{}_{}.{}",
            std::process::id(),
            nonce,
            extension
        ))
    }

    fn context(start: Option<&str>, end: &str) -> ContextInfo {
        ContextInfo {
            period: Some(end[..7].to_string()),
            period_start: start.map(str::to_string),
            period_end: Some(end.to_string()),
            _instant: start.is_none().then(|| end.to_string()),
            is_instant: start.is_none(),
            has_dimensions: false,
            is_non_consolidated: false,
            dimension_count: 0,
            explicit_members: Vec::new(),
        }
    }

    #[test]
    fn rejects_malformed_xbrl_instead_of_returning_partial_facts() {
        let path = temp_path("malformed_xbrl", "xbrl");
        std::fs::write(
            &path,
            r#"<xbrli:xbrl xmlns:xbrli="http://www.xbrl.org/2003/instance">
                <xbrli:context id="broken"></xbrli:unit>
            </xbrli:xbrl>"#,
        )
        .unwrap();

        let error = load_xbrl_artifact(path.to_str().unwrap()).unwrap_err();

        assert!(error.contains("XBRL"));
        assert!(error.contains("XML"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn preserves_duration_per_share_unit_and_decimals() {
        let path = temp_path("eps_metadata", "xbrl");
        std::fs::write(
            &path,
            r#"<xbrli:xbrl
                xmlns:xbrli="http://www.xbrl.org/2003/instance"
                xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
                xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2025-11-01/jppfs_cor">
              <xbrli:context id="CurrentYearDuration">
                <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                <xbrli:period>
                  <xbrli:startDate>2024-04-01</xbrli:startDate>
                  <xbrli:endDate>2025-03-31</xbrli:endDate>
                </xbrli:period>
              </xbrli:context>
              <xbrli:unit id="JPYPerShare">
                <xbrli:divide>
                  <xbrli:unitNumerator><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unitNumerator>
                  <xbrli:unitDenominator><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unitDenominator>
                </xbrli:divide>
              </xbrli:unit>
              <jppfs_cor:BasicEarningsLossPerShare contextRef="CurrentYearDuration"
                  unitRef="JPYPerShare" decimals="2">123.45</jppfs_cor:BasicEarningsLossPerShare>
            </xbrli:xbrl>"#,
        )
        .unwrap();

        let artifact = load_xbrl_artifact(path.to_str().unwrap()).unwrap();
        let fact = artifact
            .raw_profit_loss_facts
            .iter()
            .find(|fact| fact.concept.1 == "BasicEarningsLossPerShare")
            .unwrap();
        assert_eq!(fact.period, "2025-03");
        assert_eq!(fact.period_start.as_deref(), Some("2024-04-01"));
        assert_eq!(fact.period_end, "2025-03-31");
        assert_eq!(fact.duration_days, Some(365));
        assert_eq!(fact.unit, "JPY/share");
        assert_eq!(fact.decimals, "2");
        assert_eq!(fact.value, Some(123.45));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn declared_decimals_allow_equivalent_rounded_duplicates() {
        assert!(values_compatible(1_234_000.0, 1_234_400.0, "-3", "-3"));
        assert!(!values_compatible(1_234_000.0, 1_236_000.0, "-3", "-3"));
    }

    #[test]
    fn store_concept_value_uses_both_declared_precisions_independent_of_order() {
        let concept = ("urn:test".to_string(), "Revenue".to_string());
        for reverse in [false, true] {
            let mut bucket = HashMap::new();
            let mut precisions = HashMap::new();
            let facts = if reverse {
                [(1_234_400.0, "0"), (1_234_000.0, "-3")]
            } else {
                [(1_234_000.0, "-3"), (1_234_400.0, "0")]
            };
            for (value, decimals) in facts {
                store_concept_value(
                    &mut bucket,
                    FactBucketKind::Financial,
                    &mut precisions,
                    "2025-03|2024-04-01|2025-03-31|CurrentYearDuration",
                    concept.clone(),
                    Some(value),
                    decimals,
                )
                .unwrap();
            }
            assert_eq!(
                bucket["2025-03|2024-04-01|2025-03-31|CurrentYearDuration"][&concept],
                Some(1_234_400.0)
            );
        }
    }

    #[test]
    fn declared_precision_is_isolated_between_fact_buckets() {
        let concept = ("urn:test".to_string(), "Revenue".to_string());
        let period = "2025-03|2024-04-01|2025-03-31|CurrentYearDuration";
        let mut financial = HashMap::new();
        let mut dividend = HashMap::new();
        let mut precisions = HashMap::new();

        store_concept_value(
            &mut financial,
            FactBucketKind::Financial,
            &mut precisions,
            period,
            concept.clone(),
            Some(1_234_000.0),
            "-3",
        )
        .unwrap();
        store_concept_value(
            &mut dividend,
            FactBucketKind::NonConsolidatedDividend,
            &mut precisions,
            period,
            concept.clone(),
            Some(1_234_000.0),
            "-3",
        )
        .unwrap();
        store_concept_value(
            &mut financial,
            FactBucketKind::Financial,
            &mut precisions,
            period,
            concept.clone(),
            Some(1_234_400.0),
            "0",
        )
        .unwrap();
        store_concept_value(
            &mut dividend,
            FactBucketKind::NonConsolidatedDividend,
            &mut precisions,
            period,
            concept.clone(),
            Some(1_234_400.0),
            "0",
        )
        .unwrap();

        assert_eq!(financial[period][&concept], Some(1_234_400.0));
        assert_eq!(dividend[period][&concept], Some(1_234_400.0));
    }

    #[test]
    fn context_period_key_separates_same_end_month_different_durations() {
        let full_year = context(Some("2024-04-01"), "2025-03-31");
        let quarter = context(Some("2025-01-01"), "2025-03-31");
        assert_ne!(
            context_period_key(&full_year, "FullYear"),
            context_period_key(&quarter, "Quarter")
        );
    }

    #[test]
    fn ignores_ixbrl_element_names_that_look_like_concepts() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "stock_db_ixbrl_infra_concept_{}_{}",
            std::process::id(),
            nonce
        ));
        let public_doc = root.join("XBRL").join("PublicDoc");
        std::fs::create_dir_all(&public_doc).unwrap();
        std::fs::write(
            public_doc.join("report.xhtml"),
            r#"
            <html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL"
                  xmlns:xbrli="http://www.xbrl.org/2003/instance"
                  xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
                  xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2024-11-01/jppfs_cor">
              <body>
                <xbrli:context id="CurrentYearInstant">
                  <xbrli:entity><xbrli:identifier scheme="test">E1</xbrli:identifier></xbrli:entity>
                  <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
                </xbrli:context>
                <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
                <ix:nonFraction contextRef="CurrentYearInstant" decimals="0" scale="0" name="nonFraction" unitRef="JPY">111</ix:nonFraction>
                <ix:nonFraction contextRef="CurrentYearInstant" decimals="0" scale="0" name="jppfs_cor:CashAndDeposits" unitRef="JPY">222</ix:nonFraction>
              </body>
            </html>
            "#,
        )
        .unwrap();

        let artifact = load_xbrl_artifact(root.to_str().unwrap()).unwrap();
        let has_infra_concept = artifact
            .financial_facts
            .values()
            .chain(artifact.inventory_facts.values())
            .any(|facts| facts.keys().any(|concept| concept.1 == "nonFraction"));
        assert!(!has_infra_concept);
        assert!(
            artifact
                .inventory_facts
                .values()
                .any(|facts| facts.keys().any(|concept| concept.1 == "CashAndDeposits"))
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
