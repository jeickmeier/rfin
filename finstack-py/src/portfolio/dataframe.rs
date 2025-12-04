//! Python bindings for portfolio DataFrame exports.

use crate::portfolio::metrics::PyPortfolioMetrics;
use crate::portfolio::valuation::extract_portfolio_valuation;
use finstack_portfolio::dataframe::{
    aggregated_metrics_to_dataframe, entities_to_dataframe, metrics_to_dataframe,
    positions_to_dataframe,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use pyo3_polars::PyDataFrame;

/// Export position values to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "positions_to_polars")]
fn py_positions_to_polars(valuation: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let df = positions_to_dataframe(&valuation_inner).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("positions_to_polars failed: {}", e))
    })?;
    Ok(PyDataFrame(df))
}

/// Export entity-level aggregates to a Polars DataFrame.
#[pyfunction]
#[pyo3(name = "entities_to_polars")]
fn py_entities_to_polars(valuation: &Bound<'_, PyAny>) -> PyResult<PyDataFrame> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let df = entities_to_dataframe(&valuation_inner).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("entities_to_polars failed: {}", e))
    })?;
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
    let df = metrics_to_dataframe(&metrics_inner).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("metrics_to_polars failed: {}", e))
    })?;
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
    let df = aggregated_metrics_to_dataframe(&metrics_inner).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!(
            "aggregated_metrics_to_polars failed: {}",
            e
        ))
    })?;
    Ok(PyDataFrame(df))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    let submod = PyModule::new(py, "dataframe")?;
    submod.setattr(
        "__doc__",
        "DataFrame export utilities for portfolio results (Polars).",
    )?;

    submod.add_function(wrap_pyfunction!(py_positions_to_polars, &submod)?)?;
    submod.add_function(wrap_pyfunction!(py_entities_to_polars, &submod)?)?;
    submod.add_function(wrap_pyfunction!(py_metrics_to_polars, &submod)?)?;
    submod.add_function(wrap_pyfunction!(py_aggregated_metrics_to_polars, &submod)?)?;

    parent.add_submodule(&submod)?;
    parent.setattr("dataframe", &submod)?;

    let exports = vec![
        "positions_to_polars".to_string(),
        "entities_to_polars".to_string(),
        "metrics_to_polars".to_string(),
        "aggregated_metrics_to_polars".to_string(),
    ];
    submod.setattr("__all__", PyList::new(py, &exports)?)?;

    // Promote functions to parent for convenience
    for name in &exports {
        let func = submod.getattr(name)?;
        parent.setattr(name, &func)?;
    }

    Ok(exports)
}
