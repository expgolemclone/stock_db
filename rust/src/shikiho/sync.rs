//! Sync forecasts and dividends from japan_company_handbook into stocks.db.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior, params};

use super::storage::upsert_financial_value;

const SOURCE: &str = "shikiho";

/// Default handbook path: sibling repo ../japan_company_handbook/data/stock_performance.db
pub fn default_handbook_db_path(project_root: &Path) -> PathBuf {
    project_root
        .parent()
        .unwrap_or(project_root)
        .join("japan_company_handbook")
        .join("data")
        .join("stock_performance.db")
}

fn open_handbook(path: &Path) -> Result<Connection, String> {
    Connection::open(path).map_err(|e| format!("open handbook {}: {e}", path.display()))
}

pub fn sync_forecasts(conn: &mut Connection, handbook_db: &Path) -> Result<(usize, usize), String> {
    if !handbook_db.exists() {
        return Err(format!("Shikiho DB not found: {}", handbook_db.display()));
    }
    let hb = open_handbook(handbook_db)?;
    let mut stmt = hb
        .prepare(
            "SELECT stock_code, period, net_income FROM stock_forecasts \
             WHERE forecast_type = 'shikiho' ORDER BY stock_code, period ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut grouped: HashMap<String, Vec<(String, Option<i64>)>> = HashMap::new();
    for row in rows {
        let (ticker, period, ni) = row.map_err(|e| e.to_string())?;
        grouped.entry(ticker).or_default().push((period, ni));
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("begin shikiho forecast replacement: {e}"))?;
    tx.execute(
        "DELETE FROM financial_items WHERE source = ?1 AND statement = 'forecast' \
         AND item_name IN ('net_income_current', 'net_income_next')",
        params![SOURCE],
    )
    .map_err(|e| format!("delete shikiho forecasts: {e}"))?;

    let mut ok = 0usize;
    let mut skipped = 0usize;
    let mut tickers: Vec<String> = grouped.keys().cloned().collect();
    tickers.sort();
    for ticker in tickers {
        let mut forecasts = grouped.remove(&ticker).unwrap();
        let len = forecasts.len();
        if len < 1 {
            skipped += 1;
            continue;
        }
        // keep last 2 (latest two periods)
        let start = len.saturating_sub(2);
        forecasts.drain(..start);
        if forecasts[0].1.is_none() {
            skipped += 1;
            continue;
        }
        let period = forecasts[0].0.clone();
        if let Some(ni) = forecasts[0].1 {
            upsert_financial_value(
                &tx,
                &ticker,
                &period,
                "forecast",
                "net_income_current",
                (ni as f64) * 1_000_000.0,
                SOURCE,
            )?;
        }
        if forecasts.len() >= 2 {
            if let Some(ni) = forecasts[1].1 {
                upsert_financial_value(
                    &tx,
                    &ticker,
                    &period,
                    "forecast",
                    "net_income_next",
                    (ni as f64) * 1_000_000.0,
                    SOURCE,
                )?;
            }
        }
        ok += 1;
    }
    tx.commit()
        .map_err(|e| format!("commit shikiho forecast replacement: {e}"))?;
    Ok((ok, skipped))
}

pub fn sync_dividends(
    conn: &mut Connection,
    handbook_db: &Path,
    overrides_path: &Path,
) -> Result<(usize, usize), String> {
    if !handbook_db.exists() {
        return Err(format!("Shikiho DB not found: {}", handbook_db.display()));
    }
    let overrides = load_dividend_overrides(overrides_path)?;
    let hb = open_handbook(handbook_db)?;
    let mut stmt = hb
        .prepare(
            "SELECT stock_code, period, dividend FROM stock_dividends \
             WHERE dividend IS NOT NULL ORDER BY stock_code, period ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut grouped: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for row in rows {
        let (ticker, period, div) = row.map_err(|e| e.to_string())?;
        grouped.entry(ticker).or_default().push((period, div));
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("begin shikiho dividend replacement: {e}"))?;
    tx.execute(
        "DELETE FROM financial_items WHERE source = ?1 AND statement = 'dividend'",
        params![SOURCE],
    )
    .map_err(|e| format!("delete shikiho dividends: {e}"))?;

    let mut ok = 0usize;
    let mut skipped = 0usize;
    let mut tickers: Vec<String> = grouped.keys().cloned().collect();
    tickers.sort();
    for ticker in tickers {
        let dividends = grouped.remove(&ticker).unwrap();
        if dividends.is_empty() {
            skipped += 1;
            continue;
        }
        for (period, div) in dividends {
            let value = overrides
                .get(&ticker)
                .and_then(|m| m.get(&period))
                .copied()
                .unwrap_or(div);
            upsert_financial_value(&tx, &ticker, &period, "dividend", "dps", value, SOURCE)?;
        }
        ok += 1;
    }
    tx.commit()
        .map_err(|e| format!("commit shikiho dividend replacement: {e}"))?;
    Ok((ok, skipped))
}

pub fn load_dividend_overrides(
    path: &Path,
) -> Result<HashMap<String, HashMap<String, f64>>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    let mut out = HashMap::new();
    let Some(table) = value.get("dividend_overrides").and_then(|v| v.as_table()) else {
        return Ok(out);
    };
    for (ticker, periods) in table {
        let Some(periods) = periods.as_table() else {
            continue;
        };
        let mut m = HashMap::new();
        for (period, val) in periods {
            if let Some(f) = val.as_float() {
                m.insert(period.clone(), f);
            }
        }
        out.insert(ticker.clone(), m);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "stock_db_{name}_{}_{}.db",
            std::process::id(),
            nonce
        ))
    }

    #[test]
    fn sync_commands_fail_when_handbook_db_is_missing() {
        let mut conn = Connection::open_in_memory().unwrap();
        let missing = Path::new("/nonexistent/stock_performance.db");

        assert!(sync_forecasts(&mut conn, missing).is_err());
        assert!(
            sync_dividends(
                &mut conn,
                missing,
                Path::new("/nonexistent/dividend_overrides.toml"),
            )
            .is_err()
        );
    }

    #[test]
    fn dividend_overrides_load_from_toml() {
        let dir = std::env::temp_dir().join("edinet_xbrl_override_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("overrides.toml");
        std::fs::write(&path, "[dividend_overrides.\"8046\"]\n\"26.3\" = 40.0\n").unwrap();
        let overrides = load_dividend_overrides(&path).unwrap();
        assert_eq!(overrides.get("8046").unwrap().get("26.3"), Some(&40.0));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dividend_overrides_missing_file_is_empty() {
        let overrides = load_dividend_overrides(Path::new("/nonexistent/overrides.toml")).unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn forecast_sync_rolls_back_delete_and_partial_inserts() {
        let handbook_path = temp_db_path("handbook");
        let handbook = Connection::open(&handbook_path).unwrap();
        handbook
            .execute_batch(
                r#"
                CREATE TABLE stock_forecasts (
                    stock_code TEXT NOT NULL,
                    period TEXT NOT NULL,
                    net_income INTEGER,
                    forecast_type TEXT NOT NULL
                );
                INSERT INTO stock_forecasts VALUES
                    ('1111', '2027-03', 100, 'shikiho'),
                    ('9999', '2027-03', 200, 'shikiho');
                "#,
            )
            .unwrap();
        drop(handbook);

        let mut target = Connection::open_in_memory().unwrap();
        crate::schema::init_db(&target).unwrap();
        target
            .execute(
                "INSERT INTO stock_entities (ticker, name) VALUES ('1111', 'Existing')",
                [],
            )
            .unwrap();
        upsert_financial_value(
            &target,
            "1111",
            "2026-03",
            "forecast",
            "net_income_current",
            50.0,
            SOURCE,
        )
        .unwrap();

        let error = sync_forecasts(&mut target, &handbook_path).unwrap_err();
        assert!(error.contains("FOREIGN KEY constraint failed"));
        let value: f64 = target
            .query_row(
                "SELECT value FROM financial_items WHERE ticker = '1111' AND source = 'shikiho'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, 50.0);

        std::fs::remove_file(handbook_path).ok();
    }
}
