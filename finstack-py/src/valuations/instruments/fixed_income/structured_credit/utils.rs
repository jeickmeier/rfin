//! Python bindings for structured credit utility functions.
//!
//! Exposes rate conversion functions (CPR/SMM, CDR/MDR, PSA) and
//! waterfall validation helpers that accept JSON input.

use finstack_valuations::instruments::fixed_income::structured_credit::{
    cdr_to_mdr as rust_cdr_to_mdr, cpr_to_smm as rust_cpr_to_smm, mdr_to_cdr as rust_mdr_to_cdr,
    psa_to_cpr as rust_psa_to_cpr, smm_to_cpr as rust_smm_to_cpr,
};
use finstack_valuations::instruments::fixed_income::structured_credit::{
    get_validation_errors as rust_get_validation_errors, Waterfall,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

// ============================================================================
// SHARED HELPERS
// ============================================================================

/// Serialize a Rust value to a Python dict using `pythonize`.
///
/// Preferred over the `json.loads` round-trip because it converts
/// directly from `serde_json::Value` to Python objects without
/// an extra Python module import.
pub(crate) fn to_dict_via_serde<T: serde::Serialize>(
    py: Python<'_>,
    value: &T,
) -> PyResult<Py<PyAny>> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
    pythonize::pythonize(py, &json_value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
}

/// Convert a Python `dict` to a JSON string via `json.dumps`.
pub(crate) fn dict_to_json(dict: &Bound<'_, PyDict>) -> PyResult<String> {
    let py = dict.py();
    let json = PyModule::import(py, "json")?
        .call_method1("dumps", (dict,))?
        .extract::<String>()
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize dict: {e}")))?;
    Ok(json)
}

/// Accept either a JSON string or a Python dict and return a JSON string.
pub(crate) fn value_to_json(value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = value.extract::<String>() {
        return Ok(s);
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        return dict_to_json(dict);
    }
    Err(PyValueError::new_err("Expected JSON string or dict"))
}

/// Deserialize a Python dict or JSON string into a Rust type.
pub(crate) fn from_dict_json<T: serde::de::DeserializeOwned>(
    data: &Bound<'_, PyAny>,
) -> PyResult<T> {
    let json_str = value_to_json(data)?;
    serde_json::from_str(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Deserialization failed: {e}")))
}

// ============================================================================
// RATE CONVERSIONS
// ============================================================================

/// Convert annual CPR to monthly SMM.
///
/// Args:
///     cpr: Annual constant prepayment rate (decimal, e.g. 0.06 for 6%).
///
/// Returns:
///     Monthly single mortality rate.
#[pyfunction]
fn cpr_to_smm(cpr: f64) -> f64 {
    rust_cpr_to_smm(cpr)
}

/// Convert monthly SMM to annual CPR.
///
/// Args:
///     smm: Monthly single mortality rate (decimal).
///
/// Returns:
///     Annual constant prepayment rate.
#[pyfunction]
fn smm_to_cpr(smm: f64) -> f64 {
    rust_smm_to_cpr(smm)
}

/// Convert annual CDR to monthly MDR.
///
/// Args:
///     cdr: Annual constant default rate (decimal, e.g. 0.02 for 2%).
///
/// Returns:
///     Monthly default rate.
#[pyfunction]
fn cdr_to_mdr(cdr: f64) -> f64 {
    rust_cdr_to_mdr(cdr)
}

/// Convert monthly MDR to annual CDR.
///
/// Args:
///     mdr: Monthly default rate (decimal).
///
/// Returns:
///     Annual constant default rate.
#[pyfunction]
fn mdr_to_cdr(mdr: f64) -> f64 {
    rust_mdr_to_cdr(mdr)
}

/// Convert PSA speed and month to annual CPR.
///
/// The PSA model ramps CPR linearly from 0% to 6% over the first 30 months,
/// then holds at 6%. PSA speed is a multiplier (e.g. 1.5 = 150% PSA).
///
/// Args:
///     psa_speed: PSA speed multiplier (e.g. 1.0 for 100% PSA).
///     month: Loan age in months.
///
/// Returns:
///     Annual CPR at the given month and speed.
#[pyfunction]
fn psa_to_cpr(psa_speed: f64, month: u32) -> f64 {
    rust_psa_to_cpr(psa_speed, month)
}

// ============================================================================
// WATERFALL VALIDATION
// ============================================================================

/// Validate a waterfall specification from JSON.
///
/// Deserializes a ``Waterfall`` from JSON, runs tier-level validation,
/// and returns a list of human-readable error strings (empty if valid).
///
/// Args:
///     waterfall_json: JSON string describing the waterfall.
///
/// Returns:
///     List of validation error descriptions; empty if the waterfall is valid.
///
/// Raises:
///     ValueError: If the JSON cannot be deserialized into a Waterfall.
#[pyfunction]
fn validate_waterfall(waterfall_json: &str) -> PyResult<Vec<String>> {
    let waterfall: Waterfall = serde_json::from_str(waterfall_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid waterfall JSON: {e}")))?;

    let errors = rust_get_validation_errors(&waterfall.tiers, &[], &[]);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

/// Check whether a waterfall specification (as JSON) is valid.
///
/// Args:
///     waterfall_json: JSON string describing the waterfall.
///
/// Returns:
///     ``True`` if the waterfall passes validation.
///
/// Raises:
///     ValueError: If the JSON cannot be deserialized.
#[pyfunction]
fn is_valid_waterfall(waterfall_json: &str) -> PyResult<bool> {
    let waterfall: Waterfall = serde_json::from_str(waterfall_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid waterfall JSON: {e}")))?;

    let errors = rust_get_validation_errors(&waterfall.tiers, &[], &[]);
    Ok(errors.is_empty())
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(cpr_to_smm, module)?)?;
    module.add_function(wrap_pyfunction!(smm_to_cpr, module)?)?;
    module.add_function(wrap_pyfunction!(cdr_to_mdr, module)?)?;
    module.add_function(wrap_pyfunction!(mdr_to_cdr, module)?)?;
    module.add_function(wrap_pyfunction!(psa_to_cpr, module)?)?;
    module.add_function(wrap_pyfunction!(validate_waterfall, module)?)?;
    module.add_function(wrap_pyfunction!(is_valid_waterfall, module)?)?;

    Ok(vec![
        "cpr_to_smm",
        "smm_to_cpr",
        "cdr_to_mdr",
        "mdr_to_cdr",
        "psa_to_cpr",
        "validate_waterfall",
        "is_valid_waterfall",
    ])
}
