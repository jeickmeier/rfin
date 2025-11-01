//! Evaluator for financial models.

use crate::core::dates::periods::PyPeriodId;
use crate::core::market_data::context::PyMarketContext;
use crate::core::utils::py_to_date;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::evaluator::{Evaluator, Results, ResultsMeta};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyAnyMethods, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

/// Metadata about evaluation results.
#[pyclass(module = "finstack.statements.evaluator", name = "ResultsMeta", frozen)]
#[derive(Clone, Debug)]
pub struct PyResultsMeta {
    pub(crate) inner: ResultsMeta,
}

impl PyResultsMeta {
    pub(crate) fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyResultsMeta {
    #[getter]
    /// Evaluation time in milliseconds.
    ///
    /// Returns
    /// -------
    /// int | None
    ///     Evaluation time if available
    fn eval_time_ms(&self) -> Option<u64> {
        self.inner.eval_time_ms
    }

    #[getter]
    /// Number of nodes evaluated.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of nodes
    fn num_nodes(&self) -> usize {
        self.inner.num_nodes
    }

    #[getter]
    /// Number of periods evaluated.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of periods
    fn num_periods(&self) -> usize {
        self.inner.num_periods
    }

    fn __repr__(&self) -> String {
        format!(
            "ResultsMeta(nodes={}, periods={}, eval_time_ms={:?})",
            self.inner.num_nodes, self.inner.num_periods, self.inner.eval_time_ms
        )
    }
}

/// Results from evaluating a financial model.
#[pyclass(module = "finstack.statements.evaluator", name = "Results")]
#[derive(Clone, Debug)]
pub struct PyResults {
    pub(crate) inner: Results,
}

impl PyResults {
    pub(crate) fn new(inner: Results) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyResults {
    #[pyo3(text_signature = "(self, node_id, period_id)")]
    /// Get the value for a node at a specific period.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period identifier
    ///
    /// Returns
    /// -------
    /// float | None
    ///     Value if found, None otherwise
    fn get(&self, node_id: &str, period_id: &PyPeriodId) -> Option<f64> {
        self.inner.get(node_id, &period_id.inner)
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Get all period values for a specific node.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// dict[PeriodId, float] | None
    ///     Period values if node exists
    fn get_node(&self, node_id: &str, py: Python<'_>) -> Option<PyObject> {
        self.inner.get_node(node_id).map(|period_map| {
            let dict = PyDict::new(py);
            for (period_id, value) in period_map {
                dict.set_item(PyPeriodId::new(*period_id), value).ok();
            }
            dict.into()
        })
    }

    #[pyo3(text_signature = "(self, node_id, period_id, default)")]
    /// Get value or default.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period identifier
    /// default : float
    ///     Default value if not found
    ///
    /// Returns
    /// -------
    /// float
    ///     Value or default
    fn get_or(&self, node_id: &str, period_id: &PyPeriodId, default: f64) -> f64 {
        self.inner.get_or(node_id, &period_id.inner, default)
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Get an iterator over all periods for a node.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// list[tuple[PeriodId, float]]
    ///     List of period-value pairs
    fn all_periods(&self, node_id: &str, py: Python<'_>) -> PyObject {
        let list = PyList::empty(py);
        for (period_id, value) in self.inner.all_periods(node_id) {
            let tuple = (PyPeriodId::new(*period_id), value);
            list.append(tuple).ok();
        }
        list.into()
    }

    #[getter]
    /// Get all node results.
    ///
    /// Returns
    /// -------
    /// dict[str, dict[PeriodId, float]]
    ///     Map of node_id to period values
    fn nodes(&self, py: Python<'_>) -> PyObject {
        let dict = PyDict::new(py);
        for (node_id, period_map) in &self.inner.nodes {
            let inner_dict = PyDict::new(py);
            for (period_id, value) in period_map {
                inner_dict.set_item(PyPeriodId::new(*period_id), value).ok();
            }
            dict.set_item(node_id, inner_dict).ok();
        }
        dict.into()
    }

    #[getter]
    /// Get evaluation metadata.
    ///
    /// Returns
    /// -------
    /// ResultsMeta
    ///     Evaluation metadata
    fn meta(&self) -> PyResultsMeta {
        PyResultsMeta::new(self.inner.meta.clone())
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// Results
    ///     Deserialized results
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "Results(nodes={}, periods={})",
            self.inner.nodes.len(),
            self.inner.meta.num_periods
        )
    }
}

/// Evaluator for financial models.
///
/// The evaluator compiles formulas, resolves dependencies, and evaluates
/// nodes period-by-period according to precedence rules.
#[pyclass(
    module = "finstack.statements.evaluator",
    name = "Evaluator",
    unsendable
)]
pub struct PyEvaluator {
    inner: Evaluator,
}

#[pymethods]
impl PyEvaluator {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a new evaluator.
    ///
    /// Returns
    /// -------
    /// Evaluator
    ///     Evaluator instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: Evaluator::new(),
        }
    }

    #[pyo3(text_signature = "(self, model)")]
    /// Evaluate a financial model over all periods.
    ///
    /// This is a convenience method that calls `evaluate_with_market_context`
    /// with no market context. If your model uses capital structure with cs.*
    /// references, use `evaluate_with_market_context` and provide market data.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    ///
    /// Returns
    /// -------
    /// Results
    ///     Evaluation results
    fn evaluate(&mut self, py: Python<'_>, model: &PyFinancialModelSpec) -> PyResult<PyResults> {
        // Release GIL for compute-heavy statement evaluation
        let results = py.allow_threads(|| {
            self.inner.evaluate(&model.inner).map_err(stmt_to_py)
        })?;
        Ok(PyResults::new(results))
    }

    #[pyo3(text_signature = "(self, model, market_ctx, as_of)")]
    /// Evaluate a financial model with market context for pricing.
    ///
    /// This method allows you to provide market context for pricing capital
    /// structure instruments. If capital structure is defined but market context
    /// is not provided, capital structure cashflows will not be computed (cs.*
    /// references will fail at runtime).
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    /// market_ctx : MarketContext
    ///     Market context for pricing instruments
    /// as_of : date
    ///     Valuation date for pricing
    ///
    /// Returns
    /// -------
    /// Results
    ///     Evaluation results
    fn evaluate_with_market_context(
        &mut self,
        py: Python<'_>,
        model: &PyFinancialModelSpec,
        market_ctx: &PyMarketContext,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyResults> {
        let as_of_date = py_to_date(as_of)?;
        
        // Release GIL for compute-heavy statement evaluation with market context
        let results = py.allow_threads(|| {
            self.inner
                .evaluate_with_market_context(&model.inner, Some(&market_ctx.inner), Some(as_of_date))
                .map_err(stmt_to_py)
        })?;
        
        Ok(PyResults::new(results))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "evaluator")?;
    module.setattr("__doc__", "Evaluator for financial models.")?;

    module.add_class::<PyResultsMeta>()?;
    module.add_class::<PyResults>()?;
    module.add_class::<PyEvaluator>()?;

    parent.add_submodule(&module)?;
    parent.setattr("evaluator", &module)?;

    Ok(vec!["ResultsMeta", "Results", "Evaluator"])
}
