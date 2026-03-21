//! Python bindings for JSON-Schema helpers.

use crate::errors::core_to_py;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Return the JSON Schema for the Bond instrument configuration.
///
/// Returns:
///     dict: JSON Schema document (draft-07) describing the Bond type.
///
/// Raises:
///     FinstackError: If the embedded schema JSON is malformed.
#[pyfunction]
fn bond_schema(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let schema = finstack_valuations::schema::bond_schema().map_err(core_to_py)?;
    pythonize::pythonize(py, schema)
        .map(|obj| obj.unbind())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Return the JSON Schema for the instrument envelope or a specific instrument type.
///
/// Parameters:
///     instrument_type: Optional canonical instrument discriminator. When omitted,
///         returns the versioned instrument envelope schema.
///
/// Returns:
///     dict: JSON Schema document (draft-07) describing the requested schema.
///
/// Raises:
///     FinstackError: If the embedded schema JSON is malformed.
#[pyfunction(signature = (instrument_type=None))]
fn instrument_schema(py: Python<'_>, instrument_type: Option<&str>) -> PyResult<Py<PyAny>> {
    let schema = match instrument_type {
        Some(instrument_type) => {
            finstack_valuations::schema::instrument_schema(instrument_type).map_err(core_to_py)?
        }
        None => finstack_valuations::schema::instrument_envelope_schema()
            .map_err(core_to_py)?
            .clone(),
    };
    pythonize::pythonize(py, &schema)
        .map(|obj| obj.unbind())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Return the canonical instrument discriminators supported by the envelope schema.
#[pyfunction]
fn instrument_types() -> PyResult<Vec<String>> {
    finstack_valuations::schema::instrument_types().map_err(core_to_py)
}

/// Return the JSON Schema for the ValuationResult envelope.
///
/// Returns:
///     dict: JSON Schema document (draft-07) describing ValuationResult.
///
/// Raises:
///     FinstackError: If the embedded schema JSON is malformed.
#[pyfunction]
fn valuation_result_schema(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let schema = finstack_valuations::schema::valuation_result_schema().map_err(core_to_py)?;
    pythonize::pythonize(py, schema)
        .map(|obj| obj.unbind())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "schema")?;
    module.setattr(
        "__doc__",
        "JSON Schema helpers for Finstack instrument and result types.",
    )?;

    module.add_function(wrap_pyfunction!(bond_schema, &module)?)?;
    module.add_function(wrap_pyfunction!(instrument_schema, &module)?)?;
    module.add_function(wrap_pyfunction!(instrument_types, &module)?)?;
    module.add_function(wrap_pyfunction!(valuation_result_schema, &module)?)?;

    let exports = vec![
        "bond_schema",
        "instrument_schema",
        "instrument_types",
        "valuation_result_schema",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
