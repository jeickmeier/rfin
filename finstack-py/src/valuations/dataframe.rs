//! Python bindings for DataFrame export from valuation results.
//!
//! Provides to_polars(), to_pandas(), and to_parquet() methods for
//! batch export of valuation results.

use finstack_valuations::results::dataframe::results_to_rows;
use finstack_valuations::results::ValuationResult;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use pythonize::pythonize;

use crate::valuations::results::PyValuationResult;

/// Convert a list of PyValuationResult to a Polars DataFrame.
///
/// # Arguments
///
/// * `results` - List of valuation results to convert
///
/// # Returns
///
/// Polars DataFrame with columns: instrument_id, as_of_date, pv, currency,
/// dv01 (optional), convexity (optional), duration (optional), ytm (optional)
///
/// # Example (Python)
///
/// ```python
/// import polars as pl
/// from finstack.valuations import results_to_polars
///
/// df = results_to_polars([result1, result2, result3])
/// print(df.schema)
/// # {
/// #   'instrument_id': Utf8,
/// #   'as_of_date': Utf8,
/// #   'pv': Float64,
/// #   'currency': Utf8,
/// #   'dv01': Float64,
/// #   ...
/// # }
/// ```
#[pyfunction]
#[pyo3(name = "results_to_polars")]
pub fn py_results_to_polars(py: Python<'_>, results: Vec<PyValuationResult>) -> PyResult<Py<PyAny>> {
    // Extract inner Rust ValuationResults
    let rust_results: Vec<ValuationResult> = results.into_iter().map(|r| r.inner).collect();

    // Convert to rows
    let rows = results_to_rows(&rust_results);

    // Convert rows to Python dicts
    let py_rows: Vec<Py<PyAny>> = rows
        .iter()
        .map(|row| {
            pythonize(py, row)
                .map(|bound| bound.unbind())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Call pl.DataFrame(rows)
    let polars = py.import("polars")?;
    let df = polars.call_method1("DataFrame", (py_rows,))?;

    Ok(df.into())
}

/// Convert a list of PyValuationResult to a Pandas DataFrame.
///
/// # Arguments
///
/// * `results` - List of valuation results to convert
///
/// # Returns
///
/// Pandas DataFrame (via Polars conversion)
///
/// # Example (Python)
///
/// ```python
/// import pandas as pd
/// from finstack.valuations import results_to_pandas
///
/// df = results_to_pandas([result1, result2, result3])
/// print(df.dtypes)
/// ```
#[pyfunction]
#[pyo3(name = "results_to_pandas")]
pub fn py_results_to_pandas(py: Python<'_>, results: Vec<PyValuationResult>) -> PyResult<Py<PyAny>> {
    // Convert to Polars first
    let polars_df = py_results_to_polars(py, results)?;

    // Call df.to_pandas()
    polars_df.call_method0(py, "to_pandas")
}

/// Write a list of PyValuationResult to a Parquet file.
///
/// # Arguments
///
/// * `results` - List of valuation results to convert
/// * `path` - Output file path
///
/// # Example (Python)
///
/// ```python
/// from finstack.valuations import results_to_parquet
///
/// results_to_parquet([result1, result2, result3], "valuations.parquet")
/// ```
#[pyfunction]
#[pyo3(name = "results_to_parquet")]
pub fn py_results_to_parquet(
    py: Python<'_>,
    results: Vec<PyValuationResult>,
    path: &str,
) -> PyResult<()> {
    // Convert to Polars first
    let polars_df = py_results_to_polars(py, results)?;

    // Call df.write_parquet(path)
    polars_df.call_method1(py, "write_parquet", (path,))?;

    Ok(())
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "dataframe")?;
    module.setattr(
        "__doc__",
        "DataFrame export utilities for valuation results (Polars, Pandas, Parquet).",
    )?;
    module.add_function(wrap_pyfunction!(py_results_to_polars, &module)?)?;
    module.add_function(wrap_pyfunction!(py_results_to_pandas, &module)?)?;
    module.add_function(wrap_pyfunction!(py_results_to_parquet, &module)?)?;
    let exports = vec![
        "results_to_polars",
        "results_to_pandas",
        "results_to_parquet",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
