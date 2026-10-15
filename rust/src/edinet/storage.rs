//! SQLite helpers for the EDINET ingestion flow (sec_reports, ticker lookup).

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, params};

use super::resolver::is_jpx_security_code;

/// Open stocks.db with the same pragmas as the Python connection layer.
pub fn open_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir db parent: {e}"))?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("open {}: {e}", db_path.display()))?;
    conn.busy_timeout(Duration::from_secs(60))
        .map_err(|e| format!("busy_timeout: {e}"))?;
    crate::schema::init_db(&conn)?;
    Ok(conn)
}

fn ensure_data_source(conn: &Connection, source_code: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO data_sources (source_code) VALUES (?1)",
        params![source_code],
    )
    .map_err(|e| format!("ensure data_sources: {e}"))?;
    conn.query_row(
        "SELECT source_id FROM data_sources WHERE source_code = ?1",
        params![source_code],
        |row| row.get(0),
    )
    .map_err(|e| format!("lookup data_sources: {e}"))
}

fn ensure_doc_type(conn: &Connection, doc_type: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO doc_types (doc_type) VALUES (?1)",
        params![doc_type],
    )
    .map_err(|e| format!("ensure doc_types: {e}"))?;
    conn.query_row(
        "SELECT doc_type_id FROM doc_types WHERE doc_type = ?1",
        params![doc_type],
        |row| row.get(0),
    )
    .map_err(|e| format!("lookup doc_types: {e}"))
}

/// Upsert a sec_report_records row (ticker, fiscal_year, doc_id, xbrl_path).
pub fn upsert_sec_report(
    conn: &Connection,
    ticker: &str,
    fiscal_year: &str,
    doc_id: &str,
    xbrl_path: Option<&str>,
) -> Result<(), String> {
    let source_id = ensure_data_source(conn, "edinet")?;
    let doc_type_id = ensure_doc_type(conn, "annual_report")?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO sec_report_records
            (ticker, fiscal_year, doc_id, doc_type_id, xbrl_path, source_id, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(ticker, doc_id) DO UPDATE SET
            fiscal_year = excluded.fiscal_year,
            doc_type_id = excluded.doc_type_id,
            xbrl_path   = excluded.xbrl_path,
            source_id   = excluded.source_id,
            updated_at  = excluded.updated_at",
        params![
            ticker,
            fiscal_year,
            doc_id,
            doc_type_id,
            xbrl_path,
            source_id,
            now
        ],
    )
    .map_err(|e| format!("upsert sec_report_records: {e}"))?;
    Ok(())
}

/// All stock tickers that can participate in EDINET discovery.
///
/// Valid JPX security codes are included directly. A row with an explicit
/// EDINET code is also included so resolver fallback remains available when a
/// filing does not expose a decodable secCode.
pub fn get_edinet_target_tickers(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT ticker, edinet_code FROM stocks")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = HashSet::new();
    for row in rows {
        let (ticker, edinet_code) = row.map_err(|e| e.to_string())?;
        let has_edinet_code = edinet_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        if is_jpx_security_code(&ticker) || has_edinet_code {
            out.insert(ticker);
        }
    }
    Ok(out)
}

/// doc_ids already recorded in sec_report_records.
pub fn get_existing_doc_ids(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM sec_report_records")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = HashSet::new();
    for row in rows {
        out.insert(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Read EDINET_API_KEY from env first, then repo root .env.
pub fn get_edinet_api_key(project_root: &Path) -> Result<String, String> {
    if let Ok(val) = std::env::var("EDINET_API_KEY") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let env_path = project_root.join(".env");
    if env_path.is_file() {
        if let Ok(text) = std::fs::read_to_string(&env_path) {
            for raw in text.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let line = line.strip_prefix("export ").unwrap_or(line);
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                if key.trim() != "EDINET_API_KEY" {
                    continue;
                }
                let mut v = value.trim().to_string();
                if v.len() >= 2
                    && (v.starts_with('"') || v.starts_with('\''))
                    && v.ends_with(v.chars().next().unwrap())
                {
                    v = v[1..v.len() - 1].to_string();
                }
                if !v.is_empty() {
                    return Ok(v);
                }
            }
        }
    }
    Err("EDINET_API_KEY is not set (checked env and repo root .env)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE data_sources (source_id INTEGER PRIMARY KEY, source_code TEXT NOT NULL UNIQUE);
             CREATE TABLE doc_types (doc_type_id INTEGER PRIMARY KEY, doc_type TEXT NOT NULL UNIQUE);
             CREATE TABLE sec_report_records (
                 ticker TEXT NOT NULL, fiscal_year TEXT NOT NULL, doc_id TEXT NOT NULL,
                 doc_type_id INTEGER NOT NULL, xbrl_path TEXT, source_id INTEGER NOT NULL,
                 updated_at TEXT NOT NULL, PRIMARY KEY (ticker, doc_id),
                 FOREIGN KEY (doc_type_id) REFERENCES doc_types (doc_type_id),
                 FOREIGN KEY (source_id) REFERENCES data_sources (source_id));",
        )
        .unwrap();
        conn
    }

    #[test]
    fn upsert_sec_report_inserts_then_updates() {
        let conn = fixture_conn();
        upsert_sec_report(&conn, "7203", "FY2024", "S1000001", Some("/tmp/x")).unwrap();
        upsert_sec_report(&conn, "7203", "FY2025", "S1000001", Some("/tmp/x2")).unwrap();
        let (fy, path): (String, String) = conn
            .query_row(
                "SELECT fiscal_year, xbrl_path FROM sec_report_records WHERE ticker=?1",
                params!["7203"],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(fy, "FY2025");
        assert_eq!(path, "/tmp/x2");
    }

    #[test]
    fn edinet_targets_include_alphanumeric_jpx_codes_and_edinet_mappings() {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::init_db(&conn).unwrap();
        for (ticker, edinet_code) in [
            ("7203", None),
            ("130A", None),
            ("9A76", Some("E90001")),
            ("ABCD", Some("E90002")),
            ("WXYZ", None),
        ] {
            conn.execute(
                "INSERT INTO stock_entities (ticker, edinet_code) VALUES (?1, ?2)",
                params![ticker, edinet_code],
            )
            .unwrap();
        }

        let targets = get_edinet_target_tickers(&conn).unwrap();
        assert!(targets.contains("7203"));
        assert!(targets.contains("130A"));
        assert!(targets.contains("9A76"));
        assert!(targets.contains("ABCD"));
        assert!(!targets.contains("WXYZ"));
    }
}
