use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use rayon::prelude::*;
use rusqlite::{Connection, Transaction, params};

use crate::artifact;
use crate::financials;
use crate::potential_terms;
use crate::preferred_terms;
use crate::share_classes;
use crate::taxonomy::{self, TaxonomyConceptStatus};
use crate::types::{
    ConceptKey, DilutedEpsAdjustmentFact, PotentialEquityInstrumentFact, PotentialEquityStatusFact,
    PreferredShareTermFact, RawStatementFact, ShareClassFact, StatementPresentationEdge,
};

const SOURCE: &str = "edinet_xbrl";
const REPLACED_SOURCES: [&str; 5] = [
    "edinet_xbrl",
    "irbank",
    "irbank_bs",
    "irbank_forecast",
    "xbrl_bs",
];
const PARSE_PROGRESS_INTERVAL: Duration = Duration::from_secs(20 * 60);

type FinancialsByPeriod = HashMap<String, HashMap<String, HashMap<String, Option<f64>>>>;

#[derive(Debug, Clone)]
struct ReportRow {
    ticker: String,
    fiscal_year: String,
    doc_id: String,
    xbrl_path: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct TickerInput {
    ticker: String,
    reports: Vec<ReportRow>,
}

#[derive(Debug, Clone)]
pub struct FinancialDbRow {
    pub ticker: String,
    pub period: String,
    pub statement: String,
    pub item_name: String,
    pub value: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RawStatementDbRow {
    pub ticker: String,
    pub doc_id: String,
    pub statement: String,
    pub period: String,
    pub period_start: Option<String>,
    pub period_end: String,
    pub duration_days: Option<i64>,
    pub consolidation_scope: String,
    pub context_id: String,
    pub dimensions_json: String,
    pub concept_namespace: String,
    pub concept_name: String,
    pub canonical_statement: Option<String>,
    pub canonical_item_name: Option<String>,
    pub taxonomy_status: String,
    pub role_uri: String,
    pub unit: String,
    pub unit_ref: String,
    pub decimals: String,
    pub value: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct StatementPresentationEdgeDbRow {
    pub ticker: String,
    pub doc_id: String,
    pub statement: String,
    pub role_uri: String,
    pub parent_namespace: String,
    pub parent_name: String,
    pub parent_label: String,
    pub child_namespace: String,
    pub child_name: String,
    pub child_order: f64,
    pub child_label: String,
}

#[derive(Debug, Clone)]
pub struct ShareClassDbRow {
    pub ticker: String,
    pub period: String,
    pub class_key: String,
    pub class_name: String,
    pub shares: f64,
    pub is_preferred: bool,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct PreferredShareTermDbRow {
    pub ticker: String,
    pub period: String,
    pub class_key: String,
    pub class_name: String,
    pub terms_text: String,
    pub preferred_shares: Option<f64>,
    pub converted_common_shares: Option<f64>,
    pub conversion_ratio_to_common: Option<f64>,
    pub issue_price_yen: Option<f64>,
    pub capitalized_amount_yen: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct PotentialEquityTermDbRow {
    pub ticker: String,
    pub period: String,
    pub instrument_key: String,
    pub instrument_type: String,
    pub instrument_name: String,
    pub terms_text: String,
    pub period_end_common_shares: Option<f64>,
    pub exercise_price_yen: Option<f64>,
    pub conversion_price_yen: Option<f64>,
    pub face_value_yen: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct PotentialEquityStatusDbRow {
    pub ticker: String,
    pub period: String,
    pub status_key: String,
    pub has_potential_equity: bool,
    pub terms_text: String,
    pub source_concept: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct DilutedEpsAdjustmentDbRow {
    pub ticker: String,
    pub period: String,
    pub adjustment_key: String,
    pub adjustment_type: String,
    pub adjustment_name: String,
    pub terms_text: String,
    pub weighted_average_common_shares: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct ParsedTicker {
    pub ticker: String,
    pub financial_rows: Vec<FinancialDbRow>,
    pub raw_balance_sheet_rows: Vec<RawStatementDbRow>,
    pub raw_profit_loss_rows: Vec<RawStatementDbRow>,
    pub raw_cash_flow_rows: Vec<RawStatementDbRow>,
    pub statement_presentation_edge_rows: Vec<StatementPresentationEdgeDbRow>,
    pub share_class_rows: Vec<ShareClassDbRow>,
    pub preferred_share_term_rows: Vec<PreferredShareTermDbRow>,
    pub potential_equity_term_rows: Vec<PotentialEquityTermDbRow>,
    pub potential_equity_status_rows: Vec<PotentialEquityStatusDbRow>,
    pub diluted_eps_adjustment_rows: Vec<DilutedEpsAdjustmentDbRow>,
    pub period_count: usize,
}

#[derive(Debug, Clone)]
pub struct TickerSummary {
    pub ticker: String,
    pub status: String,
    pub financial_rows: usize,
    pub raw_balance_sheet_rows: usize,
    pub raw_profit_loss_rows: usize,
    pub raw_cash_flow_rows: usize,
    pub period_count: usize,
    pub share_class_rows: usize,
    pub preferred_share_term_rows: usize,
    pub potential_equity_term_rows: usize,
    pub potential_equity_status_rows: usize,
    pub diluted_eps_adjustment_rows: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxonomyAuditIssue {
    pub statement: String,
    pub concept_namespace: String,
    pub concept_name: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxonomyAuditConcept {
    pub statement: String,
    pub concept_namespace: String,
    pub concept_name: String,
    pub count: usize,
    pub taxonomy_status: String,
    /// How the concept was matched: exact_expanded_name, taxonomy_family_alias,
    /// safe_local_alias, or identity. Empty for excluded/extension/unknown.
    pub match_kind: String,
    pub canonical_statement: Option<String>,
    pub canonical_item_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxonomyCfSuffixIssue {
    pub statement: String,
    pub concept_name: String,
    pub canonical_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxonomyAuditSummary {
    pub checked_concepts: usize,
    pub concepts: Vec<TaxonomyAuditConcept>,
    pub missing: Vec<TaxonomyAuditIssue>,
    pub excluded: Vec<TaxonomyAuditIssue>,
    pub cf_suffix_collisions: Vec<TaxonomyCfSuffixIssue>,
}

#[derive(Debug, Clone)]
pub struct ParseDbSummary {
    pub ok: usize,
    pub errors: usize,
    pub skipped: usize,
    pub no_xbrl_files: bool,
    pub stopped_for_memory: bool,
    pub next_from_ticker: Option<String>,
    pub rss_percent: Option<f64>,
    pub results: Vec<TickerSummary>,
}

enum TickerParseError {
    Inventory(String),
    Fatal(String),
}

enum TickerParseOutcome {
    Parsed(ParsedTicker),
    InventoryError { ticker: String, message: String },
    Fatal { ticker: String, message: String },
}

pub fn parse_xbrl_financials_to_db(
    db_path: &str,
    ticker_filter: Option<&str>,
    from_ticker: Option<&str>,
    skip_existing: bool,
    emit_progress: bool,
    memory_restart_percent: Option<f64>,
    parse_batch_size: usize,
) -> Result<ParseDbSummary, String> {
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut conn = Connection::open(db_path).map_err(|err| err.to_string())?;
    conn.busy_timeout(Duration::from_secs(60))
        .map_err(|err| err.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|err| err.to_string())?;

    let mut tickers = load_ticker_inputs(&conn)?;
    if let Some(ticker) = ticker_filter {
        tickers.retain(|input| input.ticker == ticker);
    }
    if let Some(from) = from_ticker {
        tickers.retain(|input| input.ticker.as_str() >= from);
    }

    if tickers.is_empty() {
        return Ok(ParseDbSummary {
            ok: 0,
            errors: 0,
            skipped: 0,
            no_xbrl_files: true,
            stopped_for_memory: false,
            next_from_ticker: None,
            rss_percent: current_rss_percent(),
            results: Vec::new(),
        });
    }

    let skipped = if skip_existing {
        let existing = load_existing_edinet_tickers(&conn)?;
        let before = tickers.len();
        tickers.retain(|input| !existing.contains(&input.ticker));
        before - tickers.len()
    } else {
        0
    };

    if emit_progress && skipped > 0 {
        eprintln!("Skipping {skipped} tickers with existing {SOURCE} data");
    }

    let mut id_cache = IdCache::load(&conn)?;

    let parse_batch_size = parse_batch_size.max(1);
    let total = tickers.len();
    let started = Instant::now();
    let last_progress = Mutex::new(started);
    if emit_progress && total > 0 {
        eprintln!("Parsing {total} tickers; progress is reported every 20 minutes");
    }

    let mut ok = 0usize;
    let mut errors = 0usize;
    let mut results = Vec::with_capacity(total);
    let mut stopped_for_memory = false;
    let mut next_from_ticker = None;
    let mut rss_percent = current_rss_percent();

    for (chunk_index, chunk) in tickers.chunks(parse_batch_size).enumerate() {
        let processed_before = chunk_index * parse_batch_size;
        let outcomes = parse_tickers(
            chunk,
            emit_progress,
            processed_before,
            total,
            started,
            &last_progress,
        );

        if let Some((ticker, message)) = outcomes.iter().find_map(|outcome| match outcome {
            TickerParseOutcome::Fatal { ticker, message } => Some((ticker, message)),
            _ => None,
        }) {
            return Err(format!("{ticker}: {message}"));
        }

        let tx = conn.transaction().map_err(|err| err.to_string())?;
        for outcome in outcomes {
            match outcome {
                TickerParseOutcome::Parsed(parsed) => {
                    if parsed_has_no_facts(&parsed) {
                        results.push(no_facts_summary(parsed.ticker));
                        continue;
                    }
                    write_ticker_in_tx(&tx, &parsed, &mut id_cache)?;
                    ok += 1;
                    results.push(ok_summary(&parsed));
                }
                TickerParseOutcome::InventoryError { ticker, message } => {
                    errors += 1;
                    results.push(error_summary(ticker, message));
                }
                TickerParseOutcome::Fatal { .. } => {
                    unreachable!("fatal errors are returned before writes")
                }
            }
        }
        tx.commit().map_err(|err| err.to_string())?;

        rss_percent = current_rss_percent();
        let next_index = processed_before + chunk.len();
        if should_stop_for_memory(memory_restart_percent, rss_percent) && next_index < total {
            stopped_for_memory = true;
            next_from_ticker = Some(tickers[next_index].ticker.clone());
            if emit_progress {
                if let Some(percent) = rss_percent {
                    eprintln!(
                        "RSS memory {:.1}% reached threshold {:.1}%; restart from ticker {}",
                        percent,
                        memory_restart_percent.unwrap_or_default(),
                        tickers[next_index].ticker
                    );
                }
            }
            break;
        }
    }

    Ok(ParseDbSummary {
        ok,
        errors,
        skipped,
        no_xbrl_files: false,
        stopped_for_memory,
        next_from_ticker,
        rss_percent,
        results,
    })
}

pub fn audit_xbrl_taxonomy(
    db_path: &str,
    min_count: usize,
) -> Result<TaxonomyAuditSummary, String> {
    let min_count = min_count.max(1);
    let conn = Connection::open(db_path).map_err(|err| err.to_string())?;

    let tables: [(&str, &str); 3] = [
        ("xbrl_balance_sheet_facts", "bs"),
        ("xbrl_profit_loss_facts", "pl"),
        ("xbrl_cash_flow_facts", "cf"),
    ];

    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut concepts: Vec<TaxonomyAuditConcept> = Vec::new();
    let mut missing: Vec<TaxonomyAuditIssue> = Vec::new();
    let mut excluded: Vec<TaxonomyAuditIssue> = Vec::new();
    let mut cf_suffix_collisions: Vec<TaxonomyCfSuffixIssue> = Vec::new();

    for (table, statement) in &tables {
        let sql = format!(
            "SELECT concept_namespace, concept_name, COUNT(*) FROM {} \
             GROUP BY concept_namespace, concept_name",
            table
        );
        let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                ))
            })
            .map_err(|err| err.to_string())?;

        for row in rows {
            let (concept_namespace, concept_name, count) = row.map_err(|err| err.to_string())?;
            seen.insert((concept_namespace.clone(), concept_name.clone()));
            let concept_key = (concept_namespace.clone(), concept_name.clone());
            match taxonomy::concept_status(&concept_key) {
                TaxonomyConceptStatus::Semantic(canonical) => {
                    let canonical_statement =
                        taxonomy::statement_for_item_name(&canonical.item_name).map(str::to_string);
                    concepts.push(TaxonomyAuditConcept {
                        statement: statement.to_string(),
                        concept_namespace: concept_namespace.clone(),
                        concept_name: concept_name.clone(),
                        count,
                        taxonomy_status: "semantic".to_string(),
                        match_kind: canonical.match_kind.as_str().to_string(),
                        canonical_statement,
                        canonical_item_name: Some(canonical.item_name.clone()),
                    });
                    if let Some(issue) =
                        audit_cf_suffix_context(statement, &concept_key, &canonical.item_name)
                    {
                        cf_suffix_collisions.push(issue);
                    }
                }
                TaxonomyConceptStatus::Identity(candidate) => {
                    let canonical_statement =
                        taxonomy::statement_for_item_name(&candidate.item_name).map(str::to_string);
                    concepts.push(TaxonomyAuditConcept {
                        statement: statement.to_string(),
                        concept_namespace: concept_namespace.clone(),
                        concept_name: concept_name.clone(),
                        count,
                        taxonomy_status: "identity".to_string(),
                        match_kind: candidate.match_kind.as_str().to_string(),
                        canonical_statement,
                        canonical_item_name: Some(candidate.item_name.clone()),
                    });
                }
                TaxonomyConceptStatus::Extension => {
                    concepts.push(TaxonomyAuditConcept {
                        statement: statement.to_string(),
                        concept_namespace: concept_namespace.clone(),
                        concept_name: concept_name.clone(),
                        count,
                        taxonomy_status: "extension".to_string(),
                        match_kind: String::new(),
                        canonical_statement: None,
                        canonical_item_name: None,
                    });
                }
                TaxonomyConceptStatus::Excluded => {
                    concepts.push(TaxonomyAuditConcept {
                        statement: statement.to_string(),
                        concept_namespace: concept_namespace.clone(),
                        concept_name: concept_name.clone(),
                        count,
                        taxonomy_status: "excluded".to_string(),
                        match_kind: String::new(),
                        canonical_statement: None,
                        canonical_item_name: None,
                    });
                    excluded.push(TaxonomyAuditIssue {
                        statement: statement.to_string(),
                        concept_namespace,
                        concept_name,
                        count,
                    });
                }
                TaxonomyConceptStatus::Unknown => {
                    concepts.push(TaxonomyAuditConcept {
                        statement: statement.to_string(),
                        concept_namespace: concept_namespace.clone(),
                        concept_name: concept_name.clone(),
                        count,
                        taxonomy_status: "unknown".to_string(),
                        match_kind: String::new(),
                        canonical_statement: None,
                        canonical_item_name: None,
                    });
                    if count >= min_count {
                        missing.push(TaxonomyAuditIssue {
                            statement: statement.to_string(),
                            concept_namespace,
                            concept_name,
                            count,
                        });
                    }
                }
            }
        }
    }

    let checked_concepts = seen.len();
    concepts.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| a.taxonomy_status.cmp(&b.taxonomy_status))
            .then_with(|| a.concept_name.cmp(&b.concept_name))
    });
    missing.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| a.concept_name.cmp(&b.concept_name))
    });
    excluded.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| a.concept_name.cmp(&b.concept_name))
    });
    cf_suffix_collisions.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| a.concept_name.cmp(&b.concept_name))
            .then_with(|| a.canonical_name.cmp(&b.canonical_name))
    });

    Ok(TaxonomyAuditSummary {
        checked_concepts,
        concepts,
        missing,
        excluded,
        cf_suffix_collisions,
    })
}

fn audit_cf_suffix_context(
    statement: &str,
    concept: &ConceptKey,
    canonical_name: &str,
) -> Option<TaxonomyCfSuffixIssue> {
    let stem = cf_suffix_stem(&concept.1)?;
    if is_cf_context_allowlisted(canonical_name) {
        return None;
    }

    let mut reasons: Vec<&str> = Vec::new();
    let stem_key = (concept.0.clone(), stem.to_string());
    if let Some(stem_canonical) = crate::taxonomy::canonicalize_concept(&stem_key) {
        if stem_canonical.item_name == canonical_name {
            reasons.push("matches_stem_canonical");
        }
    }
    if !canonical_preserves_cf_context(canonical_name) {
        reasons.push("missing_cf_context_marker");
    }

    if reasons.is_empty() {
        return None;
    }
    Some(TaxonomyCfSuffixIssue {
        statement: statement.to_string(),
        concept_name: concept.1.to_string(),
        canonical_name: canonical_name.to_string(),
        reason: reasons.join(","),
    })
}

fn cf_suffix_stem(concept_name: &str) -> Option<&str> {
    for suffix in ["OpeCF", "FinCF", "InvCF"] {
        if let Some(stem) = concept_name.strip_suffix(suffix) {
            if !stem.is_empty() {
                return Some(stem);
            }
        }
    }
    None
}

fn canonical_preserves_cf_context(canonical_name: &str) -> bool {
    [
        "_ope_cf",
        "_operating_cf",
        "_fin_cf",
        "_financing_cf",
        "_inv_cf",
        "_investing_cf",
        "_cf_adjustment",
    ]
    .iter()
    .any(|marker| canonical_name.contains(marker))
}

fn is_cf_context_allowlisted(canonical_name: &str) -> bool {
    matches!(
        canonical_name,
        "operating_cf"
            | "investing_cf"
            | "financing_cf"
            | "capex"
            | "intangible_asset_purchases"
            | "ppe_sold"
            | "investment_securities_sold"
            | "investment_securities_purchased"
            | "treasury_stock_purchase"
            | "lt_debt_repayment"
            | "lt_debt_proceeds"
    )
}

fn parse_tickers(
    tickers: &[TickerInput],
    emit_progress: bool,
    completed_before: usize,
    total: usize,
    started: Instant,
    last_progress: &Mutex<Instant>,
) -> Vec<TickerParseOutcome> {
    if tickers.is_empty() {
        return Vec::new();
    }

    let completed = AtomicUsize::new(0);
    let aborted = AtomicBool::new(false);

    let outcomes: Vec<TickerParseOutcome> = tickers
        .par_iter()
        .filter_map(|input| {
            if aborted.load(Ordering::Relaxed) {
                return None;
            }
            let outcome = parse_ticker_outcome(input);
            let count = completed_before + completed.fetch_add(1, Ordering::Relaxed) + 1;
            let is_fatal = matches!(outcome, TickerParseOutcome::Fatal { .. });

            if is_fatal {
                aborted.store(true, Ordering::Relaxed);
            }

            if emit_progress {
                let now = Instant::now();
                let should_log = is_fatal || count == total || {
                    let guard = last_progress.lock().unwrap();
                    now.duration_since(*guard) >= PARSE_PROGRESS_INTERVAL
                };
                if should_log {
                    if let Ok(mut guard) = last_progress.lock() {
                        *guard = now;
                    }
                    log_parse_progress(count, total, &outcome, started.elapsed());
                }
            }

            Some(outcome)
        })
        .collect();

    outcomes
}

fn parsed_has_no_facts(parsed: &ParsedTicker) -> bool {
    parsed.financial_rows.is_empty()
        && parsed.raw_balance_sheet_rows.is_empty()
        && parsed.raw_profit_loss_rows.is_empty()
        && parsed.raw_cash_flow_rows.is_empty()
        && parsed.share_class_rows.is_empty()
        && parsed.preferred_share_term_rows.is_empty()
        && parsed.potential_equity_term_rows.is_empty()
        && parsed.potential_equity_status_rows.is_empty()
        && parsed.diluted_eps_adjustment_rows.is_empty()
}

fn ok_summary(parsed: &ParsedTicker) -> TickerSummary {
    TickerSummary {
        ticker: parsed.ticker.clone(),
        status: "ok".to_string(),
        financial_rows: parsed.financial_rows.len(),
        raw_balance_sheet_rows: parsed.raw_balance_sheet_rows.len(),
        raw_profit_loss_rows: parsed.raw_profit_loss_rows.len(),
        raw_cash_flow_rows: parsed.raw_cash_flow_rows.len(),
        period_count: parsed.period_count,
        share_class_rows: parsed.share_class_rows.len(),
        preferred_share_term_rows: parsed.preferred_share_term_rows.len(),
        potential_equity_term_rows: parsed.potential_equity_term_rows.len(),
        potential_equity_status_rows: parsed.potential_equity_status_rows.len(),
        diluted_eps_adjustment_rows: parsed.diluted_eps_adjustment_rows.len(),
        message: None,
    }
}

fn no_facts_summary(ticker: String) -> TickerSummary {
    TickerSummary {
        ticker,
        status: "no_facts".to_string(),
        financial_rows: 0,
        raw_balance_sheet_rows: 0,
        raw_profit_loss_rows: 0,
        raw_cash_flow_rows: 0,
        period_count: 0,
        share_class_rows: 0,
        preferred_share_term_rows: 0,
        potential_equity_term_rows: 0,
        potential_equity_status_rows: 0,
        diluted_eps_adjustment_rows: 0,
        message: None,
    }
}

fn error_summary(ticker: String, message: String) -> TickerSummary {
    TickerSummary {
        ticker,
        status: "error".to_string(),
        financial_rows: 0,
        raw_balance_sheet_rows: 0,
        raw_profit_loss_rows: 0,
        raw_cash_flow_rows: 0,
        period_count: 0,
        share_class_rows: 0,
        preferred_share_term_rows: 0,
        potential_equity_term_rows: 0,
        potential_equity_status_rows: 0,
        diluted_eps_adjustment_rows: 0,
        message: Some(message),
    }
}

fn should_stop_for_memory(threshold: Option<f64>, rss_percent: Option<f64>) -> bool {
    match (threshold, rss_percent) {
        (Some(threshold), Some(percent)) => percent >= threshold,
        _ => false,
    }
}

fn current_rss_percent() -> Option<f64> {
    let rss_kb = read_proc_kb_value("/proc/self/status", "VmRSS:")?;
    let total_kb = read_proc_kb_value("/proc/meminfo", "MemTotal:")?;
    if total_kb == 0 {
        return None;
    }
    Some((rss_kb as f64 / total_kb as f64) * 100.0)
}

fn read_proc_kb_value(path: &str, key: &str) -> Option<u64> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn log_parse_progress(
    completed: usize,
    total: usize,
    outcome: &TickerParseOutcome,
    elapsed: Duration,
) {
    let elapsed = format_duration(elapsed);
    match outcome {
        TickerParseOutcome::Fatal { ticker, message } => {
            eprintln!("[{completed}/{total}] elapsed {elapsed}; {ticker}: fatal: {message}")
        }
        _ => {
            if let Some(summary) = summary_from_outcome(outcome) {
                log_ticker_summary(completed, total, &elapsed, &summary);
            }
        }
    }
}

fn log_ticker_summary(completed: usize, total: usize, elapsed: &str, summary: &TickerSummary) {
    match summary.status.as_str() {
        "ok" => eprintln!(
            "[{completed}/{total}] elapsed {elapsed}; {}: {} canonical items, {} raw BS facts, {} raw PL facts, {} raw CF facts across {} periods, {} share classes, {} preferred terms, {} potential-equity terms, {} potential-equity statuses, {} diluted-eps adjustments",
            summary.ticker,
            summary.financial_rows,
            summary.raw_balance_sheet_rows,
            summary.raw_profit_loss_rows,
            summary.raw_cash_flow_rows,
            summary.period_count,
            summary.share_class_rows,
            summary.preferred_share_term_rows,
            summary.potential_equity_term_rows,
            summary.potential_equity_status_rows,
            summary.diluted_eps_adjustment_rows
        ),
        "no_facts" => eprintln!(
            "[{completed}/{total}] elapsed {elapsed}; {}: no parseable financial, share-class, preferred-share, or potential-equity facts",
            summary.ticker
        ),
        "error" => eprintln!(
            "[{completed}/{total}] elapsed {elapsed}; {}: {}",
            summary.ticker,
            summary.message.as_deref().unwrap_or("")
        ),
        _ => unreachable!("unexpected ticker summary status"),
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}m{seconds:02}s")
    } else {
        format!("{minutes}m{seconds:02}s")
    }
}

fn parse_ticker_outcome(input: &TickerInput) -> TickerParseOutcome {
    match parse_ticker(input) {
        Ok(parsed) => TickerParseOutcome::Parsed(parsed),
        Err(TickerParseError::Inventory(message)) => TickerParseOutcome::InventoryError {
            ticker: input.ticker.clone(),
            message,
        },
        Err(TickerParseError::Fatal(message)) => TickerParseOutcome::Fatal {
            ticker: input.ticker.clone(),
            message,
        },
    }
}

fn summary_from_outcome(outcome: &TickerParseOutcome) -> Option<TickerSummary> {
    match outcome {
        TickerParseOutcome::Parsed(parsed) => {
            if parsed.financial_rows.is_empty()
                && parsed.raw_balance_sheet_rows.is_empty()
                && parsed.raw_profit_loss_rows.is_empty()
                && parsed.raw_cash_flow_rows.is_empty()
                && parsed.share_class_rows.is_empty()
                && parsed.preferred_share_term_rows.is_empty()
                && parsed.potential_equity_term_rows.is_empty()
                && parsed.potential_equity_status_rows.is_empty()
                && parsed.diluted_eps_adjustment_rows.is_empty()
            {
                Some(TickerSummary {
                    ticker: parsed.ticker.clone(),
                    status: "no_facts".to_string(),
                    financial_rows: 0,
                    raw_balance_sheet_rows: 0,
                    raw_profit_loss_rows: 0,
                    raw_cash_flow_rows: 0,
                    period_count: 0,
                    share_class_rows: 0,
                    preferred_share_term_rows: 0,
                    potential_equity_term_rows: 0,
                    potential_equity_status_rows: 0,
                    diluted_eps_adjustment_rows: 0,
                    message: None,
                })
            } else {
                Some(TickerSummary {
                    ticker: parsed.ticker.clone(),
                    status: "ok".to_string(),
                    financial_rows: parsed.financial_rows.len(),
                    raw_balance_sheet_rows: parsed.raw_balance_sheet_rows.len(),
                    raw_profit_loss_rows: parsed.raw_profit_loss_rows.len(),
                    raw_cash_flow_rows: parsed.raw_cash_flow_rows.len(),
                    period_count: parsed.period_count,
                    share_class_rows: parsed.share_class_rows.len(),
                    preferred_share_term_rows: parsed.preferred_share_term_rows.len(),
                    potential_equity_term_rows: parsed.potential_equity_term_rows.len(),
                    potential_equity_status_rows: parsed.potential_equity_status_rows.len(),
                    diluted_eps_adjustment_rows: parsed.diluted_eps_adjustment_rows.len(),
                    message: None,
                })
            }
        }
        TickerParseOutcome::InventoryError { ticker, message } => Some(TickerSummary {
            ticker: ticker.clone(),
            status: "error".to_string(),
            financial_rows: 0,
            raw_balance_sheet_rows: 0,
            raw_profit_loss_rows: 0,
            raw_cash_flow_rows: 0,
            period_count: 0,
            share_class_rows: 0,
            preferred_share_term_rows: 0,
            potential_equity_term_rows: 0,
            potential_equity_status_rows: 0,
            diluted_eps_adjustment_rows: 0,
            message: Some(message.clone()),
        }),
        TickerParseOutcome::Fatal { .. } => None,
    }
}

fn load_ticker_inputs(conn: &Connection) -> Result<Vec<TickerInput>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT ticker, fiscal_year, doc_id, xbrl_path, updated_at
            FROM sec_reports
            WHERE xbrl_path IS NOT NULL
            ORDER BY ticker, fiscal_year ASC, updated_at ASC, doc_id ASC
            "#,
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ReportRow {
                ticker: row.get(0)?,
                fiscal_year: row.get(1)?,
                doc_id: row.get(2)?,
                xbrl_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|err| err.to_string())?;

    let mut grouped: Vec<TickerInput> = Vec::new();
    for row in rows {
        let report = row.map_err(|err| err.to_string())?;
        match grouped.last_mut() {
            Some(input) if input.ticker == report.ticker => input.reports.push(report),
            _ => grouped.push(TickerInput {
                ticker: report.ticker.clone(),
                reports: vec![report],
            }),
        }
    }
    Ok(grouped)
}

fn load_existing_edinet_tickers(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT fi.ticker
            FROM financial_items AS fi
            JOIN sec_reports AS sr
              ON sr.ticker = fi.ticker
            WHERE fi.source = ?1
            GROUP BY fi.ticker
            HAVING MAX(fi.updated_at) >= MAX(sr.updated_at)
            "#,
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([SOURCE], |row| row.get::<_, String>(0))
        .map_err(|err| err.to_string())?;

    let mut tickers = HashSet::new();
    for row in rows {
        tickers.insert(row.map_err(|err| err.to_string())?);
    }
    Ok(tickers)
}

fn parse_ticker(input: &TickerInput) -> Result<ParsedTicker, TickerParseError> {
    let mut reports = input.reports.clone();
    reports.sort_by(|a, b| {
        a.fiscal_year
            .cmp(&b.fiscal_year)
            .then_with(|| a.updated_at.cmp(&b.updated_at))
            .then_with(|| a.doc_id.cmp(&b.doc_id))
    });

    let mut merged: FinancialsByPeriod = HashMap::new();
    let mut share_classes_by_key: HashMap<(String, String), ShareClassFact> = HashMap::new();
    let mut preferred_terms_by_key: HashMap<
        (String, String, String, String),
        PreferredShareTermFact,
    > = HashMap::new();
    let mut potential_terms_by_key: HashMap<
        (String, String, String, String),
        PotentialEquityInstrumentFact,
    > = HashMap::new();
    let mut potential_statuses_by_key: HashMap<
        (String, String, String, String),
        PotentialEquityStatusFact,
    > = HashMap::new();
    let mut diluted_eps_by_key: HashMap<
        (String, String, String, String),
        DilutedEpsAdjustmentFact,
    > = HashMap::new();
    let mut raw_balance_sheet_rows: Vec<RawStatementDbRow> = Vec::new();
    let mut raw_profit_loss_rows: Vec<RawStatementDbRow> = Vec::new();
    let mut raw_cash_flow_rows: Vec<RawStatementDbRow> = Vec::new();
    let mut statement_presentation_edge_rows: Vec<StatementPresentationEdgeDbRow> = Vec::new();

    for report in &reports {
        let artifact =
            artifact::load_xbrl_artifact(&report.xbrl_path).map_err(TickerParseError::Fatal)?;
        if artifact.raw_balance_sheet_facts.is_empty() {
            if !artifact_has_parseable_facts(&artifact) {
                continue;
            }
            return Err(TickerParseError::Inventory(format!(
                "{}: no raw balance-sheet JPY facts found",
                report.doc_id
            )));
        }
        raw_balance_sheet_rows.extend(build_raw_statement_rows(
            &input.ticker,
            &report.doc_id,
            &artifact.raw_balance_sheet_facts,
        ));
        raw_profit_loss_rows.extend(build_raw_statement_rows(
            &input.ticker,
            &report.doc_id,
            &artifact.raw_profit_loss_facts,
        ));
        raw_cash_flow_rows.extend(build_raw_statement_rows(
            &input.ticker,
            &report.doc_id,
            &artifact.raw_cash_flow_facts,
        ));
        statement_presentation_edge_rows.extend(build_statement_presentation_edge_rows(
            &input.ticker,
            &report.doc_id,
            &artifact.statement_presentation_edges,
        ));
        let parsed = financials::parse_financials_from_artifact(&artifact)
            .map_err(|err| TickerParseError::Inventory(err.message))?;

        for (period, statements) in parsed {
            let period_bucket = merged.entry(period).or_default();
            for (statement, items) in statements {
                period_bucket.entry(statement).or_default().extend(items);
            }
        }

        for share_class in share_classes::parse_share_classes_from_artifact(&artifact) {
            let key = (share_class.period.clone(), share_class.class_name.clone());
            let replace = share_classes_by_key.get(&key).is_none_or(|existing| {
                share_class_source_priority(&share_class.source_kind)
                    <= share_class_source_priority(&existing.source_kind)
            });
            if replace {
                share_classes_by_key.insert(key, share_class);
            }
        }

        for term in preferred_terms::parse_preferred_share_terms_from_artifact(&artifact) {
            let key = (
                term.period.clone(),
                term.class_key.clone(),
                term.source_concept.clone(),
                term.terms_text.clone(),
            );
            preferred_terms_by_key.insert(key, term);
        }

        for term in potential_terms::parse_potential_equity_instruments_from_artifact(&artifact) {
            let key = (
                term.period.clone(),
                term.instrument_key.clone(),
                term.source_concept.clone(),
                term.terms_text.clone(),
            );
            potential_terms_by_key.insert(key, term);
        }

        for status in potential_terms::parse_potential_equity_statuses_from_artifact(&artifact) {
            let key = (
                status.period.clone(),
                status.status_key.clone(),
                status.source_concept.clone(),
                status.terms_text.clone(),
            );
            potential_statuses_by_key.insert(key, status);
        }

        for adj in potential_terms::parse_diluted_eps_adjustments_from_artifact(&artifact) {
            let key = (
                adj.period.clone(),
                adj.adjustment_key.clone(),
                adj.source_concept.clone(),
                adj.terms_text.clone(),
            );
            diluted_eps_by_key.insert(key, adj);
        }
    }

    let period_count = merged.len();
    let financial_rows = build_financial_rows(&input.ticker, &merged);
    let share_class_rows =
        build_share_class_rows(&input.ticker, share_classes_by_key.into_values().collect());
    let preferred_share_term_rows = build_preferred_share_term_rows(
        &input.ticker,
        preferred_terms_by_key.into_values().collect(),
    );
    let potential_equity_term_rows = build_potential_equity_term_rows(
        &input.ticker,
        potential_terms_by_key.into_values().collect(),
    );
    let potential_equity_status_rows = build_potential_equity_status_rows(
        &input.ticker,
        potential_statuses_by_key.into_values().collect(),
    );
    let diluted_eps_adjustment_rows = build_diluted_eps_adjustment_rows(
        &input.ticker,
        diluted_eps_by_key.into_values().collect(),
    );
    Ok(ParsedTicker {
        ticker: input.ticker.clone(),
        financial_rows,
        raw_balance_sheet_rows,
        raw_profit_loss_rows,
        raw_cash_flow_rows,
        statement_presentation_edge_rows,
        share_class_rows,
        preferred_share_term_rows,
        potential_equity_term_rows,
        potential_equity_status_rows,
        diluted_eps_adjustment_rows,
        period_count,
    })
}

fn share_class_source_priority(source_kind: &str) -> u8 {
    if source_kind == "classes_of_shares_axis" {
        0
    } else {
        1
    }
}

fn build_financial_rows(ticker: &str, merged: &FinancialsByPeriod) -> Vec<FinancialDbRow> {
    let mut periods: Vec<&String> = merged.keys().collect();
    periods.sort();
    periods.reverse();

    let mut rows = Vec::new();
    for period in periods {
        let mut statements: Vec<&String> = merged[period].keys().collect();
        statements.sort();
        for statement in statements {
            let mut item_names: Vec<&String> = merged[period][statement].keys().collect();
            item_names.sort();
            for item_name in item_names {
                rows.push(FinancialDbRow {
                    ticker: ticker.to_string(),
                    period: period.clone(),
                    statement: statement.clone(),
                    item_name: item_name.clone(),
                    value: merged[period][statement][item_name],
                });
            }
        }
    }
    rows
}

fn build_raw_statement_rows(
    ticker: &str,
    doc_id: &str,
    facts: &[RawStatementFact],
) -> Vec<RawStatementDbRow> {
    let mut rows: Vec<RawStatementDbRow> = facts
        .iter()
        .map(|fact| {
            let (canonical_statement, canonical_item_name, taxonomy_status) =
                raw_canonical_columns(&fact.concept);
            RawStatementDbRow {
                ticker: ticker.to_string(),
                doc_id: doc_id.to_string(),
                statement: fact.statement.clone(),
                period: fact.period.clone(),
                period_start: fact.period_start.clone(),
                period_end: fact.period_end.clone(),
                duration_days: fact.duration_days,
                consolidation_scope: fact.consolidation_scope.clone(),
                context_id: fact.context_id.clone(),
                dimensions_json: fact.dimensions_json.clone(),
                concept_namespace: fact.concept.0.clone(),
                concept_name: fact.concept.1.clone(),
                canonical_statement,
                canonical_item_name,
                taxonomy_status,
                role_uri: fact.role_uri.clone(),
                unit: fact.unit.clone(),
                unit_ref: fact.unit_ref.clone(),
                decimals: fact.decimals.clone(),
                value: fact.value,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.doc_id.cmp(&b.doc_id))
            .then_with(|| a.statement.cmp(&b.statement))
            .then_with(|| a.consolidation_scope.cmp(&b.consolidation_scope))
            .then_with(|| a.context_id.cmp(&b.context_id))
            .then_with(|| a.dimensions_json.cmp(&b.dimensions_json))
            .then_with(|| a.role_uri.cmp(&b.role_uri))
            .then_with(|| a.concept_namespace.cmp(&b.concept_namespace))
            .then_with(|| a.concept_name.cmp(&b.concept_name))
    });
    rows
}

fn raw_canonical_columns(concept: &ConceptKey) -> (Option<String>, Option<String>, String) {
    match taxonomy::concept_status(concept) {
        TaxonomyConceptStatus::Semantic(canonical) => {
            let statement =
                taxonomy::statement_for_item_name(&canonical.item_name).map(str::to_string);
            (statement, Some(canonical.item_name), "semantic".to_string())
        }
        TaxonomyConceptStatus::Identity(candidate) => {
            let statement =
                taxonomy::statement_for_item_name(&candidate.item_name).map(str::to_string);
            (statement, Some(candidate.item_name), "identity".to_string())
        }
        TaxonomyConceptStatus::Extension => (None, None, "extension".to_string()),
        TaxonomyConceptStatus::Excluded => (None, None, "excluded".to_string()),
        TaxonomyConceptStatus::Unknown => (None, None, "unknown".to_string()),
    }
}

fn build_statement_presentation_edge_rows(
    ticker: &str,
    doc_id: &str,
    edges: &[StatementPresentationEdge],
) -> Vec<StatementPresentationEdgeDbRow> {
    let mut rows: Vec<StatementPresentationEdgeDbRow> = edges
        .iter()
        .map(|edge| StatementPresentationEdgeDbRow {
            ticker: ticker.to_string(),
            doc_id: doc_id.to_string(),
            statement: edge.statement.clone(),
            role_uri: edge.role_uri.clone(),
            parent_namespace: edge.parent.0.clone(),
            parent_name: edge.parent.1.clone(),
            parent_label: edge.parent_label.clone(),
            child_namespace: edge.child.0.clone(),
            child_name: edge.child.1.clone(),
            child_order: edge.child_order,
            child_label: edge.child_label.clone(),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| a.role_uri.cmp(&b.role_uri))
            .then_with(|| a.parent_name.cmp(&b.parent_name))
            .then_with(|| {
                a.child_order
                    .partial_cmp(&b.child_order)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.child_name.cmp(&b.child_name))
    });
    rows
}

fn artifact_has_parseable_facts(artifact: &crate::types::LoadedXbrlArtifact) -> bool {
    !artifact.financial_facts.is_empty()
        || !artifact.inventory_facts.is_empty()
        || !artifact.non_consolidated_financial_facts.is_empty()
        || !artifact.non_consolidated_inventory_facts.is_empty()
        || !artifact.non_consolidated_facts.is_empty()
        || !artifact.shares_facts.is_empty()
        || !artifact.raw_balance_sheet_facts.is_empty()
        || !artifact.raw_profit_loss_facts.is_empty()
        || !artifact.raw_cash_flow_facts.is_empty()
        || !artifact.share_class_facts.is_empty()
        || !artifact.text_facts.is_empty()
}

fn build_share_class_rows(ticker: &str, mut rows: Vec<ShareClassFact>) -> Vec<ShareClassDbRow> {
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| b.class_name.cmp(&a.class_name))
    });

    rows.into_iter()
        .map(|row| ShareClassDbRow {
            ticker: ticker.to_string(),
            period: row.period,
            class_key: row.class_key,
            class_name: row.class_name,
            shares: row.shares,
            is_preferred: row.is_preferred,
            source_kind: row.source_kind,
        })
        .collect()
}

fn build_preferred_share_term_rows(
    ticker: &str,
    mut rows: Vec<PreferredShareTermFact>,
) -> Vec<PreferredShareTermDbRow> {
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.class_name.cmp(&b.class_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });

    rows.into_iter()
        .map(|row| PreferredShareTermDbRow {
            ticker: ticker.to_string(),
            period: row.period,
            class_key: row.class_key,
            class_name: row.class_name,
            terms_text: row.terms_text,
            preferred_shares: row.preferred_shares,
            converted_common_shares: row.converted_common_shares,
            conversion_ratio_to_common: row.conversion_ratio_to_common,
            issue_price_yen: row.issue_price_yen,
            capitalized_amount_yen: row.capitalized_amount_yen,
            source_concept: row.source_concept,
            source_kind: row.source_kind,
        })
        .collect()
}

fn build_potential_equity_term_rows(
    ticker: &str,
    mut rows: Vec<PotentialEquityInstrumentFact>,
) -> Vec<PotentialEquityTermDbRow> {
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.instrument_type.cmp(&b.instrument_type))
            .then_with(|| a.instrument_name.cmp(&b.instrument_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });

    rows.into_iter()
        .map(|row| PotentialEquityTermDbRow {
            ticker: ticker.to_string(),
            period: row.period,
            instrument_key: row.instrument_key,
            instrument_type: row.instrument_type,
            instrument_name: row.instrument_name,
            terms_text: row.terms_text,
            period_end_common_shares: row.period_end_common_shares,
            exercise_price_yen: row.exercise_price_yen,
            conversion_price_yen: row.conversion_price_yen,
            face_value_yen: row.face_value_yen,
            source_concept: row.source_concept,
            source_kind: row.source_kind,
        })
        .collect()
}

fn build_potential_equity_status_rows(
    ticker: &str,
    mut rows: Vec<PotentialEquityStatusFact>,
) -> Vec<PotentialEquityStatusDbRow> {
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.status_key.cmp(&b.status_key))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });

    rows.into_iter()
        .map(|row| PotentialEquityStatusDbRow {
            ticker: ticker.to_string(),
            period: row.period,
            status_key: row.status_key,
            has_potential_equity: row.has_potential_equity,
            terms_text: row.terms_text,
            source_concept: row.source_concept,
            source_kind: row.source_kind,
        })
        .collect()
}

fn build_diluted_eps_adjustment_rows(
    ticker: &str,
    mut rows: Vec<DilutedEpsAdjustmentFact>,
) -> Vec<DilutedEpsAdjustmentDbRow> {
    rows.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.adjustment_type.cmp(&b.adjustment_type))
            .then_with(|| a.adjustment_name.cmp(&b.adjustment_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });

    rows.into_iter()
        .map(|row| DilutedEpsAdjustmentDbRow {
            ticker: ticker.to_string(),
            period: row.period,
            adjustment_key: row.adjustment_key,
            adjustment_type: row.adjustment_type,
            adjustment_name: row.adjustment_name,
            terms_text: row.terms_text,
            weighted_average_common_shares: row.weighted_average_common_shares,
            source_concept: row.source_concept,
            source_kind: row.source_kind,
        })
        .collect()
}

struct IdCache {
    data_sources: HashMap<String, i64>,
    source_kinds: HashMap<String, i64>,
    financial_items: HashMap<(String, String), i64>,
    share_classes: HashMap<(String, String), i64>,
}

impl IdCache {
    fn load(conn: &Connection) -> Result<Self, String> {
        let data_sources =
            load_column_map(conn, "SELECT source_id, source_code FROM data_sources")?;
        let source_kinds =
            load_column_map(conn, "SELECT source_kind_id, source_kind FROM source_kinds")?;
        let financial_items = load_pair_map(
            conn,
            "SELECT item_id, statement, item_name FROM financial_item_defs",
        )?;
        let share_classes = load_pair_map_ticker(
            conn,
            "SELECT share_class_id, ticker, class_key FROM share_class_defs",
        )?;
        Ok(Self {
            data_sources,
            source_kinds,
            financial_items,
            share_classes,
        })
    }

    fn ensure_data_source(&mut self, tx: &Transaction<'_>, source: &str) -> Result<i64, String> {
        if let Some(&id) = self.data_sources.get(source) {
            return Ok(id);
        }
        tx.execute(
            "INSERT INTO data_sources (source_code) VALUES (?1)
             ON CONFLICT(source_code) DO NOTHING",
            params![source],
        )
        .map_err(|err| err.to_string())?;
        let id = tx
            .query_row(
                "SELECT source_id FROM data_sources WHERE source_code = ?1",
                params![source],
                |row| row.get(0),
            )
            .map_err(|err| err.to_string())?;
        self.data_sources.insert(source.to_string(), id);
        Ok(id)
    }

    fn ensure_source_kind(
        &mut self,
        tx: &Transaction<'_>,
        source_kind: &str,
    ) -> Result<i64, String> {
        if let Some(&id) = self.source_kinds.get(source_kind) {
            return Ok(id);
        }
        tx.execute(
            "INSERT INTO source_kinds (source_kind) VALUES (?1)
             ON CONFLICT(source_kind) DO NOTHING",
            params![source_kind],
        )
        .map_err(|err| err.to_string())?;
        let id = tx
            .query_row(
                "SELECT source_kind_id FROM source_kinds WHERE source_kind = ?1",
                params![source_kind],
                |row| row.get(0),
            )
            .map_err(|err| err.to_string())?;
        self.source_kinds.insert(source_kind.to_string(), id);
        Ok(id)
    }

    fn ensure_financial_item(
        &mut self,
        tx: &Transaction<'_>,
        statement: &str,
        item_name: &str,
    ) -> Result<i64, String> {
        let key = (statement.to_string(), item_name.to_string());
        if let Some(&id) = self.financial_items.get(&key) {
            return Ok(id);
        }
        tx.execute(
            "INSERT INTO financial_item_defs (statement, item_name) VALUES (?1, ?2)
             ON CONFLICT(statement, item_name) DO NOTHING",
            params![statement, item_name],
        )
        .map_err(|err| err.to_string())?;
        let id = tx
            .query_row(
                "SELECT item_id FROM financial_item_defs WHERE statement = ?1 AND item_name = ?2",
                params![statement, item_name],
                |row| row.get(0),
            )
            .map_err(|err| err.to_string())?;
        self.financial_items.insert(key, id);
        Ok(id)
    }

    fn ensure_share_class(
        &mut self,
        tx: &Transaction<'_>,
        ticker: &str,
        class_key: &str,
        class_name: &str,
        is_preferred: bool,
        updated_at: &str,
    ) -> Result<i64, String> {
        let key = (ticker.to_string(), class_key.to_string());
        if let Some(&id) = self.share_classes.get(&key) {
            tx.execute(
                r#"
                UPDATE share_class_defs
                SET class_name = ?1, is_preferred = ?2, updated_at = ?3
                WHERE share_class_id = ?4
                  AND (class_name <> ?1 OR is_preferred <> ?2)
                "#,
                params![
                    class_name,
                    if is_preferred { 1i64 } else { 0i64 },
                    updated_at,
                    id,
                ],
            )
            .map_err(|err| err.to_string())?;
            return Ok(id);
        }
        tx.execute(
            r#"
            INSERT INTO share_class_defs
                (ticker, class_key, class_name, is_preferred, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(ticker, class_key) DO UPDATE SET
                class_name=excluded.class_name,
                is_preferred=excluded.is_preferred,
                updated_at=excluded.updated_at
            "#,
            params![
                ticker,
                class_key,
                class_name,
                if is_preferred { 1i64 } else { 0i64 },
                updated_at,
            ],
        )
        .map_err(|err| err.to_string())?;
        let id = tx
            .query_row(
                "SELECT share_class_id FROM share_class_defs WHERE ticker = ?1 AND class_key = ?2",
                params![ticker, class_key],
                |row| row.get(0),
            )
            .map_err(|err| err.to_string())?;
        self.share_classes.insert(key, id);
        Ok(id)
    }
}

fn load_column_map(conn: &Connection, sql: &str) -> Result<HashMap<String, i64>, String> {
    let mut map = HashMap::new();
    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| err.to_string())?;
    for row in rows {
        let (id, code) = row.map_err(|err| err.to_string())?;
        map.insert(code, id);
    }
    Ok(map)
}

fn load_pair_map(conn: &Connection, sql: &str) -> Result<HashMap<(String, String), i64>, String> {
    let mut map = HashMap::new();
    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|err| err.to_string())?;
    for row in rows {
        let (id, col1, col2) = row.map_err(|err| err.to_string())?;
        map.insert((col1, col2), id);
    }
    Ok(map)
}

fn load_pair_map_ticker(
    conn: &Connection,
    sql: &str,
) -> Result<HashMap<(String, String), i64>, String> {
    load_pair_map(conn, sql)
}

fn write_ticker_in_tx(
    tx: &Transaction<'_>,
    parsed: &ParsedTicker,
    cache: &mut IdCache,
) -> Result<(), String> {
    let now = utc_now_iso();
    let source_id = cache.ensure_data_source(tx, SOURCE)?;

    tx.execute(
        "DELETE FROM financial_values
         WHERE ticker = ?1
           AND source_id IN (
               SELECT source_id FROM data_sources
               WHERE source_code IN (?2, ?3, ?4, ?5, ?6)
           )",
        params![
            parsed.ticker,
            REPLACED_SOURCES[0],
            REPLACED_SOURCES[1],
            REPLACED_SOURCES[2],
            REPLACED_SOURCES[3],
            REPLACED_SOURCES[4],
        ],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.financial_rows.is_empty() {
        let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(parsed.financial_rows.len() * 6);
        for row in &parsed.financial_rows {
            let item_id = cache.ensure_financial_item(tx, &row.statement, &row.item_name)?;
            flat.push(Box::new(row.ticker.clone()));
            flat.push(Box::new(row.period.clone()));
            flat.push(Box::new(item_id));
            flat.push(Box::new(source_id));
            flat.push(Box::new(row.value));
            flat.push(Box::new(now.clone()));
        }
        chunked_execute(
            tx,
            "INSERT INTO financial_values (ticker, period, item_id, source_id, value, updated_at) VALUES ",
            " ON CONFLICT(ticker, period, item_id, source_id) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            6,
            &flat,
        )?;
    }

    replace_raw_statement_rows(
        tx,
        "xbrl_balance_sheet_facts",
        &parsed.ticker,
        &parsed.raw_balance_sheet_rows,
        source_id,
        &now,
    )?;
    replace_raw_statement_rows(
        tx,
        "xbrl_profit_loss_facts",
        &parsed.ticker,
        &parsed.raw_profit_loss_rows,
        source_id,
        &now,
    )?;
    replace_raw_statement_rows(
        tx,
        "xbrl_cash_flow_facts",
        &parsed.ticker,
        &parsed.raw_cash_flow_rows,
        source_id,
        &now,
    )?;
    replace_statement_presentation_edge_rows(
        tx,
        &parsed.ticker,
        &parsed.statement_presentation_edge_rows,
        source_id,
        &now,
    )?;

    tx.execute(
        "DELETE FROM share_class_issuances
         WHERE source_id = ?1
           AND share_class_id IN (
               SELECT share_class_id FROM share_class_defs WHERE ticker = ?2
           )",
        params![source_id, parsed.ticker],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.share_class_rows.is_empty() {
        let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(parsed.share_class_rows.len() * 6);
        for row in &parsed.share_class_rows {
            let source_kind_id = cache.ensure_source_kind(tx, &row.source_kind)?;
            let share_class_id = cache.ensure_share_class(
                tx,
                &row.ticker,
                &row.class_key,
                &row.class_name,
                row.is_preferred,
                &now,
            )?;
            flat.push(Box::new(row.period.clone()));
            flat.push(Box::new(source_id));
            flat.push(Box::new(share_class_id));
            flat.push(Box::new(row.shares));
            flat.push(Box::new(source_kind_id));
            flat.push(Box::new(now.clone()));
        }
        chunked_execute(
            tx,
            "INSERT INTO share_class_issuances (period, source_id, share_class_id, shares, source_kind_id, updated_at) VALUES ",
            " ON CONFLICT(period, source_id, share_class_id) DO UPDATE SET shares=excluded.shares, source_kind_id=excluded.source_kind_id, updated_at=excluded.updated_at",
            6,
            &flat,
        )?;
    }

    tx.execute(
        "DELETE FROM preferred_share_text_blocks
         WHERE source_id = ?1
           AND share_class_id IN (
               SELECT share_class_id FROM share_class_defs WHERE ticker = ?2
           )",
        params![source_id, parsed.ticker],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.preferred_share_term_rows.is_empty() {
        let mut fact_flat: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for row in &parsed.preferred_share_term_rows {
            let source_kind_id = cache.ensure_source_kind(tx, &row.source_kind)?;
            let share_class_id = cache.ensure_share_class(
                tx,
                &row.ticker,
                &row.class_key,
                &row.class_name,
                true,
                &now,
            )?;
            tx.execute(
                r#"
                INSERT INTO preferred_share_text_blocks
                    (period, source_id, share_class_id, terms_text,
                     source_concept, source_kind_id, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(period, source_id, share_class_id, source_concept) DO UPDATE SET
                    terms_text=excluded.terms_text,
                    source_kind_id=excluded.source_kind_id,
                    updated_at=excluded.updated_at
                "#,
                params![
                    row.period,
                    source_id,
                    share_class_id,
                    row.terms_text,
                    row.source_concept,
                    source_kind_id,
                    now,
                ],
            )
            .map_err(|err| err.to_string())?;

            let text_block_id: i64 = tx
                .query_row(
                    "SELECT text_block_id FROM preferred_share_text_blocks
                     WHERE period = ?1
                       AND source_id = ?2
                       AND share_class_id = ?3
                       AND source_concept = ?4",
                    params![row.period, source_id, share_class_id, row.source_concept],
                    |db_row| db_row.get(0),
                )
                .map_err(|err| err.to_string())?;
            tx.execute(
                "DELETE FROM preferred_share_term_facts WHERE text_block_id = ?1",
                params![text_block_id],
            )
            .map_err(|err| err.to_string())?;

            for (fact_name, value) in [
                ("preferred_shares", row.preferred_shares),
                ("converted_common_shares", row.converted_common_shares),
                ("conversion_ratio_to_common", row.conversion_ratio_to_common),
                ("issue_price_yen", row.issue_price_yen),
                ("capitalized_amount_yen", row.capitalized_amount_yen),
            ] {
                if let Some(v) = value {
                    fact_flat.push(Box::new(text_block_id));
                    fact_flat.push(Box::new(fact_name.to_string()));
                    fact_flat.push(Box::new(v));
                }
            }
        }
        chunked_execute(
            tx,
            "INSERT INTO preferred_share_term_facts (text_block_id, fact_name, value) VALUES ",
            "",
            3,
            &fact_flat,
        )?;
    }

    tx.execute(
        "DELETE FROM potential_equity_instruments_raw
         WHERE ticker = ?1 AND source_id = ?2",
        params![parsed.ticker, source_id],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.potential_equity_term_rows.is_empty() {
        let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(parsed.potential_equity_term_rows.len() * 14);
        for row in &parsed.potential_equity_term_rows {
            let source_kind_id = cache.ensure_source_kind(tx, &row.source_kind)?;
            flat.push(Box::new(row.ticker.clone()));
            flat.push(Box::new(row.period.clone()));
            flat.push(Box::new(source_id));
            flat.push(Box::new(row.instrument_key.clone()));
            flat.push(Box::new(row.instrument_type.clone()));
            flat.push(Box::new(row.instrument_name.clone()));
            flat.push(Box::new(row.terms_text.clone()));
            flat.push(Box::new(row.period_end_common_shares));
            flat.push(Box::new(row.exercise_price_yen));
            flat.push(Box::new(row.conversion_price_yen));
            flat.push(Box::new(row.face_value_yen));
            flat.push(Box::new(row.source_concept.clone()));
            flat.push(Box::new(source_kind_id));
            flat.push(Box::new(now.clone()));
        }
        chunked_execute(
            tx,
            "INSERT INTO potential_equity_instruments_raw (ticker, period, source_id, instrument_key, instrument_type, instrument_name, terms_text, period_end_common_shares, exercise_price_yen, conversion_price_yen, face_value_yen, source_concept, source_kind_id, updated_at) VALUES ",
            " ON CONFLICT(ticker, period, source_id, instrument_key, source_concept, terms_text) DO UPDATE SET instrument_type=excluded.instrument_type, instrument_name=excluded.instrument_name, terms_text=excluded.terms_text, period_end_common_shares=excluded.period_end_common_shares, exercise_price_yen=excluded.exercise_price_yen, conversion_price_yen=excluded.conversion_price_yen, face_value_yen=excluded.face_value_yen, source_kind_id=excluded.source_kind_id, updated_at=excluded.updated_at",
            14,
            &flat,
        )?;
    }

    tx.execute(
        "DELETE FROM potential_equity_status_raw
         WHERE ticker = ?1 AND source_id = ?2",
        params![parsed.ticker, source_id],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.potential_equity_status_rows.is_empty() {
        let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(parsed.potential_equity_status_rows.len() * 9);
        for row in &parsed.potential_equity_status_rows {
            let source_kind_id = cache.ensure_source_kind(tx, &row.source_kind)?;
            flat.push(Box::new(row.ticker.clone()));
            flat.push(Box::new(row.period.clone()));
            flat.push(Box::new(source_id));
            flat.push(Box::new(row.status_key.clone()));
            flat.push(Box::new(row.has_potential_equity));
            flat.push(Box::new(row.terms_text.clone()));
            flat.push(Box::new(row.source_concept.clone()));
            flat.push(Box::new(source_kind_id));
            flat.push(Box::new(now.clone()));
        }
        chunked_execute(
            tx,
            "INSERT INTO potential_equity_status_raw (ticker, period, source_id, status_key, has_potential_equity, terms_text, source_concept, source_kind_id, updated_at) VALUES ",
            " ON CONFLICT(ticker, period, source_id, status_key, source_concept, terms_text) DO UPDATE SET has_potential_equity=excluded.has_potential_equity, terms_text=excluded.terms_text, source_kind_id=excluded.source_kind_id, updated_at=excluded.updated_at",
            9,
            &flat,
        )?;
    }

    tx.execute(
        "DELETE FROM diluted_eps_adjustments_raw
         WHERE ticker = ?1 AND source_id = ?2",
        params![parsed.ticker, source_id],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.diluted_eps_adjustment_rows.is_empty() {
        let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(parsed.diluted_eps_adjustment_rows.len() * 11);
        for row in &parsed.diluted_eps_adjustment_rows {
            let source_kind_id = cache.ensure_source_kind(tx, &row.source_kind)?;
            flat.push(Box::new(row.ticker.clone()));
            flat.push(Box::new(row.period.clone()));
            flat.push(Box::new(source_id));
            flat.push(Box::new(row.adjustment_key.clone()));
            flat.push(Box::new(row.adjustment_type.clone()));
            flat.push(Box::new(row.adjustment_name.clone()));
            flat.push(Box::new(row.terms_text.clone()));
            flat.push(Box::new(row.weighted_average_common_shares));
            flat.push(Box::new(row.source_concept.clone()));
            flat.push(Box::new(source_kind_id));
            flat.push(Box::new(now.clone()));
        }
        chunked_execute(
            tx,
            "INSERT INTO diluted_eps_adjustments_raw (ticker, period, source_id, adjustment_key, adjustment_type, adjustment_name, terms_text, weighted_average_common_shares, source_concept, source_kind_id, updated_at) VALUES ",
            " ON CONFLICT(ticker, period, source_id, adjustment_key, source_concept, terms_text) DO UPDATE SET adjustment_type=excluded.adjustment_type, adjustment_name=excluded.adjustment_name, terms_text=excluded.terms_text, weighted_average_common_shares=excluded.weighted_average_common_shares, source_kind_id=excluded.source_kind_id, updated_at=excluded.updated_at",
            11,
            &flat,
        )?;
    }

    Ok(())
}

fn replace_raw_statement_rows(
    tx: &Transaction<'_>,
    table: &str,
    ticker: &str,
    rows: &[RawStatementDbRow],
    source_id: i64,
    updated_at: &str,
) -> Result<(), String> {
    let delete_sql = format!(
        "DELETE FROM {table}
         WHERE ticker = ?1
           AND source_id IN (
               SELECT source_id FROM data_sources
               WHERE source_code IN (?2, ?3, ?4, ?5, ?6)
           )"
    );
    tx.execute(
        &delete_sql,
        params![
            ticker,
            REPLACED_SOURCES[0],
            REPLACED_SOURCES[1],
            REPLACED_SOURCES[2],
            REPLACED_SOURCES[3],
            REPLACED_SOURCES[4],
        ],
    )
    .map_err(|err| err.to_string())?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(rows.len() * 21);
    for row in rows {
        flat.push(Box::new(row.ticker.clone()));
        flat.push(Box::new(row.doc_id.clone()));
        flat.push(Box::new(row.period.clone()));
        flat.push(Box::new(row.period_start.clone()));
        flat.push(Box::new(row.period_end.clone()));
        flat.push(Box::new(row.duration_days));
        flat.push(Box::new(source_id));
        flat.push(Box::new(row.consolidation_scope.clone()));
        flat.push(Box::new(row.context_id.clone()));
        flat.push(Box::new(row.dimensions_json.clone()));
        flat.push(Box::new(row.concept_namespace.clone()));
        flat.push(Box::new(row.concept_name.clone()));
        flat.push(Box::new(row.canonical_statement.clone()));
        flat.push(Box::new(row.canonical_item_name.clone()));
        flat.push(Box::new(row.taxonomy_status.clone()));
        flat.push(Box::new(row.role_uri.clone()));
        flat.push(Box::new(row.unit.clone()));
        flat.push(Box::new(row.unit_ref.clone()));
        flat.push(Box::new(row.decimals.clone()));
        flat.push(Box::new(row.value));
        flat.push(Box::new(updated_at.to_string()));
    }

    let insert_prefix = format!(
        "INSERT INTO {table} (ticker, doc_id, period, period_start, period_end, duration_days, source_id, consolidation_scope, context_id, dimensions_json, concept_namespace, concept_name, canonical_statement, canonical_item_name, taxonomy_status, role_uri, unit, unit_ref, decimals, value, updated_at) VALUES "
    );
    chunked_execute(
        tx,
        &insert_prefix,
        " ON CONFLICT(ticker, doc_id, period, source_id, consolidation_scope, context_id, concept_namespace, concept_name, role_uri) DO UPDATE SET period_start=excluded.period_start, period_end=excluded.period_end, duration_days=excluded.duration_days, dimensions_json=excluded.dimensions_json, canonical_statement=excluded.canonical_statement, canonical_item_name=excluded.canonical_item_name, taxonomy_status=excluded.taxonomy_status, unit=excluded.unit, unit_ref=excluded.unit_ref, decimals=excluded.decimals, value=excluded.value, updated_at=excluded.updated_at",
        21,
        &flat,
    )
}

fn replace_statement_presentation_edge_rows(
    tx: &Transaction<'_>,
    ticker: &str,
    rows: &[StatementPresentationEdgeDbRow],
    source_id: i64,
    updated_at: &str,
) -> Result<(), String> {
    tx.execute(
        "DELETE FROM xbrl_statement_presentation_edges
         WHERE ticker = ?1
           AND source_id IN (
               SELECT source_id FROM data_sources
               WHERE source_code IN (?2, ?3, ?4, ?5, ?6)
           )",
        params![
            ticker,
            REPLACED_SOURCES[0],
            REPLACED_SOURCES[1],
            REPLACED_SOURCES[2],
            REPLACED_SOURCES[3],
            REPLACED_SOURCES[4],
        ],
    )
    .map_err(|err| err.to_string())?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut flat: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(rows.len() * 13);
    for row in rows {
        flat.push(Box::new(row.ticker.clone()));
        flat.push(Box::new(row.doc_id.clone()));
        flat.push(Box::new(row.statement.clone()));
        flat.push(Box::new(source_id));
        flat.push(Box::new(row.role_uri.clone()));
        flat.push(Box::new(row.parent_namespace.clone()));
        flat.push(Box::new(row.parent_name.clone()));
        flat.push(Box::new(row.parent_label.clone()));
        flat.push(Box::new(row.child_namespace.clone()));
        flat.push(Box::new(row.child_name.clone()));
        flat.push(Box::new(row.child_order));
        flat.push(Box::new(row.child_label.clone()));
        flat.push(Box::new(updated_at.to_string()));
    }

    chunked_execute(
        tx,
        "INSERT INTO xbrl_statement_presentation_edges (ticker, doc_id, statement, source_id, role_uri, parent_namespace, parent_name, parent_label, child_namespace, child_name, child_order, child_label, updated_at) VALUES ",
        " ON CONFLICT(ticker, doc_id, statement, source_id, role_uri, parent_namespace, parent_name, child_namespace, child_name) DO UPDATE SET parent_label=excluded.parent_label, child_order=excluded.child_order, child_label=excluded.child_label, updated_at=excluded.updated_at",
        13,
        &flat,
    )
}

fn utc_now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false)
}

fn chunked_execute(
    tx: &Transaction<'_>,
    base_sql: &str,
    suffix_sql: &str,
    param_count: usize,
    flat_params: &[Box<dyn rusqlite::types::ToSql>],
) -> Result<(), String> {
    if flat_params.is_empty() {
        return Ok(());
    }
    const CHUNK_ROWS: usize = 100;
    let row_placeholder = format!("({})", vec!["?"; param_count].join(", "));
    for chunk in flat_params.chunks(CHUNK_ROWS * param_count) {
        let n_rows = chunk.len() / param_count;
        let placeholders = vec![row_placeholder.as_str(); n_rows].join(", ");
        let sql = format!("{base_sql}{placeholders}{suffix_sql}");
        let mut stmt = tx.prepare(&sql).map_err(|err| err.to_string())?;
        stmt.execute(rusqlite::params_from_iter(chunk.iter().map(|p| p.as_ref())))
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
