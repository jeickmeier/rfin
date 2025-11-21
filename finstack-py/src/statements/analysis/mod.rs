//! Analysis module bindings for financial models.

use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyResults;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::types::SensitivityScenario;
use finstack_statements::analysis::{
    ParameterSpec, SensitivityAnalyzer, SensitivityConfig, SensitivityMode, SensitivityResult,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::Bound;

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
    fn parameter_values(&self, py: Python<'_>) -> PyObject {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.parameter_values {
            dict.set_item(key, value).ok();
        }
        dict.into()
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
        let result = py.allow_threads(|| analyzer.run(&config.inner).map_err(stmt_to_py))?;

        Ok(PySensitivityResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!("SensitivityAnalyzer(model='{}')", self.model.inner.id)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "analysis")?;
    module.setattr(
        "__doc__",
        "Sensitivity analysis for financial statement models.",
    )?;

    module.add_class::<PyParameterSpec>()?;
    module.add_class::<PySensitivityMode>()?;
    module.add_class::<PySensitivityConfig>()?;
    module.add_class::<PySensitivityScenario>()?;
    module.add_class::<PySensitivityResult>()?;
    module.add_class::<PySensitivityAnalyzer>()?;

    parent.add_submodule(&module)?;
    parent.setattr("analysis", &module)?;

    Ok(vec![
        "ParameterSpec",
        "SensitivityMode",
        "SensitivityConfig",
        "SensitivityScenario",
        "SensitivityResult",
        "SensitivityAnalyzer",
    ])
}
