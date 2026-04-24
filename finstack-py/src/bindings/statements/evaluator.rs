//! Python wrappers for the statement evaluator and results.

use super::types::PyFinancialModelSpec;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::market_data::context::PyMarketContext;
use crate::bindings::pandas_utils::{selected_table_to_dataframe, table_to_dataframe};
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyDict;

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
    pub(crate) inner: finstack_statements::evaluator::StatementResult,
}

#[pymethods]
impl PyStatementResult {
    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_statements::evaluator::StatementResult =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
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

    /// Export to pandas long-format ``DataFrame``.
    ///
    /// Columns: ``node_id``, ``period``, ``value``, ``value_money``,
    /// ``currency``, ``value_type``. The monetary columns are populated for
    /// `Money`-typed nodes and left null for scalar nodes; exposing them here
    /// matches the Rust schema so currency/fixed-point precision is never
    /// silently dropped at the Python boundary.
    fn to_pandas_long<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let table = self.inner.to_table_long().map_err(display_to_py)?;
        selected_table_to_dataframe(
            py,
            &table,
            &[
                ("node_id", "node_id"),
                ("period_id", "period"),
                ("value", "value"),
                ("value_money", "value_money"),
                ("currency", "currency"),
                ("value_type", "value_type"),
            ],
        )
    }

    /// Export to pandas wide-format ``DataFrame``.
    ///
    /// Rows are node identifiers, columns are period identifiers.
    fn to_pandas_wide<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let table = self.inner.to_table_wide().map_err(display_to_py)?;
        let df = table_to_dataframe(py, &table)?;
        df.call_method1("set_index", ("period_id",))?.getattr("T")
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
        let result = self.inner.evaluate(&model.inner).map_err(display_to_py)?;
        Ok(PyStatementResult { inner: result })
    }

    /// Evaluate a financial model with market context and an as-of date.
    ///
    /// Use this for capital-structure-aware models and for as-of evaluation
    /// that hides future actual values.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     The model specification to evaluate.
    /// market : MarketContext
    ///     Market data context used for instrument pricing.
    /// as_of : datetime.date
    ///     Valuation/as-of date.
    fn evaluate_with_market(
        &mut self,
        model: &PyFinancialModelSpec,
        market: &PyMarketContext,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyStatementResult> {
        let as_of = py_to_date(as_of)?;
        let result = self
            .inner
            .evaluate_with_market(&model.inner, &market.inner, as_of)
            .map_err(display_to_py)?;
        Ok(PyStatementResult { inner: result })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_period_id(s: &str) -> PyResult<finstack_core::dates::PeriodId> {
    s.parse().map_err(crate::errors::core_to_py)
}

/// Register evaluator classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStatementResult>()?;
    m.add_class::<PyEvaluator>()?;
    Ok(())
}
