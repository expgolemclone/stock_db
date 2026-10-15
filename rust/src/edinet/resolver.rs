//! EDINET secCode / edinetCode -> ticker resolution.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

const JPX_CODE_LETTERS: &str = "ACDFGHJKLMNPRSTUWXY";

fn is_jpx_code_char(value: char) -> bool {
    value.is_ascii_digit() || JPX_CODE_LETTERS.contains(value)
}

pub(crate) fn is_jpx_security_code(code: &str) -> bool {
    let chars = code.chars().collect::<Vec<_>>();
    chars.len() == 4
        && chars[0].is_ascii_digit()
        && is_jpx_code_char(chars[1])
        && chars[2].is_ascii_digit()
        && is_jpx_code_char(chars[3])
}

/// Convert an EDINET secCode to the corresponding 4-character JPX security code.
///
/// EDINET API v2 appends a check digit to the 4-character security code.
pub fn sec_code_to_ticker(sec_code: Option<&str>) -> Option<String> {
    let code = sec_code?.trim();
    let ticker = code.chars().take(4).collect::<String>();
    if is_jpx_security_code(&ticker) {
        Some(ticker)
    } else {
        None
    }
}

/// Resolver built from the current DB state (stock_entities edinet_code/name).
pub struct EdinetResolver {
    edinet_to_ticker: HashMap<String, String>,
    ticker_to_name: HashMap<String, String>,
}

impl EdinetResolver {
    pub fn from_db(conn: &Connection) -> Result<Self, String> {
        let mut edinet_to_ticker = HashMap::new();
        let mut ticker_to_name = HashMap::new();
        let mut stmt = conn
            .prepare("SELECT ticker, edinet_code, name FROM stocks")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (ticker, edinet_code, name) = row.map_err(|e| e.to_string())?;
            if let Some(name) = name {
                let n = name.trim();
                if !n.is_empty() {
                    ticker_to_name.insert(ticker.clone(), n.to_string());
                }
            }
            if let Some(code) = edinet_code {
                let code = code.trim();
                if !code.is_empty() {
                    edinet_to_ticker.insert(code.to_string(), ticker);
                }
            }
        }
        Ok(Self {
            edinet_to_ticker,
            ticker_to_name,
        })
    }

    pub fn edinet_to_ticker(&self, code: &str) -> Option<&str> {
        self.edinet_to_ticker.get(code).map(|s| s.as_str())
    }

    /// Resolve a ticker from an API result row. secCode is authoritative when
    /// it can be decoded. A secCode/edinetCode mismatch rejects the row.
    pub fn resolve_api_result(
        &self,
        sec_code: Option<&str>,
        edinet_code: Option<&str>,
        target_tickers: &HashSet<String>,
    ) -> Option<String> {
        let ticker = sec_code_to_ticker(sec_code);
        let mapped = edinet_code.and_then(|c| self.edinet_to_ticker(c).map(str::to_string));
        if let Some(t) = &ticker {
            if let Some(m) = &mapped {
                if m != t {
                    return None;
                }
            }
            if !target_tickers.contains(t) {
                return None;
            }
            return Some(t.clone());
        }
        if let Some(m) = mapped {
            if target_tickers.contains(&m) {
                return Some(m);
            }
        }
        None
    }

    /// Diagnostic: warn if filerName does not look like the DB company name.
    pub fn check_filer_name(&self, ticker: &str, filer_name: Option<&str>) {
        let Some(filer) = filer_name.map(str::trim).filter(|s| !s.is_empty()) else {
            return;
        };
        let Some(expected) = self.ticker_to_name.get(ticker) else {
            return;
        };
        if expected == filer || expected.contains(filer) || filer.contains(expected.as_str()) {
            return;
        }
        eprintln!(
            "filerName mismatch for ticker {}: DB={}, filing={}",
            ticker, expected, filer
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sec_code_to_ticker_accepts_numeric_and_alphanumeric_codes() {
        for (sec_code, ticker) in [
            ("72030", "7203"),
            ("130A0", "130A"),
            ("9A760", "9A76"),
            ("9A7A0", "9A7A"),
        ] {
            assert_eq!(sec_code_to_ticker(Some(sec_code)), Some(ticker.to_string()));
        }
        assert_eq!(sec_code_to_ticker(Some("7203 ")), Some("7203".to_string()));
    }

    #[test]
    fn sec_code_rejects_invalid_security_codes() {
        assert_eq!(sec_code_to_ticker(Some("720")), None);
        assert_eq!(sec_code_to_ticker(Some("ABCD0")), None);
        assert_eq!(sec_code_to_ticker(Some("130B0")), None);
        assert_eq!(sec_code_to_ticker(None), None);
    }

    fn targets<I: IntoIterator<Item = &'static str>>(items: I) -> HashSet<String> {
        items.into_iter().map(String::from).collect()
    }

    #[test]
    fn resolve_prefers_sec_code() {
        let mut edinet = HashMap::new();
        edinet.insert("E01234".to_string(), "7203".to_string());
        let r = EdinetResolver {
            edinet_to_ticker: edinet,
            ticker_to_name: HashMap::new(),
        };
        let t = targets(["7203", "9435"]);
        assert_eq!(
            r.resolve_api_result(Some("72030"), Some("E01234"), &t),
            Some("7203".to_string())
        );
    }

    #[test]
    fn resolve_rejects_mismatch() {
        let mut edinet = HashMap::new();
        edinet.insert("E01234".to_string(), "9435".to_string());
        let r = EdinetResolver {
            edinet_to_ticker: edinet,
            ticker_to_name: HashMap::new(),
        };
        let t = targets(["7203", "9435"]);
        assert_eq!(
            r.resolve_api_result(Some("72030"), Some("E01234"), &t),
            None
        );
    }

    #[test]
    fn resolve_falls_back_to_edinet_code() {
        let mut edinet = HashMap::new();
        edinet.insert("E01234".to_string(), "9A76".to_string());
        let r = EdinetResolver {
            edinet_to_ticker: edinet,
            ticker_to_name: HashMap::new(),
        };
        let t = targets(["7203", "9A76"]);
        assert_eq!(
            r.resolve_api_result(None, Some("E01234"), &t),
            Some("9A76".to_string())
        );
    }

    #[test]
    fn resolve_rejects_out_of_target() {
        let r = EdinetResolver {
            edinet_to_ticker: HashMap::new(),
            ticker_to_name: HashMap::new(),
        };
        let t = targets(["7203"]);
        assert_eq!(r.resolve_api_result(Some("94350"), None, &t), None);
    }
}
