//! DB helpers for shikiho financial_values upserts.

use rusqlite::{Connection, params};

pub(crate) fn ensure_data_source(conn: &Connection, source_code: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO data_sources (source_code) VALUES (?1)",
        params![source_code],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT source_id FROM data_sources WHERE source_code = ?1",
        params![source_code],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn ensure_financial_item(
    conn: &Connection,
    statement: &str,
    item_name: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO financial_item_defs (statement, item_name) VALUES (?1, ?2)",
        params![statement, item_name],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT item_id FROM financial_item_defs WHERE statement = ?1 AND item_name = ?2",
        params![statement, item_name],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn upsert_financial_value(
    conn: &Connection,
    ticker: &str,
    period: &str,
    statement: &str,
    item_name: &str,
    value: f64,
    source: &str,
) -> Result<(), String> {
    let source_id = ensure_data_source(conn, source)?;
    let item_id = ensure_financial_item(conn, statement, item_name)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO financial_values (ticker, period, item_id, source_id, value, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(ticker, period, item_id, source_id) DO UPDATE SET
            value=excluded.value, updated_at=excluded.updated_at",
        params![ticker, period, item_id, source_id, value, now],
    )
    .map_err(|e| format!("upsert financial_value: {e}"))?;
    Ok(())
}
