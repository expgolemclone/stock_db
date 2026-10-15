//! EDINET API v2 HTTP client: documents.json listing + XBRL ZIP download.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

const DOCUMENTS_LIST_URL: &str = "https://api.edinet-fsa.go.jp/api/v2/documents.json";
const DOCUMENT_DOWNLOAD_BASE: &str = "https://api.edinet-fsa.go.jp/api/v2/documents";

const ORDINANCE_CODE_ANNUAL: &str = "010";
const FORM_CODE_ANNUAL: &str = "030000";
const FORM_CODE_ANNUAL_CORRECTION: &str = "030001";

const ZIP_CONTENT_TYPES: &[&str] = &[
    "application/octet-stream",
    "application/zip",
    "application/x-zip-compressed",
];

/// Minimum-interval throttle for EDINET API calls (RULES: serial, 2s delay).
pub struct Throttle {
    interval: Duration,
    next: Option<Instant>,
}

impl Throttle {
    pub fn new(interval_seconds: f64) -> Self {
        Self {
            interval: Duration::from_secs_f64(interval_seconds.max(0.0)),
            next: None,
        }
    }

    pub fn wait(&mut self) {
        if self.interval.is_zero() {
            return;
        }
        let now = Instant::now();
        if let Some(next) = self.next {
            if now < next {
                std::thread::sleep(next - now);
            }
        }
        self.next = Some(Instant::now() + self.interval);
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentResult {
    #[serde(rename = "docID")]
    pub doc_id: String,
    #[serde(rename = "ordinanceCode")]
    pub ordinance_code: String,
    #[serde(rename = "formCode")]
    pub form_code: String,
    #[serde(rename = "docTypeCode")]
    pub doc_type_code: Option<String>,
    #[serde(rename = "parentDocID")]
    pub parent_doc_id: Option<String>,
    #[serde(rename = "withdrawalStatus")]
    pub withdrawal_status: Option<String>,
    #[serde(rename = "docInfoEditStatus")]
    pub doc_info_edit_status: Option<String>,
    #[serde(rename = "secCode")]
    pub sec_code: Option<String>,
    #[serde(rename = "edinetCode")]
    pub edinet_code: Option<String>,
    #[serde(rename = "filerName")]
    pub filer_name: Option<String>,
    #[serde(rename = "periodEnd")]
    pub period_end: Option<String>,
    #[serde(rename = "submitDateTime")]
    pub submit_date_time: Option<String>,
}

impl DocumentResult {
    /// 有価証券報告書またはその訂正報告書.
    pub fn is_annual_report(&self) -> bool {
        self.ordinance_code == ORDINANCE_CODE_ANNUAL
            && matches!(
                self.form_code.as_str(),
                FORM_CODE_ANNUAL | FORM_CODE_ANNUAL_CORRECTION
            )
    }

    pub fn is_correction(&self) -> bool {
        self.form_code == FORM_CODE_ANNUAL_CORRECTION
    }

    pub fn is_withdrawn(&self) -> bool {
        self.withdrawal_status.as_deref().map(str::trim) == Some("1")
    }
}

#[derive(Deserialize)]
struct DocumentListResponse {
    metadata: DocumentListMetadata,
    #[serde(default)]
    results: Vec<DocumentResult>,
}

#[derive(Deserialize)]
struct DocumentListMetadata {
    status: String,
}

/// Fetch the documents.json list for a single date. Returns empty on 404.
pub fn fetch_document_list(
    client: &reqwest::blocking::Client,
    date: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<Vec<DocumentResult>, String> {
    let resp = client
        .get(DOCUMENTS_LIST_URL)
        .query(&[("date", date), ("type", "2"), ("Subscription-Key", api_key)])
        .timeout(timeout)
        .send()
        .map_err(|e| format!("EDINET documents.json request failed for {date}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!(
            "EDINET documents.json returned {status} for {date}"
        ));
    }
    let parsed: DocumentListResponse = resp
        .json()
        .map_err(|e| format!("EDINET documents.json parse failed for {date}: {e}"))?;
    if parsed.metadata.status == "404" {
        return Ok(Vec::new());
    }
    if parsed.metadata.status != "200" {
        return Err(format!(
            "EDINET documents.json status {} for {}",
            parsed.metadata.status, date
        ));
    }
    Ok(parsed.results)
}

/// Download a ZIP from EDINET documents/{docID}?type=1, extract safely, and
/// install the archive and extracted artifact as one package directory.
/// Returns the extraction directory path.
pub fn download_xbrl_package(
    client: &reqwest::blocking::Client,
    doc_id: &str,
    dest_dir: &Path,
    api_key: &str,
    timeout: Duration,
) -> Result<PathBuf, String> {
    fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir {}: {e}", dest_dir.display()))?;
    let package_dest = dest_dir.join(doc_id);

    let temp_root = tempfile::Builder::new()
        .prefix(&format!("{doc_id}."))
        .tempdir_in(dest_dir)
        .map_err(|e| format!("tempdir in {}: {e}", dest_dir.display()))?;
    let temp_package = temp_root.path().join("package");
    let temp_zip = temp_package.join("source.zip");
    let temp_extract = temp_package.join("artifact");
    fs::create_dir_all(&temp_extract).map_err(|e| format!("mkdir temp extract: {e}"))?;

    let resp = client
        .get(format!("{DOCUMENT_DOWNLOAD_BASE}/{doc_id}"))
        .query(&[("type", "1"), ("Subscription-Key", api_key)])
        .timeout(timeout)
        .send()
        .map_err(|e| format!("EDINET download request failed for {doc_id}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "EDINET download returned {} for {}",
            resp.status(),
            doc_id
        ));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    if !ZIP_CONTENT_TYPES.iter().any(|t| content_type.contains(t)) {
        return Err(format!(
            "unexpected EDINET content type for {doc_id}: {content_type}"
        ));
    }

    let mut resp = resp;
    {
        let mut file = fs::File::create(&temp_zip).map_err(|e| format!("create temp zip: {e}"))?;
        std::io::copy(&mut resp, &mut file).map_err(|e| format!("write zip body: {e}"))?;
        file.flush().map_err(|e| format!("flush zip: {e}"))?;
    }

    extract_zip_safe(&temp_zip, &temp_extract)?;
    if !looks_like_xbrl_package(&temp_extract) {
        return Err(format!(
            "EDINET ZIP for {doc_id} did not contain XBRL artifacts"
        ));
    }

    install_package(&temp_package, &package_dest, temp_root.path())?;

    Ok(package_dest.join("artifact"))
}

fn install_package(staged: &Path, destination: &Path, staging_root: &Path) -> Result<(), String> {
    let backup = staging_root.join("previous");
    let had_previous = destination.exists();
    if had_previous {
        fs::rename(destination, &backup).map_err(|error| {
            format!(
                "move previous package {} aside: {error}",
                destination.display()
            )
        })?;
    }

    if let Err(install_error) = fs::rename(staged, destination) {
        if had_previous {
            if let Err(rollback_error) = fs::rename(&backup, destination) {
                return Err(format!(
                    "install package {}: {install_error}; rollback failed: {rollback_error}",
                    destination.display()
                ));
            }
        }
        return Err(format!(
            "install package {}: {install_error}",
            destination.display()
        ));
    }

    Ok(())
}

fn extract_zip_safe(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("open zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("zip entry {i}: {e}"))?;
        let entry_name = match entry.enclosed_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };
        let outpath = dest.join(&entry_name);
        // path-traversal guard: resolved path must stay under dest
        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir parent: {e}"))?;
        }
        let canonical_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
        let canonical_out = outpath
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or(canonical_dest.clone());
        if !canonical_out.starts_with(&canonical_dest) {
            return Err(format!(
                "unsafe zip member path (escapes dest): {}",
                entry_name.display()
            ));
        }
        if entry.is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("mkdir {}: {e}", outpath.display()))?;
        } else {
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("create {}: {e}", outpath.display()))?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| format!("extract: {e}"))?;
        }
    }
    Ok(())
}

fn looks_like_xbrl_package(dir: &Path) -> bool {
    fn has_xbrl_file(dir: &Path) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if has_xbrl_file(&path) {
                    return true;
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "xhtml" | "html" | "htm" | "xbrl"
                ) {
                    return true;
                }
            }
        }
        false
    }
    has_xbrl_file(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(form_code: &str) -> DocumentResult {
        DocumentResult {
            doc_id: "S1000001".into(),
            ordinance_code: "010".into(),
            form_code: form_code.into(),
            doc_type_code: None,
            parent_doc_id: None,
            withdrawal_status: None,
            doc_info_edit_status: None,
            sec_code: None,
            edinet_code: None,
            filer_name: None,
            period_end: None,
            submit_date_time: None,
        }
    }

    #[test]
    fn annual_report_filter_matches_original_and_correction() {
        assert!(document("030000").is_annual_report());
        assert!(document("030001").is_annual_report());
        assert!(document("030001").is_correction());
    }

    #[test]
    fn annual_report_filter_rejects_other_forms() {
        assert!(!document("040000").is_annual_report());
    }

    #[test]
    fn withdrawal_status_marks_document_inactive() {
        let mut r = document("030001");
        r.withdrawal_status = Some("1".into());
        assert!(r.is_withdrawn());
        r.withdrawal_status = Some("0".into());
        assert!(!r.is_withdrawn());
    }

    #[test]
    fn throttle_zero_interval_is_noop() {
        let mut t = Throttle::new(0.0);
        t.wait(); // must not block
    }

    #[test]
    fn package_install_replaces_archive_and_artifact_as_one_directory() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("S1000001");
        fs::create_dir_all(destination.join("artifact")).unwrap();
        fs::write(destination.join("artifact").join("old.xbrl"), "old").unwrap();

        let staging_root = root.path().join("staging");
        let staged = staging_root.join("package");
        fs::create_dir_all(staged.join("artifact")).unwrap();
        fs::write(staged.join("source.zip"), "zip").unwrap();
        fs::write(staged.join("artifact").join("new.xbrl"), "new").unwrap();

        install_package(&staged, &destination, &staging_root).unwrap();

        assert_eq!(fs::read_to_string(destination.join("source.zip")).unwrap(), "zip");
        assert_eq!(
            fs::read_to_string(destination.join("artifact").join("new.xbrl")).unwrap(),
            "new"
        );
        assert!(!destination.join("artifact").join("old.xbrl").exists());
    }
}
