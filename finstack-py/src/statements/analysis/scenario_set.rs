use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyStatementResult;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_statements_analytics::analysis::{
    ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet,
};
use indexmap::IndexMap;
use polars::prelude::DataFrame;
use pyo3::exceptions::{PyIOError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::{Bound, PyResult};
use pyo3_polars::PyDataFrame;
use pythonize::depythonize;

use super::variance::PyVarianceReport;

/// Python wrapper for [`ScenarioDefinition`].
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ScenarioDefinition",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioDefinition {
    pub(crate) inner: ScenarioDefinition,
}

#[pymethods]
impl PyScenarioDefinition {
    #[new]
    #[pyo3(signature = (parent=None, overrides=None, model_id=None))]
    /// Create a new scenario definition.
    ///
    /// Parameters
    /// ----------
    /// parent : str | None, default None
    ///     Optional parent scenario to inherit overrides from.
    /// overrides : dict[str, float] | None, default None
    ///     Map of node_id → scalar overrides applied to all periods.
    /// model_id : str | None, default None
    ///     Optional identifier of the underlying financial model.
    fn new(
        parent: Option<String>,
        overrides: Option<Bound<'_, PyAny>>,
        model_id: Option<String>,
    ) -> PyResult<Self> {
        let mut rust_overrides = IndexMap::new();

        if let Some(obj) = overrides {
            if let Ok(dict) = obj.cast::<PyDict>() {
                for (k, v) in dict {
                    rust_overrides.insert(k.extract::<String>()?, v.extract::<f64>()?);
                }
            } else {
                return Err(PyTypeError::new_err(
                    "overrides must be a mapping of str -> float",
                ));
            }
        }

        Ok(Self {
            inner: ScenarioDefinition {
                model_id,
                parent,
                overrides: rust_overrides,
            },
        })
    }

    /// Parent scenario name, if any.
    #[getter]
    fn parent(&self) -> Option<String> {
        self.inner.parent.clone()
    }

    /// Model identifier hint, if any.
    #[getter]
    fn model_id(&self) -> Option<String> {
        self.inner.model_id.clone()
    }

    /// Scalar overrides for this scenario.
    #[getter]
    fn overrides(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.overrides {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ScenarioDefinition(parent={:?}, overrides={}, model_id={:?})",
            self.inner.parent,
            self.inner.overrides.len(),
            self.inner.model_id
        )
    }
}

/// Python wrapper for [`ScenarioSet`].
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ScenarioSet",
    unsendable
)]
pub struct PyScenarioSet {
    pub(crate) inner: ScenarioSet,
}

#[pymethods]
impl PyScenarioSet {
    #[new]
    /// Create an empty scenario set.
    fn new() -> Self {
        Self {
            inner: ScenarioSet {
                scenarios: IndexMap::new(),
            },
        }
    }

    /// Construct a scenario set from a Python mapping.
    ///
    /// Parameters
    /// ----------
    /// mapping : dict[str, dict]
    ///     Mapping of scenario name → definition dict.
    ///
    /// The expected shape mirrors the design docs:
    ///
    /// .. code-block:: json
    ///
    ///     {
    ///       "base": { "model_id": "acme-2025", "overrides": {} },
    ///       "downside": {
    ///         "parent": "base",
    ///         "overrides": { "revenue_growth": -0.05, "margin": -0.02 }
    ///       }
    ///     }
    #[staticmethod]
    #[pyo3(signature = (mapping))]
    fn from_mapping(mapping: Bound<'_, PyAny>) -> PyResult<Self> {
        let value: serde_json::Value = depythonize(&mapping).map_err(|err| {
            PyValueError::new_err(format!("Invalid mapping for ScenarioSet: {err}"))
        })?;

        let scenario_map_value = match &value {
            serde_json::Value::Object(obj) if obj.len() == 1 => obj
                .get("scenario_set")
                .cloned()
                .unwrap_or_else(|| value.clone()),
            _ => value.clone(),
        };

        let scenarios: IndexMap<String, ScenarioDefinition> =
            serde_json::from_value(scenario_map_value).map_err(|err| {
                PyValueError::new_err(format!("Invalid scenario mapping: {}", err))
            })?;

        Ok(Self {
            inner: ScenarioSet { scenarios },
        })
    }

    /// Load a scenario set from a JSON file.
    ///
    /// The JSON can either be a raw mapping of name → definition, or an
    /// object with a top-level `"scenario_set"` key.
    #[staticmethod]
    #[pyo3(signature = (path))]
    fn from_json(path: &str) -> PyResult<Self> {
        let contents = std::fs::read_to_string(path).map_err(|err| {
            PyIOError::new_err(format!("Failed to read scenario file '{}': {}", path, err))
        })?;

        let value: serde_json::Value = serde_json::from_str(&contents).map_err(|err| {
            PyValueError::new_err(format!("Failed to parse JSON from '{}': {}", path, err))
        })?;

        let scenario_map_value = match &value {
            serde_json::Value::Object(obj) if obj.len() == 1 => obj
                .get("scenario_set")
                .cloned()
                .unwrap_or_else(|| value.clone()),
            _ => value.clone(),
        };

        let scenarios: IndexMap<String, ScenarioDefinition> =
            serde_json::from_value(scenario_map_value).map_err(|err| {
                PyValueError::new_err(format!("Invalid scenario mapping in '{}': {}", path, err))
            })?;

        Ok(Self {
            inner: ScenarioSet { scenarios },
        })
    }

    #[pyo3(signature = (name, definition))]
    /// Add or replace a scenario by name.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Scenario name.
    /// definition : ScenarioDefinition
    ///     Scenario definition instance.
    fn add_scenario(&mut self, name: String, definition: &PyScenarioDefinition) {
        self.inner.scenarios.insert(name, definition.inner.clone());
    }

    #[pyo3(signature = (name))]
    /// Remove a scenario by name, returning True if it existed.
    fn remove_scenario(&mut self, name: &str) -> bool {
        self.inner.scenarios.shift_remove(name).is_some()
    }

    /// Get the list of scenario names in insertion order.
    #[getter]
    fn scenario_names(&self) -> Vec<String> {
        self.inner.scenarios.keys().cloned().collect()
    }

    #[pyo3(signature = (base_model))]
    /// Evaluate all scenarios using a base financial model.
    ///
    /// Parameters
    /// ----------
    /// base_model : FinancialModelSpec
    ///     Base model to clone and override per scenario.
    ///
    /// Returns
    /// -------
    /// ScenarioResults
    ///     Evaluated results for all scenarios.
    fn evaluate_all(
        &self,
        py: Python<'_>,
        base_model: &PyFinancialModelSpec,
    ) -> PyResult<PyScenarioResults> {
        let inner = py.detach(|| {
            self.inner
                .evaluate_all(&base_model.inner)
                .map_err(stmt_to_py)
        })?;

        Ok(PyScenarioResults { inner })
    }

    #[pyo3(signature = (results, baseline, comparison, metrics, periods))]
    /// Compute a variance-style diff between two scenarios.
    ///
    /// Parameters
    /// ----------
    /// results : ScenarioResults
    ///     Evaluated results for all scenarios (from ``evaluate_all``).
    /// baseline : str
    ///     Baseline scenario name.
    /// comparison : str
    ///     Comparison scenario name.
    /// metrics : list[str]
    ///     Node identifiers to compare.
    /// periods : list[PeriodId]
    ///     Periods to include in the variance report.
    ///
    /// Returns
    /// -------
    /// ScenarioDiff
    ///     Diff wrapper exposing a :class:`VarianceReport`.
    fn diff(
        &self,
        py: Python<'_>,
        results: &PyScenarioResults,
        baseline: &str,
        comparison: &str,
        metrics: Vec<String>,
        periods: Vec<crate::core::dates::periods::PyPeriodId>,
    ) -> PyResult<PyScenarioDiff> {
        let period_ids: Vec<PeriodId> = periods.into_iter().map(|p| p.inner).collect();

        let diff = py.detach(|| {
            self.inner
                .diff(&results.inner, baseline, comparison, &metrics, &period_ids)
                .map_err(stmt_to_py)
        })?;

        Ok(PyScenarioDiff { inner: diff })
    }

    #[pyo3(signature = (scenario))]
    /// Return the lineage for a scenario (from root ancestor to the given name).
    fn trace(&self, scenario: &str) -> PyResult<Vec<String>> {
        self.inner.trace(scenario).map_err(stmt_to_py)
    }

    fn __repr__(&self) -> String {
        format!("ScenarioSet(scenarios={})", self.inner.scenarios.len())
    }
}

/// Python wrapper for [`ScenarioResults`].
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ScenarioResults",
    frozen
)]
pub struct PyScenarioResults {
    pub(crate) inner: ScenarioResults,
}

#[pymethods]
impl PyScenarioResults {
    /// Scenario names in insertion order.
    #[getter]
    fn scenario_names(&self) -> Vec<String> {
        self.inner.scenarios.keys().cloned().collect()
    }

    /// Number of scenarios.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[pyo3(signature = (name))]
    /// Get the `StatementResult` object for a given scenario.
    fn get(&self, name: &str) -> PyResult<PyStatementResult> {
        let results = self.inner.scenarios.get(name).ok_or_else(|| {
            PyValueError::new_err(format!("Scenario '{}' not found in ScenarioResults", name))
        })?;

        Ok(PyStatementResult::new(results.clone()))
    }

    #[pyo3(signature = (metrics))]
    /// Export a wide comparison table as a Polars DataFrame.
    ///
    /// Parameters
    /// ----------
    /// metrics : list[str]
    ///     Node identifiers to include.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     Comparison table with one column per scenario and percentage
    ///     deltas vs the baseline scenario.
    fn to_comparison_df(&self, py: Python<'_>, metrics: Vec<String>) -> PyResult<PyDataFrame> {
        let metric_refs: Vec<&str> = metrics.iter().map(|s| s.as_str()).collect();
        let df: DataFrame = py.detach(|| {
            self.inner
                .to_comparison_df(&metric_refs)
                .map_err(stmt_to_py)
        })?;

        Ok(PyDataFrame(df))
    }

    fn __repr__(&self) -> String {
        format!("ScenarioResults(scenarios={})", self.inner.len())
    }
}

/// Python wrapper for [`ScenarioDiff`].
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ScenarioDiff",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioDiff {
    pub(crate) inner: ScenarioDiff,
}

#[pymethods]
impl PyScenarioDiff {
    /// Baseline scenario name.
    #[getter]
    fn baseline(&self) -> &str {
        &self.inner.baseline
    }

    /// Comparison scenario name.
    #[getter]
    fn comparison(&self) -> &str {
        &self.inner.comparison
    }

    /// Underlying variance report.
    #[getter]
    fn variance(&self) -> PyVarianceReport {
        PyVarianceReport {
            inner: self.inner.variance.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ScenarioDiff(baseline='{}', comparison='{}')",
            self.inner.baseline, self.inner.comparison
        )
    }
}

/// Register scenario management types with the analysis module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyScenarioDefinition>()?;
    parent.add_class::<PyScenarioSet>()?;
    parent.add_class::<PyScenarioResults>()?;
    parent.add_class::<PyScenarioDiff>()?;

    Ok(vec![
        "ScenarioDefinition",
        "ScenarioSet",
        "ScenarioResults",
        "ScenarioDiff",
    ])
}
