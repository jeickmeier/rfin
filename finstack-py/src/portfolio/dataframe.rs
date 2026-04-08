//! Python bindings for portfolio DataFrame exports.

use crate::portfolio::metrics::PyPortfolioMetrics;
use crate::portfolio::valuation::extract_portfolio_valuation;
use finstack_portfolio::dataframe::{
    aggregated_metrics_to_dataframe, entities_to_dataframe, metrics_to_dataframe,
    positions_to_dataframe,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use pyo3_polars::PyDataFrame;

/// Export position values to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "positions_to_polars")]
fn py_positions_to_polars(valuation: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let df = positions_to_dataframe(&valuation_inner)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyDataFrame(df))
}

/// Export entity-level aggregates to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "entities_to_polars")]
fn py_entities_to_polars(valuation: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let df = entities_to_dataframe(&valuation_inner)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyDataFrame(df))
}

/// Export per-position metrics to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "metrics_to_polars")]
fn py_metrics_to_polars(metrics: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let metrics_inner = metrics
        .extract::<PyRef<PyPortfolioMetrics>>()?
        .inner
        .clone();
    let df = metrics_to_dataframe(&metrics_inner)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyDataFrame(df))
}

/// Export aggregated metrics to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "aggregated_metrics_to_polars")]
fn py_aggregated_metrics_to_polars(metrics: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let metrics_inner = metrics
        .extract::<PyRef<PyPortfolioMetrics>>()?
        .inner
        .clone();
    let df = aggregated_metrics_to_dataframe(&metrics_inner)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyDataFrame(df))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_function(wrap_pyfunction!(py_positions_to_polars, parent)?)?;
    parent.add_function(wrap_pyfunction!(py_entities_to_polars, parent)?)?;
    parent.add_function(wrap_pyfunction!(py_metrics_to_polars, parent)?)?;
    parent.add_function(wrap_pyfunction!(py_aggregated_metrics_to_polars, parent)?)?;

    Ok(vec![
        "positions_to_polars".to_string(),
        "entities_to_polars".to_string(),
        "metrics_to_polars".to_string(),
        "aggregated_metrics_to_polars".to_string(),
    ])
}
