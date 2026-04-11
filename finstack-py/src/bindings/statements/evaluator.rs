//! Python wrappers for the statement evaluator and results.

use super::types::PyFinancialModelSpec;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn stmts_to_py(e: finstack_statements::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// StatementResult
// ---------------------------------------------------------------------------

/// Results from evaluating a financial model.
#[pyclass(
    name = "StatementResult",
    module = "finstack.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyStatementResult {
    pub(super) inner: finstack_statements::evaluator::StatementResult,
}

#[pymethods]
impl PyStatementResult {
    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_statements::evaluator::StatementResult =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Get the value for a node at a specific period.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier (e.g. ``"revenue"``).
    /// period : str
    ///     Period identifier string (e.g. ``"2025Q1"``).
    fn get(&self, node_id: &str, period: &str) -> PyResult<Option<f64>> {
        let pid = parse_period_id(period)?;
        Ok(self.inner.get(node_id, &pid))
    }

    /// Get all period values for a specific node as a dict.
    fn get_node<'py>(
        &self,
        py: Python<'py>,
        node_id: &str,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        match self.inner.get_node(node_id) {
            Some(period_map) => {
                let dict = PyDict::new(py);
                for (pid, &val) in period_map {
                    dict.set_item(pid.to_string(), val)?;
                }
                Ok(Some(dict))
            }
            None => Ok(None),
        }
    }

    /// All node identifiers in the result.
    fn node_ids(&self) -> Vec<String> {
        self.inner.nodes.keys().cloned().collect()
    }

    /// Number of nodes in the result.
    #[getter]
    fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    /// Number of periods evaluated.
    #[getter]
    fn num_periods(&self) -> usize {
        self.inner.meta.num_periods
    }

    /// Evaluation time in milliseconds (if available).
    #[getter]
    fn eval_time_ms(&self) -> Option<u64> {
        self.inner.meta.eval_time_ms
    }

    /// Number of evaluation warnings.
    #[getter]
    fn warning_count(&self) -> usize {
        self.inner.meta.warnings.len()
    }

    /// Export to Polars long-format DataFrame.
    fn to_polars_long<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let df = self.inner.to_polars_long().map_err(stmts_to_py)?;
        let polars_df = pyo3_polars::PyDataFrame(df);
        polars_df.into_pyobject(py).map(Bound::into_any)
    }

    /// Export to Polars wide-format DataFrame.
    fn to_polars_wide<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let df = self.inner.to_polars_wide().map_err(stmts_to_py)?;
        let polars_df = pyo3_polars::PyDataFrame(df);
        polars_df.into_pyobject(py).map(Bound::into_any)
    }

    fn __repr__(&self) -> String {
        format!(
            "StatementResult(nodes={}, periods={})",
            self.inner.nodes.len(),
            self.inner.meta.num_periods
        )
    }
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// Evaluator for financial models.
#[pyclass(
    name = "Evaluator",
    module = "finstack.statements",
    skip_from_py_object
)]
pub struct PyEvaluator {
    inner: finstack_statements::evaluator::Evaluator,
}

#[pymethods]
impl PyEvaluator {
    /// Create a new evaluator.
    #[new]
    fn new() -> Self {
        Self {
            inner: finstack_statements::evaluator::Evaluator::new(),
        }
    }

    /// Evaluate a financial model.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     The model specification to evaluate.
    ///
    /// Returns
    /// -------
    /// StatementResult
    ///     Evaluation results with per-node, per-period values.
    fn evaluate(&mut self, model: &PyFinancialModelSpec) -> PyResult<PyStatementResult> {
        let result = self.inner.evaluate(&model.inner).map_err(stmts_to_py)?;
        Ok(PyStatementResult { inner: result })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_period_id(s: &str) -> PyResult<finstack_core::dates::PeriodId> {
    s.parse()
        .map_err(|e: finstack_core::Error| PyValueError::new_err(e.to_string()))
}

/// Register evaluator classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStatementResult>()?;
    m.add_class::<PyEvaluator>()?;
    Ok(())
}
