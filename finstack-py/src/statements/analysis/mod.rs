//! Analysis module bindings for financial models.
//!
//! This module provides Python bindings for analysis tools including:
//! - **Sensitivity analysis** - Parameter sweeps and tornado charts
//! - **Dependency tracing** - Identify direct and transitive dependencies
//! - **Formula explanation** - Break down calculations step-by-step
//! - **Reports** - Formatted output for P&L summaries and credit assessment
//! - **Scenario management** - Named scenario sets with diff/comparison helpers

mod backtesting;
mod corporate;
mod covenants;
mod credit_context;
mod explain;
mod orchestrator;
mod reports;
mod scenario_set;
mod variance;

use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyStatementResult;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements_analytics::analysis::types::{SensitivityScenario, TornadoEntry as CoreTornadoEntry};
use finstack_statements_analytics::analysis::MonteCarloConfig;
use finstack_statements_analytics::analysis::{
    generate_tornado_entries, ParameterSpec, SensitivityAnalyzer, SensitivityConfig,
    SensitivityMode, SensitivityResult,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::{wrap_pyfunction, Bound};
use std::str::FromStr;

/// Parameter specification for sensitivity analysis.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ParameterSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyParameterSpec {
    inner: ParameterSpec,
}

#[pymethods]
impl PyParameterSpec {
    #[new]
    #[pyo3(signature = (node_id, period_id, base_value, perturbations))]
    /// Create a new parameter specification.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period to vary
    /// base_value : float
    ///     Base value
    /// perturbations : list[float]
    ///     Perturbations to apply
    ///
    /// Returns
    /// -------
    /// ParameterSpec
    ///     Parameter specification
    fn new(
        node_id: String,
        period_id: &crate::core::dates::periods::PyPeriodId,
        base_value: f64,
        perturbations: Vec<f64>,
    ) -> Self {
        Self {
            inner: ParameterSpec::new(node_id, period_id.inner, base_value, perturbations),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (node_id, period_id, base_value, pct_range))]
    /// Create a parameter spec with percentage perturbations.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period to vary
    /// base_value : float
    ///     Base value
    /// pct_range : list[float]
    ///     Percentage range (e.g., [-10.0, 0.0, 10.0] for ±10%)
    ///
    /// Returns
    /// -------
    /// ParameterSpec
    ///     Parameter specification
    fn with_percentages(
        node_id: String,
        period_id: &crate::core::dates::periods::PyPeriodId,
        base_value: f64,
        pct_range: Vec<f64>,
    ) -> Self {
        Self {
            inner: ParameterSpec::with_percentages(node_id, period_id.inner, base_value, pct_range),
        }
    }

    #[getter]
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    fn period_id(&self) -> crate::core::dates::periods::PyPeriodId {
        crate::core::dates::periods::PyPeriodId::new(self.inner.period_id)
    }

    #[getter]
    fn base_value(&self) -> f64 {
        self.inner.base_value
    }

    #[getter]
    fn perturbations(&self) -> Vec<f64> {
        self.inner.perturbations.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ParameterSpec(node_id='{}', base_value={}, perturbations={})",
            self.inner.node_id,
            self.inner.base_value,
            self.inner.perturbations.len()
        )
    }
}

/// Sensitivity analysis mode.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "SensitivityMode",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy)]
pub struct PySensitivityMode {
    inner: SensitivityMode,
}

#[pymethods]
impl PySensitivityMode {
    #[classattr]
    const DIAGONAL: Self = Self {
        inner: SensitivityMode::Diagonal,
    };

    #[classattr]
    const FULL_GRID: Self = Self {
        inner: SensitivityMode::FullGrid,
    };

    #[classattr]
    const TORNADO: Self = Self {
        inner: SensitivityMode::Tornado,
    };

    fn __repr__(&self) -> String {
        format!("SensitivityMode.{:?}", self.inner)
    }
}

/// Sensitivity analysis configuration.
#[pyclass(module = "finstack.statements.analysis", name = "SensitivityConfig")]
pub struct PySensitivityConfig {
    inner: SensitivityConfig,
}

#[pymethods]
impl PySensitivityConfig {
    #[new]
    #[pyo3(signature = (mode))]
    /// Create a new sensitivity configuration.
    ///
    /// Parameters
    /// ----------
    /// mode : SensitivityMode
    ///     Analysis mode
    ///
    /// Returns
    /// -------
    /// SensitivityConfig
    ///     Configuration instance
    fn new(mode: &PySensitivityMode) -> Self {
        Self {
            inner: SensitivityConfig::new(mode.inner),
        }
    }

    #[pyo3(signature = (param))]
    /// Add a parameter to vary.
    ///
    /// Parameters
    /// ----------
    /// param : ParameterSpec
    ///     Parameter specification
    fn add_parameter(&mut self, param: &PyParameterSpec) {
        self.inner.add_parameter(param.inner.clone());
    }

    #[pyo3(signature = (metric))]
    /// Add a target metric to track.
    ///
    /// Parameters
    /// ----------
    /// metric : str
    ///     Metric identifier
    fn add_target_metric(&mut self, metric: String) {
        self.inner.add_target_metric(metric);
    }

    #[getter]
    fn mode(&self) -> PySensitivityMode {
        PySensitivityMode {
            inner: self.inner.mode,
        }
    }

    #[getter]
    fn parameters(&self) -> Vec<PyParameterSpec> {
        self.inner
            .parameters
            .iter()
            .map(|p| PyParameterSpec { inner: p.clone() })
            .collect()
    }

    #[getter]
    fn target_metrics(&self) -> Vec<String> {
        self.inner.target_metrics.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SensitivityConfig(mode={:?}, parameters={}, metrics={})",
            self.inner.mode,
            self.inner.parameters.len(),
            self.inner.target_metrics.len()
        )
    }
}

/// Result of a single sensitivity scenario.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "SensitivityScenario",
    frozen
)]
pub struct PySensitivityScenario {
    inner: SensitivityScenario,
}

#[pymethods]
impl PySensitivityScenario {
    #[getter]
    fn parameter_values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.parameter_values {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn results(&self) -> PyStatementResult {
        PyStatementResult::new(self.inner.results.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "SensitivityScenario(parameters={})",
            self.inner.parameter_values.len()
        )
    }
}

/// Results of sensitivity analysis.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "SensitivityResult",
    frozen
)]
pub struct PySensitivityResult {
    inner: SensitivityResult,
}

#[pymethods]
impl PySensitivityResult {
    #[getter]
    fn config(&self) -> PySensitivityConfig {
        PySensitivityConfig {
            inner: self.inner.config.clone(),
        }
    }

    #[getter]
    fn scenarios(&self) -> Vec<PySensitivityScenario> {
        self.inner
            .scenarios
            .iter()
            .map(|s| PySensitivityScenario { inner: s.clone() })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("SensitivityResult(scenarios={})", self.inner.len())
    }
}

/// Tornado chart entry summarizing downside and upside impacts.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "TornadoEntry",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyTornadoEntry {
    inner: CoreTornadoEntry,
}

impl PyTornadoEntry {
    fn from_inner(inner: CoreTornadoEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTornadoEntry {
    #[new]
    fn new(parameter_id: String, downside_impact: f64, upside_impact: f64) -> Self {
        Self::from_inner(CoreTornadoEntry {
            parameter_id,
            downside: downside_impact,
            upside: upside_impact,
        })
    }

    #[getter]
    fn parameter_id(&self) -> &str {
        &self.inner.parameter_id
    }

    #[getter]
    fn downside_impact(&self) -> f64 {
        self.inner.downside
    }

    #[getter]
    fn upside_impact(&self) -> f64 {
        self.inner.upside
    }

    #[getter]
    fn swing(&self) -> f64 {
        self.inner.swing()
    }

    fn __repr__(&self) -> String {
        format!(
            "TornadoEntry(parameter_id='{}', downside={}, upside={})",
            self.inner.parameter_id, self.inner.downside, self.inner.upside
        )
    }
}

/// Generate tornado chart entries from a sensitivity result.
#[pyfunction]
#[pyo3(signature = (result, metric))]
fn generate_tornado_chart(
    result: &PySensitivityResult,
    metric: &str,
) -> PyResult<Vec<PyTornadoEntry>> {
    let (node_id, period_hint) = parse_metric_target(metric)?;
    let entries = generate_tornado_entries(&result.inner, node_id, period_hint);
    Ok(entries
        .into_iter()
        .map(PyTornadoEntry::from_inner)
        .collect())
}

/// Goal-seek a driver node so a target node reaches the requested value.
#[pyfunction]
#[pyo3(
    signature = (model, target_node, target_period, target_value, driver_node, driver_period=None, update_model=true, bounds=None)
)]
#[allow(clippy::too_many_arguments)]
fn goal_seek(
    model: &mut PyFinancialModelSpec,
    target_node: &str,
    target_period: &str,
    target_value: f64,
    driver_node: &str,
    driver_period: Option<&str>,
    update_model: bool,
    bounds: Option<(f64, f64)>,
) -> PyResult<f64> {
    let target_period_id: finstack_core::dates::PeriodId = target_period.parse().map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid target period '{}': {}",
            target_period, e
        ))
    })?;
    let driver_period_str = driver_period.unwrap_or(target_period);
    let driver_period_id: finstack_core::dates::PeriodId =
        driver_period_str.parse().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid driver period '{}': {}",
                driver_period_str, e
            ))
        })?;
    finstack_statements_analytics::analysis::goal_seek(
        &mut model.inner,
        target_node,
        target_period_id,
        target_value,
        driver_node,
        driver_period_id,
        update_model,
        bounds,
    )
    .map_err(stmt_to_py)
}

/// Sensitivity analyzer for financial models.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "SensitivityAnalyzer",
    unsendable
)]
pub struct PySensitivityAnalyzer {
    model: PyFinancialModelSpec,
}

#[pymethods]
impl PySensitivityAnalyzer {
    #[new]
    #[pyo3(signature = (model))]
    /// Create a new sensitivity analyzer.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model to analyze
    ///
    /// Returns
    /// -------
    /// SensitivityAnalyzer
    ///     Analyzer instance
    fn new(model: &PyFinancialModelSpec) -> Self {
        Self {
            model: model.clone(),
        }
    }

    #[pyo3(signature = (config))]
    /// Run sensitivity analysis.
    ///
    /// Parameters
    /// ----------
    /// config : SensitivityConfig
    ///     Analysis configuration
    ///
    /// Returns
    /// -------
    /// SensitivityResult
    ///     Analysis results
    fn run(&self, py: Python<'_>, config: &PySensitivityConfig) -> PyResult<PySensitivityResult> {
        let analyzer = SensitivityAnalyzer::new(&self.model.inner);
        let result = py.detach(|| analyzer.run(&config.inner).map_err(stmt_to_py))?;

        Ok(PySensitivityResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!("SensitivityAnalyzer(model='{}')", self.model.inner.id)
    }
}

/// Monte Carlo simulation configuration.
///
/// Specifies the number of paths, the random seed, and which percentiles
/// to compute from the simulated distribution.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "MonteCarloConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMonteCarloConfig {
    pub(crate) inner: MonteCarloConfig,
}

#[pymethods]
impl PyMonteCarloConfig {
    #[new]
    #[pyo3(signature = (n_paths, seed))]
    /// Create a new Monte Carlo configuration.
    ///
    /// Default percentiles are ``[0.05, 0.5, 0.95]``.
    ///
    /// Parameters
    /// ----------
    /// n_paths : int
    ///     Number of Monte Carlo paths to simulate
    /// seed : int
    ///     Random seed for reproducibility
    ///
    /// Returns
    /// -------
    /// MonteCarloConfig
    ///     Configuration instance
    fn new(n_paths: usize, seed: u64) -> Self {
        Self {
            inner: MonteCarloConfig::new(n_paths, seed),
        }
    }

    /// Return a new config with the given percentiles.
    ///
    /// Parameters
    /// ----------
    /// percentiles : list[float]
    ///     Percentile values in [0.0, 1.0]
    ///
    /// Returns
    /// -------
    /// MonteCarloConfig
    ///     New configuration with updated percentiles
    fn with_percentiles(&self, percentiles: Vec<f64>) -> Self {
        Self {
            inner: self.inner.clone().with_percentiles(percentiles),
        }
    }

    #[getter]
    /// Number of Monte Carlo paths.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of paths
    fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    #[getter]
    /// Random seed.
    ///
    /// Returns
    /// -------
    /// int
    ///     Seed value
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    #[getter]
    /// Percentiles to compute.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Percentile values in [0.0, 1.0]
    fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "MonteCarloConfig(n_paths={}, seed={})",
            self.inner.n_paths, self.inner.seed
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "analysis")?;
    module.setattr(
        "__doc__",
        concat!(
            "Analysis tools for financial statement models.\n\n",
            "This module provides tools for:\n",
            "- Sensitivity analysis - Parameter sweeps and tornado charts\n",
            "- Dependency tracing - Identify direct and transitive dependencies\n",
            "- Formula explanation - Break down calculations step-by-step\n",
            "- Reports - Formatted output for P&L summaries and credit assessment"
        ),
    )?;

    // Sensitivity analysis types
    module.add_class::<PyParameterSpec>()?;
    module.add_class::<PySensitivityMode>()?;
    module.add_class::<PySensitivityConfig>()?;
    module.add_class::<PySensitivityScenario>()?;
    module.add_class::<PySensitivityResult>()?;
    module.add_class::<PySensitivityAnalyzer>()?;
    module.add_class::<PyTornadoEntry>()?;
    module.add_class::<PyMonteCarloConfig>()?;
    module.add_function(wrap_pyfunction!(generate_tornado_chart, &module)?)?;
    module.add_function(wrap_pyfunction!(goal_seek, &module)?)?;

    // Register explain types (dependency tracing, formula explanation)
    let explain_exports = explain::register(py, &module)?;

    // Register reports types (table builder, P&L summary, credit assessment)
    let reports_exports = reports::register(py, &module)?;

    if parent.hasattr("PercentileSeries")? {
        let percentile_series = parent.getattr("PercentileSeries")?;
        module.setattr("PercentileSeries", percentile_series)?;
    }

    // Register variance analysis types
    let variance_exports = variance::register(py, &module)?;

    // Register scenario management types
    let scenario_exports = scenario_set::register(py, &module)?;

    // Register backtesting types
    let backtesting_exports = backtesting::register(py, &module)?;

    // Register credit context types
    let credit_context_exports = credit_context::register(py, &module)?;

    // Register corporate DCF valuation types
    let corporate_exports = corporate::register(py, &module)?;

    // Register covenant analysis types
    let covenants_exports = covenants::register(py, &module)?;

    // Register orchestrator types (CorporateAnalysisBuilder, CorporateAnalysis, etc.)
    let orchestrator_exports = orchestrator::register(py, &module)?;

    parent.add_submodule(&module)?;
    parent.setattr("analysis", &module)?;

    // Collect all exports
    let mut all_exports = vec![
        "ParameterSpec",
        "SensitivityMode",
        "SensitivityConfig",
        "SensitivityScenario",
        "SensitivityResult",
        "SensitivityAnalyzer",
        "TornadoEntry",
        "MonteCarloConfig",
        "PercentileSeries",
        "generate_tornado_chart",
        "goal_seek",
    ];
    all_exports.extend(explain_exports);
    all_exports.extend(reports_exports);
    all_exports.extend(variance_exports);
    all_exports.extend(scenario_exports);
    all_exports.extend(backtesting_exports);
    all_exports.extend(credit_context_exports);
    all_exports.extend(corporate_exports);
    all_exports.extend(covenants_exports);
    all_exports.extend(orchestrator_exports);

    module.setattr("__all__", pyo3::types::PyList::new(py, &all_exports)?)?;

    Ok(all_exports)
}

fn parse_metric_target(metric: &str) -> PyResult<(&str, Option<finstack_core::dates::PeriodId>)> {
    if let Some((node, period_code)) = metric.split_once('@') {
        let trimmed = period_code.trim();
        let period = finstack_core::dates::PeriodId::from_str(trimmed).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid period identifier '{trimmed}': {err}"
            ))
        })?;
        Ok((node.trim(), Some(period)))
    } else {
        Ok((metric.trim(), None))
    }
}
