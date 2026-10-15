use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;

pub type ItemMap = HashMap<String, Option<f64>>;
pub type StatementMap = HashMap<String, ItemMap>;

#[derive(Debug, Clone, Serialize)]
pub struct HistoricalItems {
    pub period: String,
    pub items: ItemMap,
}

#[derive(Debug, Clone, Serialize)]
pub struct PotentialEquitySummary {
    pub has_potential_equity: Option<bool>,
    pub total_period_end_common_shares: Option<f64>,
    pub has_unquantified_terms: bool,
    pub instrument_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreeningStock {
    pub ticker: String,
    pub name: String,
    pub price: Option<f64>,
    pub price_date: Option<String>,
    pub shares_outstanding: Option<i64>,
    pub financials: StatementMap,
    pub cf_history: Vec<HistoricalItems>,
    pub pl_history: Vec<HistoricalItems>,
    pub dividend_history: Vec<HistoricalItems>,
    pub potential_equity_summary: PotentialEquitySummary,
    pub diluted_eps_common_share_increase: Option<f64>,
}

const SCREENING_READ_MODEL_CTES: &str = r#"
WITH
screening_stocks AS (
    SELECT
        se.ticker,
        se.name,
        ssc.shares_outstanding
    FROM stock_entities AS se
    LEFT JOIN stock_share_counts AS ssc
      ON ssc.ticker = se.ticker
),
screening_financial_items AS (
    SELECT
        ticker,
        period,
        statement,
        item_name,
        value,
        source
    FROM canonical_financial_items
),
screening_potential_equity_instruments AS (
    SELECT
        pet.ticker,
        pet.period,
        ds.source_code AS source,
        pet.instrument_key,
        pet.instrument_type,
        pet.period_end_common_shares
    FROM potential_equity_instruments_raw AS pet
    JOIN data_sources AS ds
      ON ds.source_id = pet.source_id
),
screening_potential_equity_status AS (
    SELECT
        pes.ticker,
        pes.period,
        ds.source_code AS source,
        pes.status_key,
        pes.has_potential_equity,
        pes.source_concept,
        sk.source_kind
    FROM potential_equity_status_raw AS pes
    JOIN data_sources AS ds
      ON ds.source_id = pes.source_id
    JOIN source_kinds AS sk
      ON sk.source_kind_id = pes.source_kind_id
),
screening_diluted_eps_adjustments AS (
    SELECT
        dea.ticker,
        dea.period,
        ds.source_code AS source,
        dea.adjustment_key,
        dea.weighted_average_common_shares
    FROM diluted_eps_adjustments_raw AS dea
    JOIN data_sources AS ds
      ON ds.source_id = dea.source_id
)
"#;

fn read_model_query(body: &str) -> String {
    format!("{SCREENING_READ_MODEL_CTES}\n{body}")
}

fn load_screening_stocks(
    db_path: &Path,
    tickers: Option<&[String]>,
    fcf_periods: usize,
    pl_periods: usize,
    payout_periods: usize,
) -> Result<Vec<ScreeningStock>, String> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("failed to open {}: {err}", db_path.display()))?;
    conn.busy_timeout(Duration::from_secs(60))
        .map_err(|err| format!("failed to configure {}: {err}", db_path.display()))?;
    let names = get_stock_names(&conn).map_err(|err| db_context_error(db_path, err))?;
    let selected_tickers = match tickers {
        Some(values) => values.to_vec(),
        None => get_all_tickers(&conn).map_err(|err| db_context_error(db_path, err))?,
    };

    selected_tickers
        .into_iter()
        .map(|ticker| {
            let name = names.get(&ticker).cloned().unwrap_or_default();
            build_screening_stock(&conn, ticker, name, fcf_periods, pl_periods, payout_periods)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| db_context_error(db_path, err))
}

pub fn load_screening_stocks_from_path(
    db_path: &Path,
    tickers: Option<&[String]>,
    fcf_periods: usize,
    pl_periods: usize,
    payout_periods: usize,
) -> Result<Vec<ScreeningStock>, String> {
    load_screening_stocks(db_path, tickers, fcf_periods, pl_periods, payout_periods)
}

pub fn load_default_screening_stocks(
    tickers: Option<&[String]>,
    fcf_periods: usize,
    pl_periods: usize,
    payout_periods: usize,
) -> Result<Vec<ScreeningStock>, String> {
    load_screening_stocks(
        &default_stocks_db_path(),
        tickers,
        fcf_periods,
        pl_periods,
        payout_periods,
    )
}

pub fn default_stocks_db_path() -> PathBuf {
    crate::paths::stocks_db_path()
}

fn db_context_error(db_path: &Path, err: String) -> String {
    format!(
        "failed to load screening data from {}: {err}",
        db_path.display()
    )
}

pub fn get_all_tickers(conn: &Connection) -> Result<Vec<String>, String> {
    let sql = read_model_query("SELECT ticker FROM screening_stocks ORDER BY ticker");
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

pub fn get_stock_names(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let sql = read_model_query("SELECT ticker, name FROM screening_stocks ORDER BY ticker");
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| err.to_string())
}

fn build_screening_stock(
    conn: &Connection,
    ticker: String,
    name: String,
    fcf_periods: usize,
    pl_periods: usize,
    payout_periods: usize,
) -> Result<ScreeningStock, String> {
    let financials = get_financial_dict(conn, &ticker)?;
    let (price, price_date, shares_outstanding) = get_latest_price_with_shares(conn, &ticker)?;
    let cf_history = get_historical_items(conn, &ticker, "cf", fcf_periods)?;
    let pl_history = get_historical_items(conn, &ticker, "pl", pl_periods)?;
    let dividend_history = get_historical_items(conn, &ticker, "dividend", payout_periods)?;
    let potential_equity_summary = get_potential_equity_summary(conn, &ticker)?;
    let diluted_eps_common_share_increase = get_diluted_eps_common_share_increase(conn, &ticker)?;

    Ok(ScreeningStock {
        ticker,
        name,
        price,
        price_date,
        shares_outstanding,
        financials,
        cf_history,
        pl_history,
        dividend_history,
        potential_equity_summary,
        diluted_eps_common_share_increase,
    })
}

fn get_financial_dict(conn: &Connection, ticker: &str) -> Result<StatementMap, String> {
    let latest_period_sql = read_model_query(
        r#"
        SELECT period
        FROM screening_financial_items
        WHERE ticker = ?1 AND statement = 'pl'
        ORDER BY period DESC
        LIMIT 1
        "#,
    );
    let period = conn
        .query_row(&latest_period_sql, params![ticker], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|err| err.to_string())?;

    let Some(period) = period else {
        return Ok(HashMap::new());
    };

    let mut result = StatementMap::new();
    let sql = read_model_query(
        r#"
        SELECT statement, item_name, value
        FROM screening_financial_items
        WHERE ticker = ?1 AND period = ?2
        "#,
    );
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, period], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
            ))
        })
        .map_err(|err| err.to_string())?;
    for row in rows {
        let (statement, item_name, value) = row.map_err(|err| err.to_string())?;
        result
            .entry(statement)
            .or_default()
            .insert(item_name, value);
    }

    merge_latest_statement_rows(conn, ticker, "forecast", None, &mut result)?;
    merge_latest_statement_rows(conn, ticker, "forecast", Some("shikiho"), &mut result)?;
    merge_latest_statement_rows(conn, ticker, "dividend", Some("shikiho"), &mut result)?;
    Ok(result)
}

fn merge_latest_statement_rows(
    conn: &Connection,
    ticker: &str,
    statement: &str,
    source: Option<&str>,
    result: &mut StatementMap,
) -> Result<(), String> {
    let body = if source.is_some() {
        r#"
        SELECT item_name, value
        FROM screening_financial_items
        WHERE ticker = ?1
          AND statement = ?2
          AND source = ?3
          AND period = (
              SELECT MAX(period)
              FROM screening_financial_items
              WHERE ticker = ?1 AND statement = ?2 AND source = ?3
          )
        "#
    } else {
        r#"
        SELECT item_name, value
        FROM screening_financial_items
        WHERE ticker = ?1
          AND statement = ?2
          AND period = (
              SELECT MAX(period)
              FROM screening_financial_items
              WHERE ticker = ?1 AND statement = ?2
          )
        "#
    };
    let sql = read_model_query(body);
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let mut rows = match source {
        Some(source_value) => stmt
            .query(params![ticker, statement, source_value])
            .map_err(|err| err.to_string())?,
        None => stmt
            .query(params![ticker, statement])
            .map_err(|err| err.to_string())?,
    };
    while let Some(row) = rows.next().map_err(|err| err.to_string())? {
        result.entry(statement.to_string()).or_default().insert(
            row.get::<_, String>(0).map_err(|err| err.to_string())?,
            row.get::<_, Option<f64>>(1)
                .map_err(|err| err.to_string())?,
        );
    }
    Ok(())
}

fn get_latest_price_with_shares(
    conn: &Connection,
    ticker: &str,
) -> Result<(Option<f64>, Option<String>, Option<i64>), String> {
    let price_row = conn
        .query_row(
            r#"
            SELECT close, date
            FROM prices
            WHERE ticker = ?1
            ORDER BY date DESC
            LIMIT 1
            "#,
            params![ticker],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    let (price, price_date) = match price_row {
        Some((price, date)) => (Some(price), Some(date)),
        None => (None, None),
    };
    let shares = conn
        .query_row(
            &read_model_query("SELECT shares_outstanding FROM screening_stocks WHERE ticker = ?1"),
            params![ticker],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten();
    Ok((price, price_date, shares))
}

fn get_latest_financial_period(conn: &Connection, ticker: &str) -> Result<Option<String>, String> {
    let sql = read_model_query(
        r#"
        SELECT MAX(period)
        FROM screening_financial_items
        WHERE ticker = ?1
          AND source = 'edinet_xbrl'
          AND statement IN ('bs', 'pl')
        "#,
    );
    conn.query_row(&sql, params![ticker], |row| row.get::<_, Option<String>>(0))
        .map_err(|err| err.to_string())
}

fn get_potential_equity_summary(
    conn: &Connection,
    ticker: &str,
) -> Result<PotentialEquitySummary, String> {
    let latest_period = match get_latest_financial_period(conn, ticker)? {
        Some(period) => period,
        None => return Ok(unknown_potential_equity_summary()),
    };
    let sql = read_model_query(
        r#"
        SELECT instrument_key, instrument_type, period_end_common_shares
        FROM screening_potential_equity_instruments
        WHERE ticker = ?1
          AND source = 'edinet_xbrl'
          AND period = ?2
        ORDER BY instrument_key, instrument_type
        "#,
    );
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, latest_period], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
            ))
        })
        .map_err(|err| err.to_string())?;

    let mut share_values_by_instrument: HashMap<String, HashSet<u64>> = HashMap::new();
    let mut instrument_types = HashSet::new();
    let mut has_rows = false;
    for row in rows {
        let (instrument_key, instrument_type, shares) = row.map_err(|err| err.to_string())?;
        has_rows = true;
        instrument_types.insert(instrument_type);
        let values = share_values_by_instrument.entry(instrument_key).or_default();
        if let Some(value) = shares {
            values.insert(value.to_bits());
        }
    }
    if !has_rows {
        return get_potential_equity_status_summary(conn, ticker, &latest_period);
    }

    let mut total = 0.0;
    let mut has_quantified = false;
    let mut has_unquantified_terms = false;
    for values in share_values_by_instrument.values() {
        if values.len() == 1 {
            total += f64::from_bits(*values.iter().next().expect("one value"));
            has_quantified = true;
        } else {
            has_unquantified_terms = true;
        }
    }
    let mut instrument_types = instrument_types.into_iter().collect::<Vec<_>>();
    instrument_types.sort();
    Ok(PotentialEquitySummary {
        has_potential_equity: Some(true),
        total_period_end_common_shares: (has_quantified && !has_unquantified_terms)
            .then_some(total),
        has_unquantified_terms,
        instrument_types,
    })
}

fn get_potential_equity_status_summary(
    conn: &Connection,
    ticker: &str,
    latest_period: &str,
) -> Result<PotentialEquitySummary, String> {
    let sql = read_model_query(
        r#"
        SELECT has_potential_equity
        FROM screening_potential_equity_status
        WHERE ticker = ?1 AND source = 'edinet_xbrl' AND period = ?2
        ORDER BY status_key, source_kind, source_concept
        "#,
    );
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, latest_period], |row| row.get::<_, i64>(0))
        .map_err(|err| err.to_string())?;
    let mut has_presence = false;
    let mut has_absence = false;
    for row in rows {
        if row.map_err(|err| err.to_string())? == 0 {
            has_absence = true;
        } else {
            has_presence = true;
        }
    }
    if has_presence {
        return Ok(PotentialEquitySummary {
            has_potential_equity: Some(true),
            total_period_end_common_shares: None,
            has_unquantified_terms: true,
            instrument_types: Vec::new(),
        });
    }
    if has_absence {
        return Ok(PotentialEquitySummary {
            has_potential_equity: Some(false),
            total_period_end_common_shares: Some(0.0),
            has_unquantified_terms: false,
            instrument_types: Vec::new(),
        });
    }
    Ok(unknown_potential_equity_summary())
}

fn get_diluted_eps_common_share_increase(
    conn: &Connection,
    ticker: &str,
) -> Result<Option<f64>, String> {
    let latest_period = match get_latest_financial_period(conn, ticker)? {
        Some(period) => period,
        None => return Ok(None),
    };
    let sql = read_model_query(
        r#"
        SELECT adjustment_key, weighted_average_common_shares
        FROM screening_diluted_eps_adjustments
        WHERE ticker = ?1 AND source = 'edinet_xbrl' AND period = ?2
        ORDER BY adjustment_key
        "#,
    );
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, latest_period], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f64>>(1)?,
            ))
        })
        .map_err(|err| err.to_string())?;
    let mut values_by_adjustment: HashMap<String, HashSet<u64>> = HashMap::new();
    for row in rows {
        let (adjustment_key, value) = row.map_err(|err| err.to_string())?;
        let values = values_by_adjustment.entry(adjustment_key).or_default();
        if let Some(value) = value {
            values.insert(value.to_bits());
        }
    }
    if values_by_adjustment.is_empty() {
        return Ok(None);
    }
    let mut total = 0.0;
    for values in values_by_adjustment.values() {
        if values.len() != 1 {
            return Ok(None);
        }
        total += f64::from_bits(*values.iter().next().expect("one value"));
    }
    Ok(Some(total))
}

fn unknown_potential_equity_summary() -> PotentialEquitySummary {
    PotentialEquitySummary {
        has_potential_equity: None,
        total_period_end_common_shares: None,
        has_unquantified_terms: false,
        instrument_types: Vec::new(),
    }
}

fn get_historical_items(
    conn: &Connection,
    ticker: &str,
    statement: &str,
    n_periods: usize,
) -> Result<Vec<HistoricalItems>, String> {
    let sql = read_model_query(
        r#"
        SELECT period, item_name, value
        FROM screening_financial_items
        WHERE ticker = ?1
          AND statement = ?2
          AND period IN (
              SELECT DISTINCT period
              FROM screening_financial_items
              WHERE ticker = ?1 AND statement = ?2
              ORDER BY period DESC
              LIMIT ?3
          )
        ORDER BY period DESC
        "#,
    );
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map(params![ticker, statement, n_periods as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
            ))
        })
        .map_err(|err| err.to_string())?;

    let mut grouped: Vec<HistoricalItems> = Vec::new();
    for row in rows {
        let (period, item_name, value) = row.map_err(|err| err.to_string())?;
        match grouped.last_mut() {
            Some(current) if current.period == period => {
                current.items.insert(item_name, value);
            }
            _ => {
                let mut items = ItemMap::new();
                items.insert(item_name, value);
                grouped.push(HistoricalItems { period, items });
            }
        }
    }
    Ok(grouped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::init_db(&conn).unwrap();
        conn
    }

    fn ensure_source(conn: &Connection, source: &str) -> i64 {
        conn.execute(
            "INSERT OR IGNORE INTO data_sources (source_code) VALUES (?1)",
            params![source],
        )
        .unwrap();
        conn.query_row(
            "SELECT source_id FROM data_sources WHERE source_code = ?1",
            params![source],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn ensure_source_kind(conn: &Connection, source_kind: &str) -> i64 {
        conn.execute(
            "INSERT OR IGNORE INTO source_kinds (source_kind) VALUES (?1)",
            params![source_kind],
        )
        .unwrap();
        conn.query_row(
            "SELECT source_kind_id FROM source_kinds WHERE source_kind = ?1",
            params![source_kind],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn insert_stock(conn: &Connection, ticker: &str) {
        conn.execute(
            "INSERT INTO stock_entities (ticker, name) VALUES (?1, ?1)",
            params![ticker],
        )
        .unwrap();
    }

    fn insert_financial_item(
        conn: &Connection,
        ticker: &str,
        period: &str,
        statement: &str,
        item_name: &str,
        value: f64,
        source: &str,
    ) {
        conn.execute(
            "INSERT OR IGNORE INTO financial_item_defs (statement, item_name) VALUES (?1, ?2)",
            params![statement, item_name],
        )
        .unwrap();
        let item_id: i64 = conn
            .query_row(
                "SELECT item_id FROM financial_item_defs WHERE statement = ?1 AND item_name = ?2",
                params![statement, item_name],
                |row| row.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO financial_values (ticker, period, item_id, source_id, value, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, '2026-01-01')",
            params![ticker, period, item_id, ensure_source(conn, source), value],
        )
        .unwrap();
    }

    #[test]
    fn query_errors_are_not_treated_as_missing_data() {
        let conn = fixture_conn();
        insert_stock(&conn, "1234");
        conn.execute_batch("DROP VIEW prices; CREATE TABLE prices (ticker TEXT, date TEXT, close TEXT);")
            .unwrap();
        conn.execute(
            "INSERT INTO prices (ticker, date, close) VALUES ('1234', '2026-05-18', 'broken')",
            [],
        )
        .unwrap();
        assert!(get_latest_price_with_shares(&conn, "1234").is_err());

        conn.execute_batch("DROP VIEW canonical_financial_items;")
            .unwrap();
        assert!(get_financial_dict(&conn, "1234").is_err());
    }

    #[test]
    fn potential_equity_counts_one_economic_instrument_once() {
        let conn = fixture_conn();
        insert_stock(&conn, "2489");
        insert_financial_item(&conn, "2489", "2025-12", "pl", "net_income", 1.0, "edinet_xbrl");
        insert_financial_item(&conn, "2489", "2025-12", "bs", "total_assets", 1.0, "edinet_xbrl");
        let source_id = ensure_source(&conn, "edinet_xbrl");
        for (concept, terms) in [("concept_a", "evidence A"), ("concept_b", "evidence B")] {
            conn.execute(
                r#"
                INSERT INTO potential_equity_instruments_raw
                    (ticker, period, source_id, instrument_key, instrument_type, instrument_name,
                     terms_text, period_end_common_shares, source_concept, source_kind_id, updated_at)
                VALUES (?1, '2025-12', ?2, 'option:11', 'stock_option', '第11回新株予約権',
                        ?3, 51900, ?4, ?5, '2026-01-01')
                "#,
                params![
                    "2489",
                    source_id,
                    terms,
                    concept,
                    ensure_source_kind(&conn, "share_options_text"),
                ],
            )
            .unwrap();
        }

        let summary = get_potential_equity_summary(&conn, "2489").unwrap();
        assert_eq!(summary.total_period_end_common_shares, Some(51_900.0));
        assert!(!summary.has_unquantified_terms);
    }

    #[test]
    fn conflicting_quantification_does_not_emit_partial_total() {
        let conn = fixture_conn();
        insert_stock(&conn, "2489");
        insert_financial_item(&conn, "2489", "2025-12", "pl", "net_income", 1.0, "edinet_xbrl");
        insert_financial_item(&conn, "2489", "2025-12", "bs", "total_assets", 1.0, "edinet_xbrl");
        let source_id = ensure_source(&conn, "edinet_xbrl");
        for (concept, shares) in [("concept_a", 100.0), ("concept_b", 200.0)] {
            conn.execute(
                r#"
                INSERT INTO potential_equity_instruments_raw
                    (ticker, period, source_id, instrument_key, instrument_type, instrument_name,
                     terms_text, period_end_common_shares, source_concept, source_kind_id, updated_at)
                VALUES (?1, '2025-12', ?2, 'option:11', 'stock_option', '第11回新株予約権',
                        ?3, ?4, ?5, ?6, '2026-01-01')
                "#,
                params![
                    "2489",
                    source_id,
                    concept,
                    shares,
                    concept,
                    ensure_source_kind(&conn, "share_options_text"),
                ],
            )
            .unwrap();
        }

        let summary = get_potential_equity_summary(&conn, "2489").unwrap();
        assert_eq!(summary.total_period_end_common_shares, None);
        assert!(summary.has_unquantified_terms);
    }
}
