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

/// Validate an instrument JSON dict against the envelope schema.
///
/// Parameters:
///     instrument_json: dict representing an instrument envelope
///         (e.g., ``{"schema": "finstack.instrument/1", "instrument": {"type": "bond", "spec": {...}}}``)
///
/// Returns:
///     None if valid.
///
/// Raises:
///     ValidationError: If the JSON does not conform to the schema, with details.
///
/// Example:
///     >>> from finstack.valuations.schema import validate_instrument_json
///     >>> validate_instrument_json({
///     ...     "schema": "finstack.instrument/1",
///     ...     "instrument": {"type": "bond", "spec": {}}
///     ... })
#[pyfunction]
fn validate_instrument_json(py: Python<'_>, instrument_json: Py<PyAny>) -> PyResult<()> {
    let json_value: serde_json::Value =
        pythonize::depythonize(instrument_json.bind(py)).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to convert dict to JSON: {e}"))
        })?;
    finstack_valuations::schema::validate_instrument_json(&json_value).map_err(core_to_py)
}

/// Validate an instrument JSON dict against a specific instrument type's schema.
///
/// Parameters:
///     instrument_type: Canonical instrument type (e.g., "bond", "interest_rate_swap")
///     instrument_json: dict representing the instrument envelope
///
/// Returns:
///     None if valid.
///
/// Raises:
///     ValidationError: If the JSON does not conform to the schema, with details.
#[pyfunction]
fn validate_instrument_type_json(
    py: Python<'_>,
    instrument_type: &str,
    instrument_json: Py<PyAny>,
) -> PyResult<()> {
    let json_value: serde_json::Value =
        pythonize::depythonize(instrument_json.bind(py)).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to convert dict to JSON: {e}"))
        })?;
    finstack_valuations::schema::validate_instrument_type_json(instrument_type, &json_value)
        .map_err(core_to_py)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "schema")?;
    module.setattr(
        "__doc__",
        "JSON Schema helpers for Finstack instrument and result types.\n\n\
         Provides schema access and validation for instrument JSON payloads.",
    )?;

    module.add_function(wrap_pyfunction!(bond_schema, &module)?)?;
    module.add_function(wrap_pyfunction!(instrument_schema, &module)?)?;
    module.add_function(wrap_pyfunction!(instrument_types, &module)?)?;
    module.add_function(wrap_pyfunction!(valuation_result_schema, &module)?)?;
    module.add_function(wrap_pyfunction!(validate_instrument_json, &module)?)?;
    module.add_function(wrap_pyfunction!(validate_instrument_type_json, &module)?)?;

    let exports = vec![
        "bond_schema",
        "instrument_schema",
        "instrument_types",
        "valuation_result_schema",
        "validate_instrument_json",
        "validate_instrument_type_json",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
