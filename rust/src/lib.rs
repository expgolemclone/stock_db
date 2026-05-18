use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

mod artifact;
mod auto_update;
mod db;
mod financials;
mod inventory;
pub mod screening;
mod share_classes;
mod stooq;
mod types;
mod xml_util;

type PyFinancials = std::collections::HashMap<
    String,
    std::collections::HashMap<String, std::collections::HashMap<String, Option<f64>>>,
>;

/// Custom exception for inventory tag mismatch errors.
#[derive(Debug, Clone)]
pub struct InventoriesTagMismatchError {
    pub message: String,
}

impl InventoriesTagMismatchError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for InventoriesTagMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for InventoriesTagMismatchError {}

impl From<InventoriesTagMismatchError> for PyErr {
    fn from(err: InventoriesTagMismatchError) -> PyErr {
        PyRuntimeError::new_err(err.message)
    }
}

/// EDINET XBRL artifact parser — Rust-backed core.
#[pymodule]
fn _edinet_xbrl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_inventories, m)?)?;
    m.add_function(wrap_pyfunction!(parse_xbrl_artifact, m)?)?;
    m.add_function(wrap_pyfunction!(parse_financials, m)?)?;
    m.add_function(wrap_pyfunction!(parse_share_classes, m)?)?;
    m.add_function(wrap_pyfunction!(parse_xbrl_financials_to_db, m)?)?;
    m.add_function(wrap_pyfunction!(parse_stooq_daily_file, m)?)?;
    m.add_function(wrap_pyfunction!(is_valid_xbrl_text, m)?)?;
    m.add_function(wrap_pyfunction!(is_valid_xbrl_path, m)?)?;
    Ok(())
}

/// Parse an EDINET XBRL artifact and return inventories-only balance sheet data.
///
/// Returns: dict[str, dict[str, float | None]]  (period → {"inventories": value})
#[pyfunction]
fn parse_inventories(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let artifact = py.allow_threads(|| {
        artifact::load_xbrl_artifact(path).map_err(|e| PyRuntimeError::new_err(e))
    })?;
    let result = py.allow_threads(|| {
        inventory::parse_inventories_from_artifact(&artifact)
            .map_err(|e: InventoriesTagMismatchError| PyRuntimeError::new_err(e.message))
    })?;

    let outer = pyo3::types::PyDict::new(py);
    for (period, items) in &result {
        let inner = pyo3::types::PyDict::new(py);
        for (key, value) in items {
            match value {
                Some(v) => inner.set_item(key, *v)?,
                None => inner.set_item(key, py.None())?,
            }
        }
        outer.set_item(period, inner)?;
    }
    Ok(outer.into())
}

/// Parse an EDINET XBRL artifact once and return financials plus share classes.
#[pyfunction]
fn parse_xbrl_artifact(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let (financials, share_classes) = py.allow_threads(|| parse_xbrl_artifact_inner(path))?;

    let outer = pyo3::types::PyDict::new(py);
    outer.set_item("financials", financials_to_py(py, &financials)?)?;
    outer.set_item("share_classes", share_classes_to_py(py, &share_classes)?)?;
    Ok(outer.into())
}

/// Parse an EDINET XBRL artifact and return canonical financial_items.
///
/// Returns: dict[str, dict[str, dict[str, float | None]]]
///          (period → statement → item_name → value)
#[pyfunction]
fn parse_financials(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let (financials, _) = py.allow_threads(|| parse_xbrl_artifact_inner(path))?;
    financials_to_py(py, &financials)
}

/// Parse an EDINET XBRL artifact and return share class issued-share details.
///
/// Returns: list[dict[str, str | float | bool]]
#[pyfunction]
fn parse_share_classes(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let (_, result) = py.allow_threads(|| parse_xbrl_artifact_inner(path))?;
    share_classes_to_py(py, &result)
}

/// Parse selected EDINET XBRL artifacts from SQLite and replace DB rows.
#[pyfunction]
#[pyo3(signature = (
    db_path,
    ticker=None,
    from_ticker=None,
    skip_existing=true,
    emit_progress=false,
))]
fn parse_xbrl_financials_to_db(
    py: Python<'_>,
    db_path: &str,
    ticker: Option<String>,
    from_ticker: Option<String>,
    skip_existing: bool,
    emit_progress: bool,
) -> PyResult<PyObject> {
    let summary = py.allow_threads(|| {
        db::parse_xbrl_financials_to_db(
            db_path,
            ticker.as_deref(),
            from_ticker.as_deref(),
            skip_existing,
            emit_progress,
        )
        .map_err(PyRuntimeError::new_err)
    })?;
    summary_to_py(py, &summary)
}

#[pyfunction]
fn is_valid_xbrl_text(content: &str) -> bool {
    xml_util::is_valid_xbrl_text(content)
}

#[pyfunction]
fn is_valid_xbrl_path(path: Option<&str>) -> bool {
    artifact::is_valid_xbrl_path(path)
}

#[pyfunction]
fn parse_stooq_daily_file(path: &str) -> PyResult<Vec<(String, String, f64)>> {
    stooq::parse_daily_file(path).map_err(PyRuntimeError::new_err)
}

fn parse_xbrl_artifact_inner(path: &str) -> PyResult<(PyFinancials, Vec<types::ShareClassFact>)> {
    let artifact = artifact::load_xbrl_artifact(path).map_err(|e| PyRuntimeError::new_err(e))?;
    let financials = financials::parse_financials_from_artifact(&artifact)
        .map_err(|e: InventoriesTagMismatchError| PyRuntimeError::new_err(e.message))?;
    let share_classes = share_classes::parse_share_classes_from_artifact(&artifact);
    Ok((financials, share_classes))
}

fn financials_to_py(py: Python<'_>, result: &PyFinancials) -> PyResult<PyObject> {
    let outer = pyo3::types::PyDict::new(py);
    for (period, statements) in result {
        let mid = pyo3::types::PyDict::new(py);
        for (statement, items) in statements {
            let inner = pyo3::types::PyDict::new(py);
            for (key, value) in items {
                match value {
                    Some(v) => inner.set_item(key, *v)?,
                    None => inner.set_item(key, py.None())?,
                }
            }
            mid.set_item(statement, inner)?;
        }
        outer.set_item(period, mid)?;
    }
    Ok(outer.into())
}

fn share_classes_to_py(py: Python<'_>, result: &[types::ShareClassFact]) -> PyResult<PyObject> {
    let rows = pyo3::types::PyList::empty(py);
    for row in result {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("period", &row.period)?;
        item.set_item("class_key", &row.class_key)?;
        item.set_item("class_name", &row.class_name)?;
        item.set_item("shares", row.shares)?;
        item.set_item("is_preferred", row.is_preferred)?;
        item.set_item("source_kind", &row.source_kind)?;
        rows.append(item)?;
    }
    Ok(rows.into())
}

fn summary_to_py(py: Python<'_>, summary: &db::ParseDbSummary) -> PyResult<PyObject> {
    let result = pyo3::types::PyDict::new(py);
    result.set_item("ok", summary.ok)?;
    result.set_item("errors", summary.errors)?;
    result.set_item("skipped", summary.skipped)?;
    result.set_item("no_xbrl_files", summary.no_xbrl_files)?;

    let rows = pyo3::types::PyList::empty(py);
    for row in &summary.results {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("ticker", &row.ticker)?;
        item.set_item("status", &row.status)?;
        item.set_item("financial_rows", row.financial_rows)?;
        item.set_item("period_count", row.period_count)?;
        item.set_item("share_class_rows", row.share_class_rows)?;
        match &row.message {
            Some(message) => item.set_item("message", message)?,
            None => item.set_item("message", py.None())?,
        }
        rows.append(item)?;
    }
    result.set_item("results", rows)?;
    Ok(result.into())
}
