//! Shared helpers for constructing pandas DataFrames from Rust data.

use crate::bindings::core::dates::utils::date_to_py;
use finstack_core::table::{TableColumn, TableColumnData, TableEnvelope};
use numpy::PyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

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

/// Convert a table column into a Python list suitable for pandas construction.
pub fn table_column_to_pylist<'py>(
    py: Python<'py>,
    column: &TableColumn,
) -> PyResult<Bound<'py, PyAny>> {
    let obj: Bound<'py, PyAny> = match &column.data {
        TableColumnData::String(values) => PyList::new(py, values.iter().cloned())?.into_any(),
        TableColumnData::NullableString(values) => {
            PyList::new(py, values.iter().cloned())?.into_any()
        }
        TableColumnData::Float64(values) => PyArray1::from_vec(py, values.clone()).into_any(),
        TableColumnData::NullableFloat64(values) => {
            PyList::new(py, values.iter().copied())?.into_any()
        }
        TableColumnData::UInt32(values) => PyArray1::from_vec(py, values.clone()).into_any(),
        TableColumnData::NullableUInt32(values) => {
            PyList::new(py, values.iter().copied())?.into_any()
        }
        TableColumnData::Int64(values) => PyArray1::from_vec(py, values.clone()).into_any(),
        TableColumnData::NullableInt64(values) => {
            PyList::new(py, values.iter().copied())?.into_any()
        }
    };
    Ok(obj)
}

/// Build a pandas DataFrame from every column in a table envelope.
pub fn table_to_dataframe<'py>(
    py: Python<'py>,
    table: &TableEnvelope,
) -> PyResult<Bound<'py, PyAny>> {
    let columns = PyDict::new(py);
    for column in &table.columns {
        columns.set_item(column.name.as_str(), table_column_to_pylist(py, column)?)?;
    }
    dict_to_dataframe(py, &columns, None)
}

/// Build a pandas DataFrame from a selected set of table columns.
///
/// Each tuple is `(source_column_name, pandas_column_name)`.
pub fn selected_table_to_dataframe<'py>(
    py: Python<'py>,
    table: &TableEnvelope,
    selected_columns: &[(&str, &str)],
) -> PyResult<Bound<'py, PyAny>> {
    let columns = PyDict::new(py);
    for (source, target) in selected_columns {
        let column = table.column(source).ok_or_else(|| {
            PyValueError::new_err(format!("missing required table column '{source}'"))
        })?;
        columns.set_item(*target, table_column_to_pylist(py, column)?)?;
    }
    dict_to_dataframe(py, &columns, None)
}
