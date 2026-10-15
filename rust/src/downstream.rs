use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{config, paths, screening};

#[derive(Debug, Deserialize)]
struct TickerRequest {
    tickers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PriceRefreshCommandResult {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize)]
pub struct StockPriceMetadata {
    pub price_date: Option<String>,
    pub target_price_date: String,
}

#[derive(Debug, Serialize)]
pub struct StockXbrlArtifact {
    pub doc_id: String,
    pub xbrl_path: String,
    pub source_size: u64,
    pub source_mtime_ns: i128,
}

#[derive(Debug, Serialize)]
pub struct BalanceSheetNode {
    pub concept_namespace: String,
    pub concept_name: String,
    pub label: String,
    pub values: Vec<Option<f64>>,
    pub children: Vec<BalanceSheetNode>,
}

#[derive(Debug, Serialize)]
pub struct BalanceSheetHistory {
    pub ticker: String,
    pub periods: Vec<String>,
    pub baseline_period: Option<String>,
    pub baseline_total_assets: Option<f64>,
    pub total_assets: Vec<Option<f64>>,
    pub unit: String,
    pub roots: Vec<BalanceSheetNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug)]
struct EdgeRow {
    role_uri: String,
    parent_namespace: String,
    parent_name: String,
    parent_label: String,
    child_namespace: String,
    child_name: String,
    child_label: String,
    child_order: f64,
}

#[derive(Debug)]
struct ArtifactFingerprint {
    source_size: u64,
    source_mtime_ns: i128,
}

type NodeKey = (String, String, String);

pub fn default_stocks_db_path() -> PathBuf {
    paths::stocks_db_path()
}

pub fn read_tickers_from_stdin() -> Result<Vec<String>, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("read stdin: {err}"))?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(trimmed)
            .map(normalize_tickers)
            .map_err(|err| format!("parse ticker list JSON: {err}"));
    }
    serde_json::from_str::<TickerRequest>(trimmed)
        .map(|request| normalize_tickers(request.tickers))
        .map_err(|err| format!("parse ticker request JSON: {err}"))
}

pub fn write_json_stdout<T: Serialize>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, value).map_err(|err| err.to_string())?;
    handle.write_all(b"\n").map_err(|err| err.to_string())
}

pub fn open_existing_stock_db(db_path: Option<PathBuf>) -> Result<Connection, String> {
    let path = db_path.unwrap_or_else(default_stocks_db_path);
    if !path.is_file() {
        return Err(format!("stocks DB does not exist: {}", path.display()));
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    conn.busy_timeout(StdDuration::from_secs(60))
        .map_err(|err| err.to_string())?;
    Ok(conn)
}

pub fn get_all_tickers(db_path: Option<PathBuf>) -> Result<Vec<String>, String> {
    let conn = open_existing_stock_db(db_path)?;
    let mut stmt = conn
        .prepare("SELECT ticker FROM stocks ORDER BY ticker")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

pub fn get_stock_names(db_path: Option<PathBuf>) -> Result<BTreeMap<String, String>, String> {
    let conn = open_existing_stock_db(db_path)?;
    let mut stmt = conn
        .prepare("SELECT ticker, name FROM stocks ORDER BY ticker")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|err| err.to_string())
}

pub fn get_screening_tickers(
    db_path: Option<PathBuf>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let conn = open_existing_stock_db(db_path)?;
    let body = r#"
        SELECT s.ticker
        FROM stocks AS s
        WHERE EXISTS (SELECT 1 FROM prices AS p WHERE p.ticker = s.ticker)
          AND EXISTS (
              SELECT 1
              FROM canonical_financial_items AS f
              WHERE f.ticker = s.ticker
                AND f.statement = 'pl'
          )
        ORDER BY s.ticker
    "#;
    match limit {
        Some(value) => {
            let sql = format!("{body} LIMIT ?1");
            let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
            let rows = stmt
                .query_map(params![value as i64], |row| row.get::<_, String>(0))
                .map_err(|err| err.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|err| err.to_string())
        }
        None => {
            let mut stmt = conn.prepare(body).map_err(|err| err.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|err| err.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|err| err.to_string())
        }
    }
}

pub fn load_screening_stocks(
    db_path: Option<PathBuf>,
    tickers: Option<&[String]>,
    fcf_periods: usize,
    pl_periods: usize,
    payout_periods: usize,
) -> Result<Vec<screening::ScreeningStock>, String> {
    let path = db_path.unwrap_or_else(default_stocks_db_path);
    if !path.is_file() {
        return Err(format!("stocks DB does not exist: {}", path.display()));
    }
    screening::load_screening_stocks_from_path(
        &path,
        tickers,
        fcf_periods,
        pl_periods,
        payout_periods,
    )
}

pub fn get_stock_price_metadata(db_path: Option<PathBuf>) -> Result<StockPriceMetadata, String> {
    let conn = open_existing_stock_db(db_path)?;
    let latest_date = conn
        .query_row("SELECT MAX(date) FROM prices", [], |row| {
            row.get::<_, Option<String>>(0)
        })
        .map_err(|err| err.to_string())?;
    let target_price_date = previous_jpx_business_day(None)?;
    Ok(StockPriceMetadata {
        price_date: latest_date,
        target_price_date: target_price_date.to_string(),
    })
}

pub fn get_stock_market_caps(
    db_path: Option<PathBuf>,
    tickers: &[String],
    max_age_days: i64,
) -> Result<BTreeMap<String, i64>, String> {
    if max_age_days < 0 {
        return Err("max_age_days must be >= 0".to_string());
    }
    let selected = normalize_tickers(tickers.iter().cloned());
    if selected.is_empty() {
        return Ok(BTreeMap::new());
    }
    let conn = open_existing_stock_db(db_path)?;
    let cutoff_date = today_jst() - Duration::days(max_age_days);
    let mut result = BTreeMap::new();
    for ticker in selected {
        if let Some((shares_outstanding, close, price_date)) =
            latest_market_cap_inputs(&conn, &ticker)?
        {
            let parsed_price_date = NaiveDate::parse_from_str(&price_date, "%Y-%m-%d")
                .map_err(|err| format!("invalid price date for {ticker}: {err}"))?;
            if parsed_price_date >= cutoff_date {
                result.insert(ticker, (shares_outstanding as f64 * close).round() as i64);
            }
        }
    }
    Ok(result)
}

pub fn get_latest_xbrl_artifacts(
    db_path: Option<PathBuf>,
    tickers: &[String],
) -> Result<BTreeMap<String, StockXbrlArtifact>, String> {
    let selected = normalize_tickers(tickers.iter().cloned());
    if selected.is_empty() {
        return Ok(BTreeMap::new());
    }
    let conn = open_existing_stock_db(db_path)?;
    let mut result = BTreeMap::new();
    for ticker in selected {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT ticker, doc_id, xbrl_path
                FROM sec_reports
                WHERE ticker = ?1
                  AND COALESCE(doc_id, '') <> ''
                  AND COALESCE(xbrl_path, '') <> ''
                ORDER BY
                    CASE WHEN fiscal_year = 'latest' THEN 0 ELSE 1 END,
                    updated_at DESC,
                    doc_id DESC
                "#,
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![ticker], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|err| err.to_string())?;
        for row in rows {
            let (row_ticker, doc_id, xbrl_path) = row.map_err(|err| err.to_string())?;
            if let Some(fingerprint) = fingerprint_xbrl_artifact(Path::new(&xbrl_path))? {
                let resolved = std::fs::canonicalize(&xbrl_path)
                    .map_err(|err| format!("canonicalize {xbrl_path}: {err}"))?;
                result.insert(
                    row_ticker,
                    StockXbrlArtifact {
                        doc_id,
                        xbrl_path: resolved.to_string_lossy().to_string(),
                        source_size: fingerprint.source_size,
                        source_mtime_ns: fingerprint.source_mtime_ns,
                    },
                );
                break;
            }
        }
    }
    Ok(result)
}

pub fn get_balance_sheet_histories(
    db_path: Option<PathBuf>,
    tickers: &[String],
    n_periods: usize,
    source: &str,
) -> Result<BTreeMap<String, BalanceSheetHistory>, String> {
    let selected = normalize_tickers(tickers.iter().cloned());
    let conn = open_existing_stock_db(db_path)?;
    let mut result = BTreeMap::new();
    for ticker in selected {
        result.insert(
            ticker.clone(),
            get_balance_sheet_history_for_conn(&conn, &ticker, n_periods, source)?,
        );
    }
    Ok(result)
}

pub fn get_balance_sheet_history(
    db_path: Option<PathBuf>,
    ticker: &str,
    n_periods: usize,
    source: &str,
) -> Result<BalanceSheetHistory, String> {
    let conn = open_existing_stock_db(db_path)?;
    get_balance_sheet_history_for_conn(&conn, ticker, n_periods, source)
}

fn latest_market_cap_inputs(
    conn: &Connection,
    ticker: &str,
) -> Result<Option<(i64, f64, String)>, String> {
    conn.query_row(
        r#"
        WITH latest_price AS (
            SELECT ticker, date, close
            FROM prices
            WHERE ticker = ?1
              AND close IS NOT NULL
            ORDER BY date DESC
            LIMIT 1
        )
        SELECT s.shares_outstanding, lp.close, lp.date
        FROM stocks AS s
        JOIN latest_price AS lp
          ON lp.ticker = s.ticker
        WHERE s.ticker = ?1
          AND s.shares_outstanding IS NOT NULL
        "#,
        params![ticker],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn get_balance_sheet_history_for_conn(
    conn: &Connection,
    ticker: &str,
    n_periods: usize,
    source: &str,
) -> Result<BalanceSheetHistory, String> {
    let periods = get_balance_sheet_periods(conn, ticker, source, n_periods)?;
    if periods.is_empty() {
        return Ok(empty_balance_sheet_history(
            ticker,
            "xbrl_balance_sheet_missing",
        ));
    }

    let mut selected = BTreeMap::new();
    for period in &periods {
        let Some(doc_scope) = select_balance_sheet_doc_scope(conn, ticker, period, source)? else {
            return Ok(empty_balance_sheet_history(
                ticker,
                "xbrl_balance_sheet_scope_missing",
            ));
        };
        selected.insert(period.clone(), doc_scope);
    }

    let latest_period = periods.last().expect("periods are non-empty");
    let (latest_doc_id, _) = selected
        .get(latest_period)
        .expect("selected doc scope exists for latest period");
    let edge_rows = get_balance_sheet_presentation_edges(conn, ticker, latest_doc_id, source)?;
    if edge_rows.is_empty() {
        return Ok(empty_balance_sheet_history(
            ticker,
            "xbrl_presentation_missing",
        ));
    }

    let values = get_balance_sheet_values(conn, ticker, &periods, &selected, source)?;
    let total_assets = get_total_assets_by_period(conn, ticker, &periods, source)?;
    let totals = periods
        .iter()
        .map(|period| total_assets.get(period).copied().flatten())
        .collect::<Vec<_>>();
    let baseline_total_assets = totals.first().copied().flatten();
    if baseline_total_assets.is_none() {
        return Ok(empty_balance_sheet_history(ticker, "total_assets_missing"));
    }

    let roots = build_balance_sheet_tree(&edge_rows, &values, periods.len());
    if roots.is_empty() {
        return Ok(empty_balance_sheet_history(
            ticker,
            "xbrl_presentation_values_missing",
        ));
    }

    Ok(BalanceSheetHistory {
        ticker: ticker.to_string(),
        periods: periods.clone(),
        baseline_period: periods.first().cloned(),
        baseline_total_assets,
        total_assets: totals,
        unit: "JPY".to_string(),
        roots,
        error: None,
    })
}

fn empty_balance_sheet_history(ticker: &str, error: &str) -> BalanceSheetHistory {
    BalanceSheetHistory {
        ticker: ticker.to_string(),
        periods: Vec::new(),
        baseline_period: None,
        baseline_total_assets: None,
        total_assets: Vec::new(),
        unit: "JPY".to_string(),
        roots: Vec::new(),
        error: Some(error.to_string()),
    }
}

fn get_balance_sheet_periods(
    conn: &Connection,
    ticker: &str,
    source: &str,
    n_periods: usize,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT DISTINCT f.period
            FROM xbrl_balance_sheet_facts AS f
            JOIN data_sources AS ds
              ON ds.source_id = f.source_id
            WHERE f.ticker = ?1
              AND ds.source_code = ?2
            ORDER BY f.period DESC
            LIMIT ?3
            "#,
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, source, n_periods as i64], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|err| err.to_string())?;
    let mut periods = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    periods.reverse();
    Ok(periods)
}

fn select_balance_sheet_doc_scope(
    conn: &Connection,
    ticker: &str,
    period: &str,
    source: &str,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        r#"
        SELECT
            f.doc_id,
            f.consolidation_scope,
            COUNT(*) AS fact_count,
            SUM(CASE WHEN f.role_uri <> '' THEN 1 ELSE 0 END) AS role_fact_count
        FROM xbrl_balance_sheet_facts AS f
        JOIN data_sources AS ds
          ON ds.source_id = f.source_id
        WHERE f.ticker = ?1
          AND f.period = ?2
          AND ds.source_code = ?3
          AND f.value IS NOT NULL
        GROUP BY f.doc_id, f.consolidation_scope
        ORDER BY
            CASE
                WHEN f.consolidation_scope = 'consolidated'
                 AND SUM(CASE WHEN f.role_uri <> '' THEN 1 ELSE 0 END) > 0 THEN 0
                WHEN f.consolidation_scope = 'non_consolidated'
                 AND SUM(CASE WHEN f.role_uri <> '' THEN 1 ELSE 0 END) > 0 THEN 1
                WHEN f.consolidation_scope = 'consolidated' THEN 2
                ELSE 3
            END,
            role_fact_count DESC,
            fact_count DESC,
            f.doc_id DESC
        LIMIT 1
        "#,
        params![ticker, period, source],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn get_balance_sheet_presentation_edges(
    conn: &Connection,
    ticker: &str,
    doc_id: &str,
    source: &str,
) -> Result<Vec<EdgeRow>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                e.role_uri,
                e.parent_namespace,
                e.parent_name,
                e.parent_label,
                e.child_namespace,
                e.child_name,
                e.child_label,
                e.child_order
            FROM xbrl_statement_presentation_edges AS e
            JOIN data_sources AS ds
              ON ds.source_id = e.source_id
            WHERE e.ticker = ?1
              AND e.doc_id = ?2
              AND e.statement = 'bs'
              AND ds.source_code = ?3
            ORDER BY e.role_uri, e.parent_name, e.child_order, e.child_name
            "#,
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, doc_id, source], |row| {
            Ok(EdgeRow {
                role_uri: row.get(0)?,
                parent_namespace: row.get(1)?,
                parent_name: row.get(2)?,
                parent_label: row.get(3)?,
                child_namespace: row.get(4)?,
                child_name: row.get(5)?,
                child_label: row.get(6)?,
                child_order: row.get(7)?,
            })
        })
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

fn get_balance_sheet_values(
    conn: &Connection,
    ticker: &str,
    periods: &[String],
    selected: &BTreeMap<String, (String, String)>,
    source: &str,
) -> Result<BTreeMap<(String, String), Vec<Option<f64>>>, String> {
    let mut values: BTreeMap<(String, String), Vec<Option<f64>>> = BTreeMap::new();
    for (index, period) in periods.iter().enumerate() {
        let (doc_id, scope) = selected
            .get(period)
            .ok_or_else(|| format!("missing selected doc scope for period {period}"))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    f.concept_namespace,
                    f.concept_name,
                    f.value
                FROM xbrl_balance_sheet_facts AS f
                JOIN data_sources AS ds
                  ON ds.source_id = f.source_id
                WHERE f.ticker = ?1
                  AND f.period = ?2
                  AND f.doc_id = ?3
                  AND f.consolidation_scope = ?4
                  AND ds.source_code = ?5
                ORDER BY
                    CASE WHEN f.dimensions_json = '[]' THEN 0 ELSE 1 END,
                    LENGTH(f.dimensions_json),
                    CASE WHEN f.role_uri <> '' THEN 0 ELSE 1 END,
                    f.context_id
                "#,
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![ticker, period, doc_id, scope, source], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                ))
            })
            .map_err(|err| err.to_string())?;
        for row in rows {
            let (namespace, name, value) = row.map_err(|err| err.to_string())?;
            let bucket = values
                .entry((namespace, name))
                .or_insert_with(|| vec![None; periods.len()]);
            if bucket[index].is_none() {
                bucket[index] = value;
            }
        }
    }
    Ok(values)
}

fn get_total_assets_by_period(
    conn: &Connection,
    ticker: &str,
    periods: &[String],
    source: &str,
) -> Result<BTreeMap<String, Option<f64>>, String> {
    let mut result = BTreeMap::new();
    for period in periods {
        let value = conn
            .query_row(
                r#"
                SELECT value
                FROM financial_items
                WHERE ticker = ?1
                  AND source = ?2
                  AND statement = 'bs'
                  AND item_name = 'total_assets'
                  AND period = ?3
                LIMIT 1
                "#,
                params![ticker, source, period],
                |row| row.get::<_, Option<f64>>(0),
            )
            .optional()
            .map_err(|err| err.to_string())?
            .flatten();
        result.insert(period.clone(), value);
    }
    Ok(result)
}

fn build_balance_sheet_tree(
    edge_rows: &[EdgeRow],
    values: &BTreeMap<(String, String), Vec<Option<f64>>>,
    period_count: usize,
) -> Vec<BalanceSheetNode> {
    let mut labels: BTreeMap<NodeKey, String> = BTreeMap::new();
    let mut children_by_parent: BTreeMap<NodeKey, Vec<(f64, NodeKey)>> = BTreeMap::new();
    let mut parents: BTreeSet<NodeKey> = BTreeSet::new();
    let mut children: BTreeSet<NodeKey> = BTreeSet::new();

    for row in edge_rows {
        let parent = (
            row.role_uri.clone(),
            row.parent_namespace.clone(),
            row.parent_name.clone(),
        );
        let child = (
            row.role_uri.clone(),
            row.child_namespace.clone(),
            row.child_name.clone(),
        );
        labels.insert(
            parent.clone(),
            non_empty_label_or_name(&row.parent_label, &row.parent_name),
        );
        labels.insert(
            child.clone(),
            non_empty_label_or_name(&row.child_label, &row.child_name),
        );
        parents.insert(parent.clone());
        children.insert(child.clone());
        children_by_parent
            .entry(parent)
            .or_default()
            .push((row.child_order, child));
    }

    let mut roots = parents.difference(&children).cloned().collect::<Vec<_>>();
    roots.sort_by(|left, right| node_sort_key(left, &labels).cmp(&node_sort_key(right, &labels)));
    roots
        .iter()
        .filter_map(|root| {
            build_balance_sheet_node(root, &labels, &children_by_parent, values, period_count)
        })
        .collect()
}

fn build_balance_sheet_node(
    key: &NodeKey,
    labels: &BTreeMap<NodeKey, String>,
    children_by_parent: &BTreeMap<NodeKey, Vec<(f64, NodeKey)>>,
    values: &BTreeMap<(String, String), Vec<Option<f64>>>,
    period_count: usize,
) -> Option<BalanceSheetNode> {
    let mut child_specs = children_by_parent.get(key).cloned().unwrap_or_default();
    child_specs.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| node_sort_key(&left.1, labels).cmp(&node_sort_key(&right.1, labels)))
    });
    let children = child_specs
        .iter()
        .filter_map(|(_, child)| {
            build_balance_sheet_node(child, labels, children_by_parent, values, period_count)
        })
        .collect::<Vec<_>>();
    let (_role_uri, namespace, name) = key;
    let node_values = values
        .get(&(namespace.clone(), name.clone()))
        .cloned()
        .unwrap_or_else(|| vec![None; period_count]);
    if !node_values.iter().any(Option::is_some) && children.is_empty() {
        return None;
    }
    Some(BalanceSheetNode {
        concept_namespace: namespace.clone(),
        concept_name: name.clone(),
        label: labels.get(key).cloned().unwrap_or_else(|| name.clone()),
        values: node_values,
        children,
    })
}

fn node_sort_key(key: &NodeKey, labels: &BTreeMap<NodeKey, String>) -> (String, String, String) {
    (
        key.0.clone(),
        labels.get(key).cloned().unwrap_or_else(|| key.2.clone()),
        key.2.clone(),
    )
}

fn non_empty_label_or_name(value: &str, name: &str) -> String {
    if value.is_empty() {
        name.to_string()
    } else {
        value.to_string()
    }
}

fn fingerprint_xbrl_artifact(path: &Path) -> Result<Option<ArtifactFingerprint>, String> {
    let zip_path = path.with_extension("zip");
    if !path.is_dir() || !zip_path.is_file() {
        return Ok(None);
    }

    let zip_meta = std::fs::metadata(&zip_path)
        .map_err(|err| format!("metadata {}: {err}", zip_path.display()))?;
    let mut source_size = zip_meta.size();
    let mut source_mtime_ns = metadata_mtime_ns(&zip_meta);
    let mut has_xbrl_body = false;

    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).map_err(|err| format!("read_dir {}: {err}", dir.display()))?
        {
            let entry = entry.map_err(|err| err.to_string())?;
            let child_path = entry.path();
            let meta = entry
                .metadata()
                .map_err(|err| format!("metadata {}: {err}", child_path.display()))?;
            if meta.is_dir() {
                stack.push(child_path);
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            source_size += meta.size();
            source_mtime_ns = source_mtime_ns.max(metadata_mtime_ns(&meta));
            if matches!(
                child_path
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.to_ascii_lowercase())
                    .as_deref(),
                Some("xbrl" | "xhtml" | "html" | "htm")
            ) {
                has_xbrl_body = true;
            }
        }
    }

    if !has_xbrl_body {
        return Ok(None);
    }
    Ok(Some(ArtifactFingerprint {
        source_size,
        source_mtime_ns,
    }))
}

fn metadata_mtime_ns(meta: &std::fs::Metadata) -> i128 {
    (meta.mtime() as i128) * 1_000_000_000 + meta.mtime_nsec() as i128
}

fn normalize_tickers<I>(tickers: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for ticker in tickers {
        let value = ticker.trim().to_string();
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        result.push(value);
    }
    result
}

fn previous_jpx_business_day(today: Option<NaiveDate>) -> Result<NaiveDate, String> {
    let mut current = today.unwrap_or_else(today_jst) - Duration::days(1);
    loop {
        if is_jpx_business_day(current)? {
            return Ok(current);
        }
        current -= Duration::days(1);
    }
}

fn today_jst() -> NaiveDate {
    (Utc::now() + Duration::hours(9)).date_naive()
}

fn is_jpx_business_day(day: NaiveDate) -> Result<bool, String> {
    if day.weekday().number_from_monday() >= 6 {
        return Ok(false);
    }
    Ok(!jpx_market_holidays_for_year(day.year())?.contains(&day))
}

fn jpx_market_holidays_for_year(year: i32) -> Result<BTreeSet<NaiveDate>, String> {
    let data = config::load_toml("jpx_market_holidays.toml")?;
    let holidays = data
        .get("holidays")
        .and_then(|value| value.as_table())
        .ok_or_else(|| "jpx_market_holidays.toml missing [holidays]".to_string())?;
    let raw_dates = holidays
        .get(&year.to_string())
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("JPX market holidays are not configured for {year}"))?;
    let mut result = BTreeSet::new();
    for raw in raw_dates {
        let value = raw
            .as_str()
            .ok_or_else(|| format!("invalid JPX holiday entry for {year}: {raw}"))?;
        result.insert(
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|err| format!("invalid JPX holiday date {value}: {err}"))?,
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_business_day_skips_weekend_and_configured_holiday() {
        assert_eq!(
            previous_jpx_business_day(Some(NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        );
    }

    #[test]
    fn normalizes_tickers() {
        assert_eq!(
            normalize_tickers(vec![
                " 7203 ".to_string(),
                "".to_string(),
                "7203".to_string(),
                "130A".to_string(),
            ]),
            vec!["7203".to_string(), "130A".to_string()],
        );
    }

    #[test]
    fn screening_tickers_require_canonical_pl_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stocks.db");
        let conn = Connection::open(&path).unwrap();
        crate::schema::init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO stock_entities (ticker, name) VALUES ('1234', 'Example')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO data_sources (source_code) VALUES ('test_price'), ('legacy'), ('edinet_xbrl')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO price_observations (ticker, date, source_id, close, updated_at) VALUES ('1234', '2026-05-08', (SELECT source_id FROM data_sources WHERE source_code='test_price'), 100, '2026-05-08')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO financial_item_defs (statement, item_name) VALUES ('pl', 'net_income')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO financial_values (ticker, period, item_id, source_id, value, updated_at) VALUES ('1234', '2025-03', (SELECT item_id FROM financial_item_defs WHERE statement='pl' AND item_name='net_income'), (SELECT source_id FROM data_sources WHERE source_code='legacy'), 1, '2026-05-08')",
            [],
        )
        .unwrap();
        drop(conn);

        assert!(get_screening_tickers(Some(path.clone()), None).unwrap().is_empty());

        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO financial_values (ticker, period, item_id, source_id, value, updated_at) VALUES ('1234', '2025-03', (SELECT item_id FROM financial_item_defs WHERE statement='pl' AND item_name='net_income'), (SELECT source_id FROM data_sources WHERE source_code='edinet_xbrl'), 2, '2026-05-08')",
            [],
        )
        .unwrap();
        drop(conn);

        assert_eq!(
            get_screening_tickers(Some(path), None).unwrap(),
            vec!["1234".to_string()]
        );
    }
}
