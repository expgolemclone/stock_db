use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone, PartialEq)]
pub struct StockPriceOnDate {
    pub ticker: String,
    pub price_date: String,
    pub close: f64,
}

pub fn default_stocks_db_path() -> PathBuf {
    crate::screening::default_stocks_db_path()
}

pub fn load_default_stock_price_on_date(
    ticker: &str,
    price_date: &str,
) -> Result<Option<StockPriceOnDate>, String> {
    load_stock_price_on_date(&default_stocks_db_path(), ticker, price_date)
}

pub fn load_stock_price_on_date(
    db_path: &Path,
    ticker: &str,
    price_date: &str,
) -> Result<Option<StockPriceOnDate>, String> {
    crate::auto_update::ensure_prices_fresh_for_api(db_path)?;

    let conn = Connection::open(db_path).map_err(|err| err.to_string())?;
    get_stock_price_on_date(&conn, ticker, price_date)
}

fn get_stock_price_on_date(
    conn: &Connection,
    ticker: &str,
    price_date: &str,
) -> Result<Option<StockPriceOnDate>, String> {
    let close = conn
        .query_row(
            r#"
            SELECT close
            FROM prices
            WHERE ticker = ?1
              AND date = ?2
              AND close IS NOT NULL
            LIMIT 1
            "#,
            params![ticker, price_date],
            |row| row.get::<_, f64>(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    Ok(close.map(|close| StockPriceOnDate {
        ticker: ticker.to_string(),
        price_date: price_date.to_string(),
        close,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE prices (
                ticker TEXT NOT NULL,
                date TEXT NOT NULL,
                close REAL,
                volume INTEGER,
                updated_at TEXT,
                PRIMARY KEY (ticker, date)
            );
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn returns_exact_date_price() {
        let conn = fixture_conn();
        conn.execute(
            "INSERT INTO prices (ticker, date, close) VALUES (?1, ?2, ?3)",
            params!["9435", "2026-05-22", 37610.0_f64],
        )
        .unwrap();

        assert_eq!(
            get_stock_price_on_date(&conn, "9435", "2026-05-22").unwrap(),
            Some(StockPriceOnDate {
                ticker: "9435".to_string(),
                price_date: "2026-05-22".to_string(),
                close: 37610.0,
            })
        );
    }

    #[test]
    fn does_not_use_another_date() {
        let conn = fixture_conn();
        conn.execute(
            "INSERT INTO prices (ticker, date, close) VALUES (?1, ?2, ?3)",
            params!["9435", "2026-05-21", 37310.0_f64],
        )
        .unwrap();

        assert_eq!(
            get_stock_price_on_date(&conn, "9435", "2026-05-22").unwrap(),
            None
        );
    }

    #[test]
    fn ignores_null_close() {
        let conn = fixture_conn();
        conn.execute(
            "INSERT INTO prices (ticker, date, close) VALUES (?1, ?2, NULL)",
            params!["9435", "2026-05-22"],
        )
        .unwrap();

        assert_eq!(
            get_stock_price_on_date(&conn, "9435", "2026-05-22").unwrap(),
            None
        );
    }
}
