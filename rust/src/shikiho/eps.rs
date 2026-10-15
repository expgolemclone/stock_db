//! Compute forecast EPS (source=computed) from forecast net income / current shares.
//!
//! Historical EPS is never recomputed here. Basic and diluted historical EPS are
//! accounting-standard measures whose denominators use weighted-average ordinary
//! shares and whose numerators can require attribution/preference adjustments.
//! The canonical historical values therefore come from reported XBRL facts.

use std::collections::HashSet;

use rusqlite::{Connection, TransactionBehavior, params};

use super::storage::upsert_financial_value;

const SOURCE: &str = "computed";
const FORECAST_SOURCE: &str = "shikiho";

const FORECAST_SQL: &str = "\
SELECT fi.ticker, fi.period, fi.item_name, fi.value AS net_income, \
       s.shares_outstanding AS shares \
FROM canonical_financial_items fi \
JOIN stocks s ON fi.ticker = s.ticker \
WHERE fi.statement = 'forecast' \
  AND fi.item_name IN ('net_income_current', 'net_income_next') \
  AND fi.source = ?1 \
  AND fi.value IS NOT NULL \
  AND s.shares_outstanding IS NOT NULL \
  AND s.shares_outstanding > 0";

pub fn compute_eps(conn: &mut Connection) -> Result<(usize, usize), String> {
    let mut db_rows: Vec<(String, String, String, String, f64)> = Vec::new();
    let mut tickers: HashSet<String> = HashSet::new();

    {
        let mut stmt = conn.prepare(FORECAST_SQL).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([FORECAST_SOURCE], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, f64>(3)?,
                    r.get::<_, f64>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (ticker, period, item_name, ni, shares) = row.map_err(|e| e.to_string())?;
            let eps_name = if item_name == "net_income_current" {
                "eps_current"
            } else {
                "eps_next"
            };
            db_rows.push((
                ticker.clone(),
                period,
                "forecast".into(),
                eps_name.into(),
                ni / shares,
            ));
            tickers.insert(ticker);
        }
    }

    let count = db_rows.len();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("begin computed eps replacement: {e}"))?;
    tx.execute(
        "DELETE FROM financial_items WHERE source = ?1 \
         AND item_name IN ('eps', 'eps_current', 'eps_next')",
        params![SOURCE],
    )
    .map_err(|e| format!("delete computed eps: {e}"))?;

    for (ticker, period, statement, item_name, value) in db_rows {
        upsert_financial_value(
            &tx, &ticker, &period, &statement, &item_name, value, SOURCE,
        )?;
    }
    tx.commit()
        .map_err(|e| format!("commit computed eps replacement: {e}"))?;
    Ok((tickers.len(), count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    fn initialized_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::init_db(&conn).unwrap();
        conn
    }

    fn insert_stock(conn: &Connection, ticker: &str, shares: i64) {
        conn.execute(
            "INSERT INTO stock_entities (ticker, name) VALUES (?1, ?1)",
            [ticker],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_share_counts \
             (ticker, shares_outstanding, shares_updated_at, updated_at) \
             VALUES (?1, ?2, 'now', 'now')",
            params![ticker, shares],
        )
        .unwrap();
    }

    fn computed_value(
        conn: &Connection,
        ticker: &str,
        statement: &str,
        item_name: &str,
    ) -> Option<f64> {
        conn.query_row(
            "SELECT value FROM financial_items \
             WHERE ticker = ?1 AND statement = ?2 AND item_name = ?3 AND source = ?4",
            params![ticker, statement, item_name, SOURCE],
            |row| row.get(0),
        )
        .ok()
    }

    #[test]
    fn computes_forecast_eps_but_never_historical_eps() {
        let mut conn = initialized_db();
        insert_stock(&conn, "1234", 20);
        upsert_financial_value(
            &conn, "1234", "2025-03", "pl", "net_income", 100.0, "edinet_xbrl",
        )
        .unwrap();
        upsert_financial_value(
            &conn, "1234", "2025-03", "bs", "shares_outstanding", 10.0, "edinet_xbrl",
        )
        .unwrap();
        upsert_financial_value(
            &conn, "1234", "2026-03", "forecast", "net_income_current", 200.0,
            FORECAST_SOURCE,
        )
        .unwrap();
        upsert_financial_value(
            &conn, "1234", "2026-03", "forecast", "net_income_current", 999.0, "other",
        )
        .unwrap();

        assert_eq!(compute_eps(&mut conn).unwrap(), (1, 1));
        assert_eq!(computed_value(&conn, "1234", "pl", "eps"), None);
        assert_eq!(
            computed_value(&conn, "1234", "forecast", "eps_current"),
            Some(10.0)
        );
    }

    #[test]
    fn clears_stale_computed_eps_when_inputs_disappear() {
        let mut conn = initialized_db();
        insert_stock(&conn, "1234", 20);
        upsert_financial_value(
            &conn, "1234", "2025-03", "pl", "eps", 12.0, SOURCE,
        )
        .unwrap();

        assert_eq!(compute_eps(&mut conn).unwrap(), (0, 0));
        assert_eq!(computed_value(&conn, "1234", "pl", "eps"), None);
    }

    #[test]
    fn rolls_back_replacement_when_a_forecast_insert_fails() {
        let mut conn = initialized_db();
        insert_stock(&conn, "1234", 20);
        upsert_financial_value(
            &conn, "1234", "2024-03", "pl", "eps", 12.0, SOURCE,
        )
        .unwrap();
        upsert_financial_value(
            &conn, "1234", "2026-03", "forecast", "net_income_current", 200.0,
            FORECAST_SOURCE,
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER reject_computed_eps BEFORE INSERT ON financial_values \
             WHEN NEW.source_id = (SELECT source_id FROM data_sources WHERE source_code = 'computed') \
             BEGIN SELECT RAISE(ABORT, 'reject computed eps'); END;",
        )
        .unwrap();

        assert!(compute_eps(&mut conn).is_err());
        assert_eq!(computed_value(&conn, "1234", "pl", "eps"), Some(12.0));
    }
}
