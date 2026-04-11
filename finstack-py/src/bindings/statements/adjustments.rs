//! Python wrappers for EBITDA normalization and adjustments.

use super::evaluator::PyStatementResult;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn stmts_to_py(e: finstack_statements::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// NormalizationConfig — JSON wrapper
// ---------------------------------------------------------------------------

/// Configuration for normalizing a financial metric.
#[pyclass(
    name = "NormalizationConfig",
    module = "finstack.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyNormalizationConfig {
    pub(super) inner: finstack_statements::adjustments::types::NormalizationConfig,
}

#[pymethods]
impl PyNormalizationConfig {
    /// Create a new normalization configuration for a target node.
    #[new]
    fn new(target_node: &str) -> Self {
        Self {
            inner: finstack_statements::adjustments::types::NormalizationConfig::new(target_node),
        }
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_statements::adjustments::types::NormalizationConfig =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Target node being normalized.
    #[getter]
    fn target_node(&self) -> &str {
        &self.inner.target_node
    }

    /// Number of adjustments configured.
    #[getter]
    fn adjustment_count(&self) -> usize {
        self.inner.adjustments.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "NormalizationConfig(target={:?}, adjustments={})",
            self.inner.target_node,
            self.inner.adjustments.len()
        )
    }
}

// ---------------------------------------------------------------------------
// normalize() function
// ---------------------------------------------------------------------------

/// Run normalization on statement results.
///
/// Parameters
/// ----------
/// results : StatementResult
///     Evaluated statement results.
/// config : NormalizationConfig
///     Normalization configuration (target node + adjustments).
///
/// Returns
/// -------
/// str
///     JSON-serialized list of ``NormalizationResult`` objects.
#[pyfunction]
fn normalize(results: &PyStatementResult, config: &PyNormalizationConfig) -> PyResult<String> {
    let norm_results = finstack_statements::adjustments::engine::NormalizationEngine::normalize(
        &results.inner,
        &config.inner,
    )
    .map_err(stmts_to_py)?;

    serde_json::to_string(&norm_results).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Run normalization and return results as a list of dicts.
///
/// Parameters
/// ----------
/// results : StatementResult
///     Evaluated statement results.
/// config : NormalizationConfig
///     Normalization configuration.
///
/// Returns
/// -------
/// list[dict]
///     List of normalization result dictionaries with keys:
///     ``period``, ``base_value``, ``final_value``, ``adjustments``.
#[pyfunction]
fn normalize_to_dicts<'py>(
    py: Python<'py>,
    results: &PyStatementResult,
    config: &PyNormalizationConfig,
) -> PyResult<Vec<Bound<'py, PyDict>>> {
    let norm_results = finstack_statements::adjustments::engine::NormalizationEngine::normalize(
        &results.inner,
        &config.inner,
    )
    .map_err(stmts_to_py)?;

    let mut out = Vec::with_capacity(norm_results.len());
    for nr in &norm_results {
        let dict = PyDict::new(py);
        dict.set_item("period", nr.period.to_string())?;
        dict.set_item("base_value", nr.base_value)?;
        dict.set_item("final_value", nr.final_value)?;

        let adj_list: Vec<Bound<'py, PyDict>> = nr
            .adjustments
            .iter()
            .map(|a| {
                let d = PyDict::new(py);
                d.set_item("id", &a.adjustment_id)?;
                d.set_item("name", &a.name)?;
                d.set_item("raw_amount", a.raw_amount)?;
                d.set_item("capped_amount", a.capped_amount)?;
                d.set_item("is_capped", a.is_capped)?;
                Ok(d)
            })
            .collect::<PyResult<Vec<_>>>()?;
        dict.set_item("adjustments", adj_list)?;
        out.push(dict);
    }
    Ok(out)
}

/// Register adjustment classes and functions.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNormalizationConfig>()?;
    m.add_function(pyo3::wrap_pyfunction!(normalize, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(normalize_to_dicts, m)?)?;
    Ok(())
}
