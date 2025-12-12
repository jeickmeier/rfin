//! Analysis module bindings for financial models.
//!
//! This module provides Python bindings for analysis tools including:
//! - **Sensitivity analysis** - Parameter sweeps and tornado charts
//! - **Dependency tracing** - Identify direct and transitive dependencies
//! - **Formula explanation** - Break down calculations step-by-step
//! - **Reports** - Formatted output for P&L summaries and credit assessment
//! - **Scenario management** - Named scenario sets with diff/comparison helpers

mod explain;
mod reports;
mod scenario_set;
mod variance;

use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyResults;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::types::SensitivityScenario;
use finstack_statements::analysis::{
    ParameterSpec, SensitivityAnalyzer, SensitivityConfig, SensitivityMode, SensitivityResult,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::{wrap_pyfunction, Bound};
use std::cmp::Ordering;
use std::str::FromStr;

/// Parameter specification for sensitivity analysis.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ParameterSpec",
    frozen
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
    frozen
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
    fn results(&self) -> PyResults {
        PyResults::new(self.inner.results.clone())
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
#[pyclass(module = "finstack.statements.analysis", name = "TornadoEntry", frozen)]
#[derive(Clone)]
pub struct PyTornadoEntry {
    parameter_id: String,
    downside_impact: f64,
    upside_impact: f64,
}

impl PyTornadoEntry {
    fn new_internal(parameter_id: String, downside: f64, upside: f64) -> Self {
        Self {
            parameter_id,
            downside_impact: downside,
            upside_impact: upside,
        }
    }
}

#[pymethods]
impl PyTornadoEntry {
    #[new]
    fn new(parameter_id: String, downside_impact: f64, upside_impact: f64) -> Self {
        Self::new_internal(parameter_id, downside_impact, upside_impact)
    }

    #[getter]
    fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    #[getter]
    fn downside_impact(&self) -> f64 {
        self.downside_impact
    }

    #[getter]
    fn upside_impact(&self) -> f64 {
        self.upside_impact
    }

    #[getter]
    fn swing(&self) -> f64 {
        self.upside_impact - self.downside_impact
    }

    fn __repr__(&self) -> String {
        format!(
            "TornadoEntry(parameter_id='{}', downside={}, upside={})",
            self.parameter_id, self.downside_impact, self.upside_impact
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
    let mut entries = Vec::new();

    for param in &result.inner.config.parameters {
        if let Some(entry) = build_tornado_entry(&result.inner, param, node_id, period_hint) {
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| {
        b.swing()
            .abs()
            .partial_cmp(&a.swing().abs())
            .unwrap_or(Ordering::Equal)
    });

    Ok(entries)
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
    module.add_function(wrap_pyfunction!(generate_tornado_chart, &module)?)?;

    // Register explain types (dependency tracing, formula explanation)
    let explain_exports = explain::register(py, &module)?;

    // Register reports types (table builder, P&L summary, credit assessment)
    let reports_exports = reports::register(py, &module)?;

    // Register variance analysis types
    let variance_exports = variance::register(py, &module)?;

    // Register scenario management types
    let scenario_exports = scenario_set::register(py, &module)?;

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
        "generate_tornado_chart",
    ];
    all_exports.extend(explain_exports);
    all_exports.extend(reports_exports);
    all_exports.extend(variance_exports);
    all_exports.extend(scenario_exports);

    Ok(all_exports)
}

fn build_tornado_entry(
    result: &SensitivityResult,
    param: &ParameterSpec,
    metric_node: &str,
    period_hint: Option<finstack_core::dates::PeriodId>,
) -> Option<PyTornadoEntry> {
    let mut min_record: Option<(f64, f64)> = None;
    let mut max_record: Option<(f64, f64)> = None;
    let mut baseline_metric = None;

    for scenario in &result.scenarios {
        let param_value = scenario.parameter_values.get(&param.node_id)?;
        let metric_value = extract_metric_value(&scenario.results, metric_node, period_hint)?;

        if approx_equal(*param_value, param.base_value) {
            baseline_metric = Some(metric_value);
        }

        match &mut min_record {
            Some((current_value, current_metric)) => {
                if *param_value < *current_value {
                    *current_value = *param_value;
                    *current_metric = metric_value;
                }
            }
            None => {
                min_record = Some((*param_value, metric_value));
            }
        }

        match &mut max_record {
            Some((current_value, current_metric)) => {
                if *param_value > *current_value {
                    *current_value = *param_value;
                    *current_metric = metric_value;
                }
            }
            None => {
                max_record = Some((*param_value, metric_value));
            }
        }
    }

    let base = baseline_metric
        .or_else(|| min_record.map(|(_, value)| value))
        .or_else(|| max_record.map(|(_, value)| value))?;

    let downside = min_record.map(|(_, value)| value - base).unwrap_or(0.0);
    let upside = max_record.map(|(_, value)| value - base).unwrap_or(0.0);

    Some(PyTornadoEntry::new_internal(
        param.node_id.clone(),
        downside,
        upside,
    ))
}

fn extract_metric_value(
    results: &finstack_statements::evaluator::Results,
    node_id: &str,
    period_hint: Option<finstack_core::dates::PeriodId>,
) -> Option<f64> {
    if let Some(period) = period_hint {
        results.get(node_id, &period)
    } else {
        results
            .nodes
            .get(node_id)
            .and_then(|periods| periods.values().next().copied())
    }
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

fn approx_equal(lhs: f64, rhs: f64) -> bool {
    let scale = lhs.abs().max(rhs.abs()).max(1.0);
    (lhs - rhs).abs() <= 1e-9 * scale
}
