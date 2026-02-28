//! Python bindings for portfolio results.

use crate::core::money::PyMoney;
use crate::portfolio::metrics::PyPortfolioMetrics;
use crate::portfolio::valuation::PyPortfolioValuation;
use finstack_portfolio::PortfolioResult;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use pyo3::Bound;

/// Complete results from portfolio evaluation.
///
/// Contains valuation, metrics, and metadata about the calculation.
///
/// Examples:
///     >>> results.total_value()
///     Money(USD, 10000000.0)
///     >>> results.get_metric("dv01")
///     125.0
#[pyclass(
    module = "finstack.portfolio",
    name = "PortfolioResult",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioResult {
    pub(crate) inner: PortfolioResult,
}

impl PyPortfolioResult {
    pub(crate) fn new(inner: PortfolioResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioResult {
    #[new]
    #[pyo3(signature = (valuation, metrics, meta))]
    #[pyo3(text_signature = "(valuation, metrics, meta)")]
    /// Create a new portfolio results instance.
    ///
    /// Args:
    ///     valuation: Portfolio valuation component.
    ///     metrics: Portfolio metrics component.
    ///     meta: Metadata describing calculation context.
    ///
    /// Returns:
    ///     PortfolioResult: New results instance.
    fn new_py(
        valuation: &Bound<'_, PyAny>,
        metrics: &Bound<'_, PyAny>,
        meta: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let val_inner = valuation
            .extract::<PyRef<PyPortfolioValuation>>()?
            .inner
            .clone();
        let metrics_inner = metrics
            .extract::<PyRef<PyPortfolioMetrics>>()?
            .inner
            .clone();
        let meta_inner: finstack_core::config::ResultsMeta = pythonize::depythonize(meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;

        Ok(Self::new(PortfolioResult::new(
            val_inner,
            metrics_inner,
            meta_inner,
        )))
    }

    #[pyo3(text_signature = "($self)")]
    /// Get the total portfolio value.
    ///
    /// Returns:
    ///     Money: Total portfolio value in base currency.
    ///
    /// Examples:
    ///     >>> results.total_value()
    ///     Money(USD, 10000000.0)
    fn total_value(&self) -> PyMoney {
        PyMoney::new(*self.inner.total_value())
    }

    #[pyo3(text_signature = "($self, metric_id)")]
    /// Get a specific aggregated metric.
    ///
    /// Args:
    ///     metric_id: Identifier of the metric to retrieve.
    ///
    /// Returns:
    ///     float or None: Metric value if found.
    ///
    /// Examples:
    ///     >>> results.get_metric("dv01")
    ///     125.0
    fn get_metric(&self, metric_id: &str) -> Option<f64> {
        self.inner.get_metric(metric_id)
    }

    #[getter]
    /// Get the portfolio valuation results.
    fn valuation(&self) -> PyPortfolioValuation {
        PyPortfolioValuation::new(self.inner.valuation.clone())
    }

    #[getter]
    /// Get the aggregated metrics.
    fn metrics(&self) -> PyPortfolioMetrics {
        PyPortfolioMetrics::new(self.inner.metrics.clone())
    }

    #[getter]
    /// Get metadata about the calculation.
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let bound = pythonize::pythonize(py, &self.inner.meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;
        Ok(bound.unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioResult(total={}, positions={}, metrics={})",
            self.inner.valuation.total_base_ccy,
            self.inner.valuation.position_values.len(),
            self.inner.metrics.aggregated.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Portfolio Results: {} total, {} metrics",
            self.inner.valuation.total_base_ccy,
            self.inner.metrics.aggregated.len()
        )
    }
}

/// Register results module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyPortfolioResult>()?;

    Ok(vec!["PortfolioResult".to_string()])
}
