use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

mod artifact;
mod financials;
mod inventory;
mod share_classes;
mod types;
mod xml_util;

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
    m.add_function(wrap_pyfunction!(parse_financials, m)?)?;
    m.add_function(wrap_pyfunction!(parse_share_classes, m)?)?;
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

/// Parse an EDINET XBRL artifact and return canonical financial_items.
///
/// Returns: dict[str, dict[str, dict[str, float | None]]]
///          (period → statement → item_name → value)
#[pyfunction]
fn parse_financials(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let artifact = py.allow_threads(|| {
        artifact::load_xbrl_artifact(path).map_err(|e| PyRuntimeError::new_err(e))
    })?;
    let result = py.allow_threads(|| {
        financials::parse_financials_from_artifact(&artifact)
            .map_err(|e: InventoriesTagMismatchError| PyRuntimeError::new_err(e.message))
    })?;

    let outer = pyo3::types::PyDict::new(py);
    for (period, statements) in &result {
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

/// Parse an EDINET XBRL artifact and return share class issued-share details.
///
/// Returns: list[dict[str, str | float | bool]]
#[pyfunction]
fn parse_share_classes(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let artifact = py.allow_threads(|| {
        artifact::load_xbrl_artifact(path).map_err(|e| PyRuntimeError::new_err(e))
    })?;
    let result = py.allow_threads(|| share_classes::parse_share_classes_from_artifact(&artifact));

    let rows = pyo3::types::PyList::empty(py);
    for row in &result {
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
