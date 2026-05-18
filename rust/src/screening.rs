use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, params};

pub type ItemMap = HashMap<String, Option<f64>>;
pub type StatementMap = HashMap<String, ItemMap>;

#[derive(Debug, Clone)]
pub struct HistoricalItems {
    pub period: String,
    pub items: ItemMap,
}

#[derive(Debug, Clone)]
pub struct ScreeningStock {
    pub ticker: String,
    pub name: String,
    pub price: Option<f64>,
    pub price_date: Option<String>,
    pub shares_outstanding: Option<i64>,
    pub financials: StatementMap,
    pub cf_history: Vec<HistoricalItems>,
    pub pl_history: Vec<HistoricalItems>,
}

pub fn load_screening_stocks(
    db_path: &Path,
    tickers: Option<&[String]>,
    fcf_periods: usize,
    pl_periods: usize,
) -> Result<Vec<ScreeningStock>, String> {
    crate::auto_update::ensure_prices_fresh_for_api(db_path)?;

    let conn = Connection::open(db_path).map_err(|err| err.to_string())?;
    let names = get_stock_names(&conn)?;
    let selected_tickers = match tickers {
        Some(values) => values.to_vec(),
        None => get_all_tickers(&conn)?,
    };

    selected_tickers
        .into_iter()
        .map(|ticker| {
            let name = names.get(&ticker).cloned().unwrap_or_default();
            build_screening_stock(&conn, ticker, name, fcf_periods, pl_periods)
        })
        .collect()
}

pub fn get_all_tickers(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT ticker FROM stocks ORDER BY ticker")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

pub fn get_stock_names(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT ticker, name FROM stocks ORDER BY ticker")
        .map_err(|err| err.to_string())?;
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
) -> Result<ScreeningStock, String> {
    let financials = get_financial_dict(conn, &ticker)?;
    let (price, price_date, shares_outstanding) = get_latest_price_with_shares(conn, &ticker)?;
    let cf_history = get_historical_items(conn, &ticker, "cf", fcf_periods)?;
    let pl_history = get_historical_items(conn, &ticker, "pl", pl_periods)?;

    Ok(ScreeningStock {
        ticker,
        name,
        price,
        price_date,
        shares_outstanding,
        financials,
        cf_history,
        pl_history,
    })
}

fn get_financial_dict(conn: &Connection, ticker: &str) -> Result<StatementMap, String> {
    let period: Option<String> = conn
        .query_row(
            r#"
            SELECT period
            FROM financial_items
            WHERE ticker = ?1 AND statement = 'pl'
            ORDER BY period DESC
            LIMIT 1
            "#,
            params![ticker],
            |row| row.get(0),
        )
        .ok();

    let Some(period) = period else {
        return Ok(HashMap::new());
    };

    let mut result = StatementMap::new();
    {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT statement, item_name, value
                FROM financial_items
                WHERE ticker = ?1 AND period = ?2
                "#,
            )
            .map_err(|err| err.to_string())?;
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
    let sql = match source {
        Some(_) => {
            r#"
            SELECT item_name, value
            FROM financial_items
            WHERE ticker = ?1
              AND statement = ?2
              AND source = ?3
              AND period = (
                  SELECT MAX(period)
                  FROM financial_items
                  WHERE ticker = ?1 AND statement = ?2 AND source = ?3
              )
            "#
        }
        None => {
            r#"
            SELECT item_name, value
            FROM financial_items
            WHERE ticker = ?1
              AND statement = ?2
              AND period = (
                  SELECT MAX(period)
                  FROM financial_items
                  WHERE ticker = ?1 AND statement = ?2
              )
            "#
        }
    };

    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
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
            |row| {
                Ok((
                    row.get::<_, Option<f64>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .ok();
    let (price, price_date) = price_row.unwrap_or((None, None));
    let shares = conn
        .query_row(
            "SELECT shares_outstanding FROM stocks WHERE ticker = ?1",
            params![ticker],
            |row| row.get::<_, Option<i64>>(0),
        )
        .ok()
        .flatten();
    Ok((price, price_date, shares))
}

fn get_historical_items(
    conn: &Connection,
    ticker: &str,
    statement: &str,
    n_periods: usize,
) -> Result<Vec<HistoricalItems>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT period, item_name, value
            FROM financial_items
            WHERE ticker = ?1
              AND statement = ?2
              AND period IN (
                  SELECT DISTINCT period
                  FROM financial_items
                  WHERE ticker = ?1 AND statement = ?2
                  ORDER BY period DESC
                  LIMIT ?3
              )
            ORDER BY period DESC
            "#,
        )
        .map_err(|err| err.to_string())?;
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
        conn.execute_batch(
            r#"
            CREATE TABLE stocks (
                ticker TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                shares_outstanding INTEGER
            );
            CREATE TABLE prices (
                ticker TEXT NOT NULL,
                date TEXT NOT NULL,
                close REAL
            );
            CREATE TABLE financial_items (
                ticker TEXT NOT NULL,
                period TEXT NOT NULL,
                statement TEXT NOT NULL,
                item_name TEXT NOT NULL,
                value REAL,
                source TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn build_screening_stock_merges_source_specific_rows_and_history() {
        let conn = fixture_conn();
        conn.execute(
            "INSERT INTO stocks (ticker, name, shares_outstanding) VALUES (?1, ?2, ?3)",
            params!["1234", "Example", 1_000_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prices (ticker, date, close) VALUES (?1, ?2, ?3)",
            params!["1234", "2026-05-18", 321.0_f64],
        )
        .unwrap();
        for (period, statement, item_name, value, source) in [
            ("2025-03", "pl", "net_income", 100.0, "edinet_xbrl"),
            ("2025-03", "bs", "total_assets", 900.0, "edinet_xbrl"),
            ("2024-03", "cf", "operating_cf", 10.0, "edinet_xbrl"),
            ("2025-03", "cf", "operating_cf", 20.0, "edinet_xbrl"),
            ("2024-03", "pl", "net_income", 80.0, "edinet_xbrl"),
            ("2026-03", "forecast", "net_income", 111.0, "edinet_xbrl"),
            ("2027-03", "forecast", "net_income", 222.0, "edinet_xbrl"),
            ("2026-03", "forecast", "net_income", 333.0, "shikiho"),
            ("2027-03", "forecast", "net_income", 444.0, "shikiho"),
            ("2026-03", "dividend", "dividend_per_share", 10.0, "shikiho"),
            ("2027-03", "dividend", "dividend_per_share", 12.0, "shikiho"),
        ] {
            conn.execute(
                r#"
                INSERT INTO financial_items
                    (ticker, period, statement, item_name, value, source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params!["1234", period, statement, item_name, value, source],
            )
            .unwrap();
        }

        let stock =
            build_screening_stock(&conn, "1234".to_string(), "Example".to_string(), 2, 2).unwrap();

        assert_eq!(stock.price, Some(321.0));
        assert_eq!(stock.shares_outstanding, Some(1_000));
        assert_eq!(stock.financials["pl"].get("net_income"), Some(&Some(100.0)));
        assert_eq!(
            stock.financials["bs"].get("total_assets"),
            Some(&Some(900.0))
        );
        assert_eq!(
            stock.financials["forecast"].get("net_income"),
            Some(&Some(444.0))
        );
        assert_eq!(
            stock.financials["dividend"].get("dividend_per_share"),
            Some(&Some(12.0))
        );
        assert_eq!(
            stock
                .cf_history
                .iter()
                .map(|row| row.period.as_str())
                .collect::<Vec<_>>(),
            vec!["2025-03", "2024-03"]
        );
        assert_eq!(
            stock
                .pl_history
                .iter()
                .map(|row| row.period.as_str())
                .collect::<Vec<_>>(),
            vec!["2025-03", "2024-03"]
        );
    }
}
