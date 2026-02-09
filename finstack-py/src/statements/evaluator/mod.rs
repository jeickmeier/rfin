//! Evaluator for financial models.

use crate::core::dates::periods::PyPeriodId;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::{MonteCarloConfig, MonteCarloResults as RsMonteCarloResults};
use finstack_statements::evaluator::{Evaluator, ResultsMeta, StatementResult};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyAnyMethods, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

/// Metadata about evaluation results.
#[pyclass(module = "finstack.statements.evaluator", name = "ResultsMeta", frozen)]
#[derive(Clone, Debug)]
pub struct PyStatementResultMeta {
    pub(crate) inner: ResultsMeta,
}

impl PyStatementResultMeta {
    pub(crate) fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStatementResultMeta {
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
#[pyclass(module = "finstack.statements.evaluator", name = "StatementResult")]
#[derive(Clone, Debug)]
pub struct PyStatementResult {
    pub(crate) inner: StatementResult,
}

impl PyStatementResult {
    pub(crate) fn new(inner: StatementResult) -> Self {
        Self { inner }
    }
}

/// Monte Carlo results for statement forecasts.
#[pyclass(
    module = "finstack.statements.evaluator",
    name = "MonteCarloResults",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyMonteCarloResults {
    pub(crate) inner: RsMonteCarloResults,
}

impl PyMonteCarloResults {
    pub(crate) fn new(inner: RsMonteCarloResults) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStatementResult {
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
    fn get_node(&self, node_id: &str, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .get_node(node_id)
            .map(|period_map| -> PyResult<Py<PyAny>> {
                let dict = PyDict::new(py);
                for (period_id, value) in period_map {
                    dict.set_item(PyPeriodId::new(*period_id), value)?;
                }
                Ok(dict.into())
            })
            .transpose()
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
    fn all_periods(&self, node_id: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for (period_id, value) in self.inner.all_periods(node_id) {
            let tuple = (PyPeriodId::new(*period_id), value);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    #[getter]
    /// Get all node results.
    ///
    /// Returns
    /// -------
    /// dict[str, dict[PeriodId, float]]
    ///     Map of node_id to period values
    fn nodes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (node_id, period_map) in &self.inner.nodes {
            let inner_dict = PyDict::new(py);
            for (period_id, value) in period_map {
                inner_dict.set_item(PyPeriodId::new(*period_id), value)?;
            }
            dict.set_item(node_id, inner_dict)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get evaluation metadata.
    ///
    /// Returns
    /// -------
    /// ResultsMeta
    ///     Evaluation metadata
    fn meta(&self) -> PyStatementResultMeta {
        PyStatementResultMeta::new(self.inner.meta.clone())
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
    /// StatementResult
    ///     Deserialized results
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    #[pyo3(text_signature = "(self)")]
    /// Export results to long-format Polars DataFrame.
    ///
    /// Schema: (node_id, period_id, value, value_money, currency, value_type)
    ///
    /// Includes both f64 and Money representations where applicable.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     Long-format DataFrame with all node-period combinations
    ///
    /// Examples
    /// --------
    /// >>> df = results.to_polars_long()
    /// >>> print(df)
    /// # ┌─────────────┬───────────┬────────────┬──────────────┬──────────┬────────────┐
    /// # │ node_id     │ period_id │ value      │ value_money  │ currency │ value_type │
    /// # ├─────────────┼───────────┼────────────┼──────────────┼──────────┼────────────┤
    /// # │ revenue     │ 2025Q1    │ 100000.0   │ 100000.0     │ USD      │ monetary   │
    /// # │ revenue     │ 2025Q2    │ 105000.0   │ 105000.0     │ USD      │ monetary   │
    /// # │ margin_pct  │ 2025Q1    │ 0.35       │ null         │ null     │ scalar     │
    /// # └─────────────┴───────────┴────────────┴──────────────┴──────────┴────────────┘
    fn to_polars_long(&self) -> PyResult<pyo3_polars::PyDataFrame> {
        use finstack_statements::evaluator::to_polars_long;

        let df = to_polars_long(&self.inner).map_err(stmt_to_py)?;
        Ok(pyo3_polars::PyDataFrame(df))
    }

    #[pyo3(text_signature = "(self)")]
    /// Export results to wide-format Polars DataFrame.
    ///
    /// Schema: periods as rows, nodes as columns
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     Wide-format DataFrame with periods as rows and nodes as columns
    ///
    /// Examples
    /// --------
    /// >>> df = results.to_polars_wide()
    /// >>> print(df)
    /// # ┌───────────┬────────────┬──────────┐
    /// # │ period_id │ revenue    │ cogs     │
    /// # ├───────────┼────────────┼──────────┤
    /// # │ 2025Q1    │ 100000.0   │ 60000.0  │
    /// # │ 2025Q2    │ 105000.0   │ 63000.0  │
    /// # └───────────┴────────────┴──────────┘
    fn to_polars_wide(&self) -> PyResult<pyo3_polars::PyDataFrame> {
        use finstack_statements::evaluator::to_polars_wide;

        let df = to_polars_wide(&self.inner).map_err(stmt_to_py)?;
        Ok(pyo3_polars::PyDataFrame(df))
    }

    #[pyo3(text_signature = "(self, node_filter)")]
    /// Export results to long-format Polars DataFrame with node filtering.
    ///
    /// Schema: (node_id, period_id, value, value_money, currency, value_type)
    ///
    /// Parameters
    /// ----------
    /// node_filter : list[str]
    ///     List of node IDs to include (empty list includes all nodes)
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     Filtered long-format DataFrame
    ///
    /// Examples
    /// --------
    /// >>> df = results.to_polars_long_filtered(["revenue", "cogs"])
    /// >>> print(df)
    fn to_polars_long_filtered(
        &self,
        node_filter: Vec<String>,
    ) -> PyResult<pyo3_polars::PyDataFrame> {
        use finstack_statements::evaluator::to_polars_long_filtered;

        let node_filter_refs: Vec<&str> = node_filter.iter().map(|s| s.as_str()).collect();
        let df = to_polars_long_filtered(&self.inner, &node_filter_refs).map_err(stmt_to_py)?;
        Ok(pyo3_polars::PyDataFrame(df))
    }

    fn __repr__(&self) -> String {
        format!(
            "StatementResult(nodes={}, periods={})",
            self.inner.nodes.len(),
            self.inner.meta.num_periods
        )
    }
}

#[pymethods]
impl PyMonteCarloResults {
    #[getter]
    /// Number of Monte Carlo paths simulated.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of paths
    fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    #[getter]
    /// Percentiles computed for each metric/period.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Percentile values in [0.0, 1.0]
    fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    /// Get a percentile time series for a metric.
    ///
    /// Parameters
    /// ----------
    /// metric : str
    ///     Metric / node identifier (e.g. ``\"ebitda\"``)
    /// percentile : float
    ///     Percentile in [0.0, 1.0] (e.g. 0.95 for P95)
    ///
    /// Returns
    /// -------
    /// dict[PeriodId, float] | None
    ///     Map of period → percentile value or ``None`` if unavailable
    fn get_percentile(
        &self,
        metric: &str,
        percentile: f64,
        py: Python<'_>,
    ) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .get_percentile_series(metric, percentile)
            .map(|series| -> PyResult<Py<PyAny>> {
                let dict = PyDict::new(py);
                for (period_id, value) in series {
                    dict.set_item(PyPeriodId::new(period_id), value)?;
                }
                Ok(dict.into())
            })
            .transpose()
    }

    /// Estimate breach probability for a metric crossing a threshold.
    ///
    /// The current implementation returns the probability that
    /// ``metric > threshold`` in **any forecast period** across all paths.
    ///
    /// Parameters
    /// ----------
    /// metric : str
    ///     Metric / node identifier (e.g. ``\"leverage\"``)
    /// threshold : float
    ///     Breach threshold (e.g. 4.5 for leverage)
    ///
    /// Returns
    /// -------
    /// float | None
    ///     Breach probability in [0.0, 1.0] or ``None`` if metric not present
    fn breach_probability(&self, metric: &str, threshold: f64) -> Option<f64> {
        self.inner.breach_probability(metric, threshold)
    }

    fn __repr__(&self) -> String {
        format!(
            "MonteCarloResults(n_paths={}, percentiles={:?})",
            self.inner.n_paths, self.inner.percentiles
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
    /// StatementResult
    ///     Evaluation results
    fn evaluate(
        &mut self,
        py: Python<'_>,
        model: &PyFinancialModelSpec,
    ) -> PyResult<PyStatementResult> {
        // Release GIL for compute-heavy statement evaluation
        let results = py.detach(|| self.inner.evaluate(&model.inner).map_err(stmt_to_py))?;
        Ok(PyStatementResult::new(results))
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
    /// StatementResult
    ///     Evaluation results
    fn evaluate_with_market_context(
        &mut self,
        py: Python<'_>,
        model: &PyFinancialModelSpec,
        market_ctx: &PyMarketContext,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyStatementResult> {
        let as_of_date = py_to_date(as_of)?;

        // Release GIL for compute-heavy statement evaluation with market context
        let results = py.detach(|| {
            self.inner
                .evaluate_with_market_context(
                    &model.inner,
                    Some(&market_ctx.inner),
                    Some(as_of_date),
                )
                .map_err(stmt_to_py)
        })?;

        Ok(PyStatementResult::new(results))
    }

    #[pyo3(text_signature = "(self, model, n_paths, seed, percentiles=None)")]
    /// Evaluate a financial model using Monte Carlo simulation of forecasts.
    ///
    /// This method replays the model ``n_paths`` times with independent, but
    /// deterministic, seeds for stochastic forecast methods (Normal, LogNormal)
    /// and aggregates paths into percentile bands.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    /// n_paths : int
    ///     Number of Monte Carlo paths to simulate
    /// seed : int
    ///     Base random seed (same inputs ⇒ same results)
    /// percentiles : list[float] | None, optional
    ///     Percentiles in [0.0, 1.0] (default: [0.05, 0.5, 0.95])
    ///
    /// Returns
    /// -------
    /// MonteCarloResults
    ///     Monte Carlo percentile results
    fn evaluate_monte_carlo(
        &mut self,
        py: Python<'_>,
        model: &PyFinancialModelSpec,
        n_paths: usize,
        seed: u64,
        percentiles: Option<Vec<f64>>,
    ) -> PyResult<PyMonteCarloResults> {
        let mut cfg = MonteCarloConfig::new(n_paths, seed);
        if let Some(pcts) = percentiles {
            cfg = cfg.with_percentiles(pcts);
        }

        let inner = py.detach(|| {
            self.inner
                .evaluate_monte_carlo(&model.inner, &cfg)
                .map_err(stmt_to_py)
        })?;

        Ok(PyMonteCarloResults::new(inner))
    }
}

/// Evaluator with pre-configured market context.
///
/// This is a convenience wrapper that stores market context and as-of date
/// for capital structure evaluation.
#[pyclass(
    module = "finstack.statements.evaluator",
    name = "EvaluatorWithContext",
    unsendable
)]
pub struct PyEvaluatorWithContext {
    inner: finstack_statements::evaluator::EvaluatorWithContext,
}

#[pymethods]
impl PyEvaluatorWithContext {
    #[classmethod]
    #[pyo3(text_signature = "(cls, market_ctx, as_of)")]
    /// Create a new evaluator with pre-configured market context.
    ///
    /// Parameters
    /// ----------
    /// market_ctx : MarketContext
    ///     Market context with discount/forward curves
    /// as_of : date
    ///     Valuation date for pricing
    ///
    /// Returns
    /// -------
    /// EvaluatorWithContext
    ///     Evaluator instance with stored context
    fn new(
        _cls: &Bound<'_, PyType>,
        market_ctx: &crate::core::market_data::context::PyMarketContext,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::core::dates::utils::py_to_date;

        let as_of_date = py_to_date(as_of)?;
        let inner = finstack_statements::evaluator::Evaluator::with_market_context(
            &market_ctx.inner,
            as_of_date,
        );

        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self, model)")]
    /// Evaluate a financial model using the stored market context.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    ///
    /// Returns
    /// -------
    /// StatementResult
    ///     Evaluation results
    fn evaluate(
        &mut self,
        py: Python<'_>,
        model: &PyFinancialModelSpec,
    ) -> PyResult<PyStatementResult> {
        let results = py.detach(|| self.inner.evaluate(&model.inner).map_err(stmt_to_py))?;
        Ok(PyStatementResult::new(results))
    }
}

/// Dependency graph for financial models.
///
/// Provides DAG construction and topological ordering for model nodes.
#[pyclass(
    module = "finstack.statements.evaluator",
    name = "DependencyGraph",
    frozen
)]
pub struct PyDependencyGraph {
    pub(crate) inner: finstack_statements::evaluator::DependencyGraph,
}

#[pymethods]
impl PyDependencyGraph {
    #[classmethod]
    #[pyo3(text_signature = "(cls, model)")]
    /// Construct a dependency graph from a financial model.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    ///
    /// Returns
    /// -------
    /// DependencyGraph
    ///     Dependency graph instance
    fn from_model(_cls: &Bound<'_, PyType>, model: &PyFinancialModelSpec) -> PyResult<Self> {
        let inner = finstack_statements::evaluator::DependencyGraph::from_model(&model.inner)
            .map_err(stmt_to_py)?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    /// Get topological ordering of nodes.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Node IDs in evaluation order
    fn topological_order(&self) -> PyResult<Vec<String>> {
        finstack_statements::evaluator::evaluate_order(&self.inner)
            .map_err(stmt_to_py)
            .map(|order| order.into_iter().map(|s| s.to_string()).collect())
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Get direct dependencies for a node.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     List of node IDs that this node depends on
    fn dependencies(&self, node_id: &str) -> Vec<String> {
        self.inner
            .get_dependencies(node_id)
            .map(|deps| deps.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    #[pyo3(text_signature = "(self)")]
    /// Check if the graph has cycles.
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if there are circular dependencies
    fn has_cycle(&self) -> bool {
        // Try to get topological order; if it fails, there's a cycle
        finstack_statements::evaluator::evaluate_order(&self.inner).is_err()
    }

    fn __repr__(&self) -> String {
        let node_count = self.inner.dependencies.len();
        format!("DependencyGraph(nodes={})", node_count)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "evaluator")?;
    module.setattr("__doc__", "Evaluator for financial models.")?;

    module.add_class::<PyStatementResultMeta>()?;
    module.add_class::<PyStatementResult>()?;
    module.add_class::<PyMonteCarloResults>()?;
    module.add_class::<PyEvaluator>()?;
    module.add_class::<PyEvaluatorWithContext>()?;
    module.add_class::<PyDependencyGraph>()?;

    parent.add_submodule(&module)?;
    parent.setattr("evaluator", &module)?;

    Ok(vec![
        "ResultsMeta",
        "StatementResult",
        "MonteCarloResults",
        "Evaluator",
        "EvaluatorWithContext",
        "DependencyGraph",
    ])
}
