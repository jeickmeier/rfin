//! Shared helpers for constructing pandas DataFrames from Rust data.

use crate::bindings::core::dates::utils::date_to_py;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Build a `pd.DataFrame` from a dict of column data with an optional index.
///
/// `columns` is a pre-populated `PyDict` mapping column names to list-like values.
/// If `index` is `Some`, it is passed as the `index=` keyword argument.
pub fn dict_to_dataframe<'py>(
    py: Python<'py>,
    columns: &Bound<'py, PyDict>,
    index: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let pd = py.import("pandas")?;
    let kwargs = PyDict::new(py);
    if let Some(idx) = index {
        kwargs.set_item("index", idx)?;
    }
    pd.call_method("DataFrame", (columns,), Some(&kwargs))
}

/// Convert a slice of `time::Date` into a Python list suitable for a DataFrame index.
pub fn dates_to_pylist<'py>(
    py: Python<'py>,
    dates: &[time::Date],
) -> PyResult<Vec<Bound<'py, PyAny>>> {
    dates.iter().map(|&d| date_to_py(py, d)).collect()
}
