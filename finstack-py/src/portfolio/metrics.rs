//! Python bindings for portfolio metrics.

use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::valuation::extract_portfolio_valuation;
use finstack_portfolio::metrics::{aggregate_metrics, AggregatedMetric, PortfolioMetrics};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;

/// Aggregated metric across the portfolio.
///
/// Contains portfolio-wide totals as well as breakdowns by entity.
///
/// Examples:
///     >>> metric = metrics.get_metric("dv01")
///     >>> metric.total
///     125.0
///     >>> metric.by_entity["ENTITY_A"]
///     75.0
#[pyclass(module = "finstack.portfolio", name = "AggregatedMetric")]
#[derive(Clone)]
pub struct PyAggregatedMetric {
    pub(crate) inner: AggregatedMetric,
}

impl PyAggregatedMetric {
    pub(crate) fn new(inner: AggregatedMetric) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAggregatedMetric {
    #[getter]
    /// Get the metric identifier.
    fn metric_id(&self) -> String {
        self.inner.metric_id.clone()
    }

    #[getter]
    /// Get the total value across all positions (for summable metrics).
    fn total(&self) -> f64 {
        self.inner.total
    }

    #[getter]
    /// Get aggregated values by entity.
    fn by_entity(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (id, value) in &self.inner.by_entity {
            dict.set_item(id.as_str(), value)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "AggregatedMetric(id='{}', total={}, entities={})",
            self.inner.metric_id,
            self.inner.total,
            self.inner.by_entity.len()
        )
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.inner.metric_id, self.inner.total)
    }
}

/// Complete portfolio metrics results.
///
/// Holds both aggregated metrics and per-position values.
///
/// Examples:
///     >>> metrics = aggregate_metrics(valuation)
///     >>> dv01 = metrics.get_metric("dv01")
///     >>> position_metrics = metrics.get_position_metrics("POS_1")
#[pyclass(module = "finstack.portfolio", name = "PortfolioMetrics")]
#[derive(Clone)]
pub struct PyPortfolioMetrics {
    pub(crate) inner: PortfolioMetrics,
}

impl PyPortfolioMetrics {
    pub(crate) fn new(inner: PortfolioMetrics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioMetrics {
    #[pyo3(text_signature = "($self, metric_id)")]
    /// Get an aggregated metric by identifier.
    ///
    /// Args:
    ///     metric_id: Identifier of the metric to look up.
    ///
    /// Returns:
    ///     AggregatedMetric or None: The metric if found.
    ///
    /// Examples:
    ///     >>> metric = metrics.get_metric("dv01")
    fn get_metric(&self, metric_id: &str) -> Option<PyAggregatedMetric> {
        self.inner
            .get_metric(metric_id)
            .map(|m| PyAggregatedMetric::new(m.clone()))
    }

    #[pyo3(text_signature = "($self, position_id)")]
    /// Get metrics for a specific position.
    ///
    /// Args:
    ///     position_id: Identifier of the position to query.
    ///
    /// Returns:
    ///     dict[str, float] or None: Mapping of metric IDs to values for the position.
    ///
    /// Examples:
    ///     >>> position_metrics = metrics.get_position_metrics("POS_1")
    ///     >>> position_metrics["dv01"]
    ///     5.0
    fn get_position_metrics(
        &self,
        position_id: &str,
        py: Python<'_>,
    ) -> PyResult<Option<PyObject>> {
        if let Some(metrics_map) = self.inner.get_position_metrics(position_id) {
            let dict = PyDict::new(py);
            for (metric_id, value) in metrics_map {
                dict.set_item(metric_id, value)?;
            }
            Ok(Some(dict.into()))
        } else {
            Ok(None)
        }
    }

    #[pyo3(text_signature = "($self, metric_id)")]
    /// Get the total value of a specific metric across the portfolio.
    ///
    /// Args:
    ///     metric_id: Identifier of the metric.
    ///
    /// Returns:
    ///     float or None: Total metric value if found.
    ///
    /// Examples:
    ///     >>> total_dv01 = metrics.get_total("dv01")
    ///     >>> total_dv01
    ///     125.0
    fn get_total(&self, metric_id: &str) -> Option<f64> {
        self.inner.get_total(metric_id)
    }

    #[getter]
    /// Get aggregated metrics (summable only).
    fn aggregated(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (id, metric) in &self.inner.aggregated {
            dict.set_item(id, PyAggregatedMetric::new(metric.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get raw metrics by position (all metrics).
    fn by_position(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (pos_id, metrics_map) in &self.inner.by_position {
            let inner_dict = PyDict::new(py);
            for (metric_id, value) in metrics_map {
                inner_dict.set_item(metric_id, value)?;
            }
            dict.set_item(pos_id.as_str(), inner_dict)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioMetrics(aggregated={}, positions={})",
            self.inner.aggregated.len(),
            self.inner.by_position.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Portfolio Metrics: {} aggregated, {} positions",
            self.inner.aggregated.len(),
            self.inner.by_position.len()
        )
    }
}

/// Aggregate metrics from portfolio valuation.
///
/// Computes portfolio-wide metrics by summing position-level results where appropriate.
/// Only summable metrics (DV01, CS01, Theta, etc.) are aggregated.
///
/// Args:
///     valuation: Portfolio valuation results.
///
/// Returns:
///     PortfolioMetrics: Aggregated metrics results.
///
/// Raises:
///     RuntimeError: If aggregation fails.
///
/// Examples:
///     >>> from finstack.portfolio import aggregate_metrics
///     >>> metrics = aggregate_metrics(valuation)
///     >>> metrics.get_total("dv01")
///     125.0
#[pyfunction]
fn py_aggregate_metrics(valuation: &Bound<'_, PyAny>) -> PyResult<PyPortfolioMetrics> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let metrics = aggregate_metrics(&valuation_inner).map_err(portfolio_to_py)?;
    Ok(PyPortfolioMetrics::new(metrics))
}

/// Register metrics module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyAggregatedMetric>()?;
    parent.add_class::<PyPortfolioMetrics>()?;

    let wrapped_fn = wrap_pyfunction!(py_aggregate_metrics, parent)?;
    parent.add("aggregate_metrics", wrapped_fn)?;

    Ok(vec![
        "AggregatedMetric".to_string(),
        "PortfolioMetrics".to_string(),
        "aggregate_metrics".to_string(),
    ])
}
