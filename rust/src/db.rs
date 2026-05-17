use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use rayon::prelude::*;
use rusqlite::{Connection, params};

use crate::artifact;
use crate::financials;
use crate::share_classes;
use crate::types::ShareClassFact;

const SOURCE: &str = "edinet_xbrl";
const REPLACED_SOURCES: [&str; 5] = [
    "edinet_xbrl",
    "irbank",
    "irbank_bs",
    "irbank_forecast",
    "xbrl_bs",
];

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
pub struct ParsedTicker {
    pub ticker: String,
    pub financial_rows: Vec<FinancialDbRow>,
    pub share_class_rows: Vec<ShareClassDbRow>,
    pub period_count: usize,
}

#[derive(Debug, Clone)]
pub struct TickerSummary {
    pub ticker: String,
    pub status: String,
    pub financial_rows: usize,
    pub period_count: usize,
    pub share_class_rows: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseDbSummary {
    pub ok: usize,
    pub errors: usize,
    pub skipped: usize,
    pub no_xbrl_files: bool,
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
) -> Result<ParseDbSummary, String> {
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut conn = Connection::open(db_path).map_err(|err| err.to_string())?;
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

    let outcomes: Vec<TickerParseOutcome> = tickers
        .par_iter()
        .map(|input| match parse_ticker(input) {
            Ok(parsed) => TickerParseOutcome::Parsed(parsed),
            Err(TickerParseError::Inventory(message)) => TickerParseOutcome::InventoryError {
                ticker: input.ticker.clone(),
                message,
            },
            Err(TickerParseError::Fatal(message)) => TickerParseOutcome::Fatal {
                ticker: input.ticker.clone(),
                message,
            },
        })
        .collect();

    if let Some((ticker, message)) = outcomes.iter().find_map(|outcome| match outcome {
        TickerParseOutcome::Fatal { ticker, message } => Some((ticker, message)),
        _ => None,
    }) {
        return Err(format!("{ticker}: {message}"));
    }

    let mut ok = 0usize;
    let mut errors = 0usize;
    let mut results = Vec::with_capacity(outcomes.len());

    for outcome in outcomes {
        match outcome {
            TickerParseOutcome::Parsed(parsed) => {
                if parsed.financial_rows.is_empty() && parsed.share_class_rows.is_empty() {
                    results.push(TickerSummary {
                        ticker: parsed.ticker,
                        status: "no_facts".to_string(),
                        financial_rows: 0,
                        period_count: 0,
                        share_class_rows: 0,
                        message: None,
                    });
                    continue;
                }
                write_ticker(&mut conn, &parsed)?;
                ok += 1;
                results.push(TickerSummary {
                    ticker: parsed.ticker,
                    status: "ok".to_string(),
                    financial_rows: parsed.financial_rows.len(),
                    period_count: parsed.period_count,
                    share_class_rows: parsed.share_class_rows.len(),
                    message: None,
                });
            }
            TickerParseOutcome::InventoryError { ticker, message } => {
                errors += 1;
                results.push(TickerSummary {
                    ticker,
                    status: "error".to_string(),
                    financial_rows: 0,
                    period_count: 0,
                    share_class_rows: 0,
                    message: Some(message),
                });
            }
            TickerParseOutcome::Fatal { .. } => {
                unreachable!("fatal errors are returned before writes")
            }
        }
    }

    Ok(ParseDbSummary {
        ok,
        errors,
        skipped,
        no_xbrl_files: false,
        results,
    })
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
        .prepare("SELECT DISTINCT ticker FROM financial_items WHERE source = ?")
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

    for report in &reports {
        let artifact =
            artifact::load_xbrl_artifact(&report.xbrl_path).map_err(TickerParseError::Fatal)?;
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
    }

    let period_count = merged.len();
    let financial_rows = build_financial_rows(&input.ticker, &merged);
    let share_class_rows =
        build_share_class_rows(&input.ticker, share_classes_by_key.into_values().collect());

    Ok(ParsedTicker {
        ticker: input.ticker.clone(),
        financial_rows,
        share_class_rows,
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

fn write_ticker(conn: &mut Connection, parsed: &ParsedTicker) -> Result<(), String> {
    let now = utc_now_iso();
    let tx = conn.transaction().map_err(|err| err.to_string())?;

    tx.execute(
        "DELETE FROM financial_items WHERE ticker = ? AND source IN (?2, ?3, ?4, ?5, ?6)",
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
        let mut stmt = tx
            .prepare(
                r#"
                INSERT INTO financial_items
                    (ticker, period, statement, item_name, value, source, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(ticker, period, statement, item_name) DO UPDATE SET
                    value=excluded.value,
                    source=excluded.source,
                    updated_at=excluded.updated_at
                "#,
            )
            .map_err(|err| err.to_string())?;
        for row in &parsed.financial_rows {
            stmt.execute(params![
                row.ticker,
                row.period,
                row.statement,
                row.item_name,
                row.value,
                SOURCE,
                now,
            ])
            .map_err(|err| err.to_string())?;
        }
    }

    tx.execute(
        "DELETE FROM share_classes WHERE ticker = ? AND source = ?",
        params![parsed.ticker, SOURCE],
    )
    .map_err(|err| err.to_string())?;

    if !parsed.share_class_rows.is_empty() {
        let mut stmt = tx
            .prepare(
                r#"
                INSERT INTO share_classes
                    (
                        ticker,
                        period,
                        source,
                        class_key,
                        class_name,
                        shares,
                        is_preferred,
                        source_kind,
                        updated_at
                    )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(ticker, period, source, class_key) DO UPDATE SET
                    class_name=excluded.class_name,
                    shares=excluded.shares,
                    is_preferred=excluded.is_preferred,
                    source_kind=excluded.source_kind,
                    updated_at=excluded.updated_at
                "#,
            )
            .map_err(|err| err.to_string())?;
        for row in &parsed.share_class_rows {
            stmt.execute(params![
                row.ticker,
                row.period,
                SOURCE,
                row.class_key,
                row.class_name,
                row.shares,
                if row.is_preferred { 1i64 } else { 0i64 },
                row.source_kind,
                now,
            ])
            .map_err(|err| err.to_string())?;
        }
    }

    tx.commit().map_err(|err| err.to_string())
}

fn utc_now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false)
}
