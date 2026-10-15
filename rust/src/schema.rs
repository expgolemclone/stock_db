//! Versioned, transactional SQLite schema initialization.
//!
//! The SQL resources are shared with the Python price stack. Existing
//! unversioned databases are intentionally rejected instead of being mutated
//! through an implicit compatibility migration.

use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 4;
const BASE_SCHEMA_SQL: &str =
    include_str!("../../src/stock_db/storage/sql/0001_base.sql");
const VIEWS_SCHEMA_SQL: &str =
    include_str!("../../src/stock_db/storage/sql/0001_views.sql");
const CONTEXT_PERIOD_VIEWS_SQL: &str =
    include_str!("../../src/stock_db/storage/sql/0002_context_period_views.sql");

pub fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("enable foreign keys: {e}"))?;

    let version = schema_version(conn)?;
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    if version != 0 {
        return Err(format!(
            "unsupported stocks DB schema version {version}; expected {SCHEMA_VERSION}; rebuild the DB"
        ));
    }
    if application_object_count(conn)? != 0 {
        return Err(
            "unversioned stocks DB detected; rebuild it explicitly before using this binary"
                .to_string(),
        );
    }

    if let Err(error) = create_schema(conn) {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(error);
    }
    Ok(())
}

fn schema_version(conn: &Connection) -> Result<i64, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("read schema version: {e}"))
}

fn application_object_count(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("inspect existing schema: {e}"))
}

fn create_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| format!("begin schema transaction: {e}"))?;
    conn.execute_batch(BASE_SCHEMA_SQL)
        .map_err(|e| format!("0001_base.sql: {e}"))?;
    conn.execute_batch(VIEWS_SCHEMA_SQL)
        .map_err(|e| format!("0001_views.sql: {e}"))?;
    conn.execute_batch(CONTEXT_PERIOD_VIEWS_SQL)
        .map_err(|e| format!("0002_context_period_views.sql: {e}"))?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|e| format!("write schema version: {e}"))?;

    let has_foreign_key_violation = conn
        .prepare("PRAGMA foreign_key_check")
        .and_then(|mut statement| statement.exists([]))
        .map_err(|e| format!("foreign key check: {e}"))?;
    if has_foreign_key_violation {
        return Err("new schema failed foreign key validation".to_string());
    }

    conn.execute_batch("COMMIT;")
        .map_err(|e| format!("commit schema transaction: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_db_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        init_db(&conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn init_db_creates_canonical_views() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        for view in [
            "prices",
            "stocks",
            "financial_items",
            "financial_statement_contexts",
            "canonical_financial_items",
            "sec_reports",
            "preferred_share_terms",
            "potential_equity_instruments",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='view' AND name=?1",
                    rusqlite::params![view],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "view {view} missing");
        }
    }

    #[test]
    fn init_db_creates_price_observations_table() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='price_observations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn init_db_rejects_unversioned_existing_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE legacy (id INTEGER PRIMARY KEY);")
            .unwrap();
        let error = init_db(&conn).unwrap_err();
        assert!(error.contains("unversioned stocks DB"));
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
    }
}
