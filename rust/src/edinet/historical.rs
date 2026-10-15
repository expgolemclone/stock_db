//! Discover historical annual reports via the EDINET API v2 documents.json listing.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use chrono::NaiveDate;

use super::client::{DocumentResult, Throttle, fetch_document_list};
use super::resolver::EdinetResolver;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredReport {
    pub ticker: String,
    pub doc_id: String,
    pub fiscal_year: String,
    pub period_end: Option<String>,
    pub submit_date: Option<String>,
    pub filer_name: Option<String>,
}

#[derive(Debug, Clone)]
struct AnnualCandidate {
    ticker: String,
    document: DocumentResult,
}

/// Scan `[from_date, to_date]` day by day and collect effective annual-report
/// docIDs whose resolved ticker is in `target_tickers`. Corrected reports are
/// resolved after the whole scan so a later correction can supersede an
/// original filing discovered on an earlier date. Serial with a per-request
/// interval (RULES: 2s delay). `skip_dates` are skipped (already scanned).
pub fn discover_historical_reports(
    client: &reqwest::blocking::Client,
    resolver: &EdinetResolver,
    api_key: &str,
    from_date: &str,
    to_date: &str,
    target_tickers: &HashSet<String>,
    interval_seconds: f64,
    timeout: Duration,
    skip_dates: &HashSet<String>,
) -> Result<BTreeMap<String, Vec<DiscoveredReport>>, String> {
    let start = NaiveDate::parse_from_str(from_date, "%Y-%m-%d")
        .map_err(|e| format!("from_date {from_date}: {e}"))?;
    let end = NaiveDate::parse_from_str(to_date, "%Y-%m-%d")
        .map_err(|e| format!("to_date {to_date}: {e}"))?;

    let mut throttle = Throttle::new(interval_seconds);
    let mut candidates: Vec<AnnualCandidate> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut current = start;

    while current <= end {
        let date_str = current.to_string();
        current = current
            .succ_opt()
            .unwrap_or(end + chrono::Duration::days(1));

        if skip_dates.contains(&date_str) {
            continue;
        }

        throttle.wait();
        let results = fetch_document_list(client, &date_str, api_key, timeout)
            .map_err(|error| format!("EDINET discovery failed for {date_str}: {error}"))?;

        for document in results.into_iter().filter(DocumentResult::is_annual_report) {
            let Some(ticker) = resolver.resolve_api_result(
                document.sec_code.as_deref(),
                document.edinet_code.as_deref(),
                target_tickers,
            ) else {
                continue;
            };
            resolver.check_filer_name(&ticker, document.filer_name.as_deref());
            if !seen.insert((ticker.clone(), document.doc_id.clone())) {
                continue;
            }
            candidates.push(AnnualCandidate { ticker, document });
        }
    }

    Ok(select_effective_reports(candidates))
}

fn select_effective_reports(candidates: Vec<AnnualCandidate>) -> BTreeMap<String, Vec<DiscoveredReport>> {
    let active: Vec<AnnualCandidate> = candidates
        .into_iter()
        .filter(|candidate| !candidate.document.is_withdrawn())
        .collect();

    let superseded: HashSet<String> = active
        .iter()
        .filter(|candidate| candidate.document.is_correction())
        .filter_map(|candidate| candidate.document.parent_doc_id.as_deref())
        .map(str::trim)
        .filter(|parent| !parent.is_empty())
        .map(str::to_string)
        .collect();

    let mut selected_by_period: HashMap<(String, String), AnnualCandidate> = HashMap::new();
    for candidate in active {
        if superseded.contains(&candidate.document.doc_id) {
            continue;
        }
        let period_key = candidate
            .document
            .period_end
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("doc:{}", candidate.document.doc_id));
        let key = (candidate.ticker.clone(), period_key);
        match selected_by_period.get_mut(&key) {
            None => {
                selected_by_period.insert(key, candidate);
            }
            Some(existing) if candidate_is_newer(&candidate, existing) => {
                *existing = candidate;
            }
            Some(_) => {}
        }
    }

    let mut reports: BTreeMap<String, Vec<DiscoveredReport>> = BTreeMap::new();
    for candidate in selected_by_period.into_values() {
        let document = candidate.document;
        reports
            .entry(candidate.ticker.clone())
            .or_default()
            .push(DiscoveredReport {
                ticker: candidate.ticker,
                doc_id: document.doc_id,
                fiscal_year: fiscal_year_from_period_end(document.period_end.as_deref()),
                period_end: document.period_end,
                submit_date: document.submit_date_time,
                filer_name: document.filer_name,
            });
    }
    for rows in reports.values_mut() {
        rows.sort_by(|left, right| {
            left.period_end
                .cmp(&right.period_end)
                .then_with(|| left.submit_date.cmp(&right.submit_date))
                .then_with(|| left.doc_id.cmp(&right.doc_id))
        });
    }
    reports
}

fn candidate_is_newer(left: &AnnualCandidate, right: &AnnualCandidate) -> bool {
    left.document
        .submit_date_time
        .cmp(&right.document.submit_date_time)
        .then_with(|| left.document.is_correction().cmp(&right.document.is_correction()))
        .then_with(|| left.document.doc_id.cmp(&right.document.doc_id))
        .is_gt()
}

fn fiscal_year_from_period_end(period_end: Option<&str>) -> String {
    let Some(pe) = period_end else {
        return "unknown".to_string();
    };
    let bytes = pe.as_bytes();
    if bytes.len() < 4 || !bytes[..4].iter().all(|b| b.is_ascii_digit()) {
        return "unknown".to_string();
    }
    let year = std::str::from_utf8(&bytes[..4]).unwrap_or("unknown");
    format!("FY{year}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        doc_id: &str,
        form_code: &str,
        parent_doc_id: Option<&str>,
        withdrawal_status: &str,
        submit_date_time: &str,
    ) -> AnnualCandidate {
        AnnualCandidate {
            ticker: "1234".to_string(),
            document: DocumentResult {
                doc_id: doc_id.to_string(),
                ordinance_code: "010".to_string(),
                form_code: form_code.to_string(),
                doc_type_code: None,
                parent_doc_id: parent_doc_id.map(str::to_string),
                withdrawal_status: Some(withdrawal_status.to_string()),
                doc_info_edit_status: None,
                sec_code: Some("12340".to_string()),
                edinet_code: Some("E00001".to_string()),
                filer_name: Some("test".to_string()),
                period_end: Some("2025-03-31".to_string()),
                submit_date_time: Some(submit_date_time.to_string()),
            },
        }
    }

    #[test]
    fn fiscal_year_extracts_year_prefix() {
        assert_eq!(fiscal_year_from_period_end(Some("2024-03-31")), "FY2024");
        assert_eq!(fiscal_year_from_period_end(Some("2024")), "FY2024");
    }

    #[test]
    fn fiscal_year_handles_missing_or_garbage() {
        assert_eq!(fiscal_year_from_period_end(None), "unknown");
        assert_eq!(fiscal_year_from_period_end(Some("abcd-03-31")), "unknown");
        assert_eq!(fiscal_year_from_period_end(Some("")), "unknown");
    }

    #[test]
    fn active_correction_supersedes_original() {
        let reports = select_effective_reports(vec![
            candidate("ORIGINAL", "030000", None, "0", "2025-06-20T10:00:00"),
            candidate(
                "CORRECTION",
                "030001",
                Some("ORIGINAL"),
                "0",
                "2025-07-01T10:00:00",
            ),
        ]);
        let rows = reports.get("1234").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].doc_id, "CORRECTION");
    }

    #[test]
    fn withdrawn_correction_does_not_supersede_original() {
        let reports = select_effective_reports(vec![
            candidate("ORIGINAL", "030000", None, "0", "2025-06-20T10:00:00"),
            candidate(
                "CORRECTION",
                "030001",
                Some("ORIGINAL"),
                "1",
                "2025-07-01T10:00:00",
            ),
        ]);
        let rows = reports.get("1234").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].doc_id, "ORIGINAL");
    }

    #[test]
    fn correction_chain_keeps_latest_active_leaf() {
        let reports = select_effective_reports(vec![
            candidate("ORIGINAL", "030000", None, "0", "2025-06-20T10:00:00"),
            candidate(
                "CORRECTION1",
                "030001",
                Some("ORIGINAL"),
                "0",
                "2025-07-01T10:00:00",
            ),
            candidate(
                "CORRECTION2",
                "030001",
                Some("CORRECTION1"),
                "0",
                "2025-07-10T10:00:00",
            ),
        ]);
        let rows = reports.get("1234").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].doc_id, "CORRECTION2");
    }
}
