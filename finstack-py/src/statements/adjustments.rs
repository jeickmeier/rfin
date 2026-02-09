use crate::statements::evaluator::PyStatementResult;
use finstack_core::dates::PeriodId;
use finstack_core::HashMap;
use finstack_statements::adjustments::engine::NormalizationEngine as RustNormalizationEngine;
use finstack_statements::adjustments::types::{
    Adjustment as RustAdjustment, AdjustmentCap as RustAdjustmentCap,
    NormalizationConfig as RustNormalizationConfig, NormalizationResult as RustNormalizationResult,
};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use std::str::FromStr;

/// Configuration for normalizing a financial metric.
#[pyclass(name = "NormalizationConfig")]
#[derive(Clone)]
pub struct PyNormalizationConfig {
    pub inner: RustNormalizationConfig,
}

#[pymethods]
impl PyNormalizationConfig {
    #[new]
    pub fn new(target_node: String) -> Self {
        Self {
            inner: RustNormalizationConfig::new(target_node),
        }
    }

    pub fn add_adjustment(&mut self, adjustment: &PyAdjustment) {
        self.inner.adjustments.push(adjustment.inner.clone());
    }
}

/// Specification for a single adjustment.
#[pyclass(name = "Adjustment")]
#[derive(Clone)]
pub struct PyAdjustment {
    pub inner: RustAdjustment,
}

#[pymethods]
impl PyAdjustment {
    #[staticmethod]
    pub fn fixed(id: String, name: String, amounts: HashMap<String, f64>) -> PyResult<Self> {
        let mut index_map = IndexMap::new();
        for (k, v) in amounts {
            let period = PeriodId::from_str(&k)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            index_map.insert(period, v);
        }

        Ok(Self {
            inner: RustAdjustment::fixed(id, name, index_map),
        })
    }

    #[staticmethod]
    pub fn percentage(id: String, name: String, node_id: String, percentage: f64) -> Self {
        Self {
            inner: RustAdjustment::percentage(id, name, node_id, percentage),
        }
    }

    pub fn with_cap(&mut self, base_node: Option<String>, value: f64) -> Self {
        let mut new_adj = self.inner.clone();
        new_adj.cap = Some(RustAdjustmentCap { base_node, value });
        Self { inner: new_adj }
    }
}

/// Result of a normalization process.
#[pyclass(name = "NormalizationResult")]
pub struct PyNormalizationResult {
    inner: RustNormalizationResult,
}

#[pymethods]
impl PyNormalizationResult {
    #[getter]
    pub fn period(&self) -> String {
        self.inner.period.to_string()
    }

    #[getter]
    pub fn base_value(&self) -> f64 {
        self.inner.base_value
    }

    #[getter]
    pub fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    #[getter]
    pub fn adjustments(&self) -> Vec<PyAppliedAdjustment> {
        self.inner
            .adjustments
            .iter()
            .map(|a| PyAppliedAdjustment { inner: a.clone() })
            .collect()
    }
}

/// Details of an applied adjustment.
#[pyclass(name = "AppliedAdjustment")]
#[derive(Clone)]
pub struct PyAppliedAdjustment {
    inner: finstack_statements::adjustments::types::AppliedAdjustment,
}

#[pymethods]
impl PyAppliedAdjustment {
    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    pub fn raw_amount(&self) -> f64 {
        self.inner.raw_amount
    }

    #[getter]
    pub fn capped_amount(&self) -> f64 {
        self.inner.capped_amount
    }

    #[getter]
    pub fn is_capped(&self) -> bool {
        self.inner.is_capped
    }
}

/// Engine for calculating normalized metrics.
#[pyclass(name = "NormalizationEngine")]
pub struct PyNormalizationEngine;

#[pymethods]
impl PyNormalizationEngine {
    #[staticmethod]
    pub fn normalize(
        py: Python<'_>,
        results: Py<PyAny>,
        config: Py<PyAny>,
    ) -> PyResult<Vec<PyNormalizationResult>> {
        let results_bound = results.bind(py);
        let config_bound = config.bind(py);

        let results: &Bound<'_, PyStatementResult> = results_bound.downcast()?;
        let config: &Bound<'_, PyNormalizationConfig> = config_bound.downcast()?;

        let results_ref = results.borrow();
        let config_ref = config.borrow();
        let res = RustNormalizationEngine::normalize(&results_ref.inner, &config_ref.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(res
            .into_iter()
            .map(|r| PyNormalizationResult { inner: r })
            .collect())
    }

    #[staticmethod]
    pub fn merge_into_results(
        py: Python<'_>,
        results: Py<PyAny>,
        normalization_results: Vec<PyRef<'_, PyNormalizationResult>>,
        output_node_id: String,
    ) -> PyResult<()> {
        let results_bound = results.bind(py);
        let results: &Bound<'_, PyStatementResult> = results_bound.downcast()?;

        let rust_results: Vec<RustNormalizationResult> = normalization_results
            .into_iter()
            .map(|r| r.inner.clone()) // PyRef derefs to the struct, we clone inner
            .collect();

        let mut results_mut = results.borrow_mut();
        RustNormalizationEngine::merge_into_results(
            &mut results_mut.inner,
            &rust_results,
            &output_node_id,
        );
        Ok(())
    }
}

pub fn register<'py>(
    py: Python<'py>,
    parent_module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let m = PyModule::new(py, "adjustments")?;
    m.add_class::<PyNormalizationConfig>()?;
    m.add_class::<PyAdjustment>()?;
    m.add_class::<PyNormalizationResult>()?;
    m.add_class::<PyAppliedAdjustment>()?;
    m.add_class::<PyNormalizationEngine>()?;
    parent_module.add_submodule(&m)?;

    Ok(vec![
        "NormalizationConfig",
        "Adjustment",
        "NormalizationResult",
        "AppliedAdjustment",
        "NormalizationEngine",
    ])
}
