use finstack_core::error::{Error, InputError};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::PyErr;

pub(crate) fn core_to_py(err: Error) -> PyErr {
    match err {
        Error::Input(input) => input_to_py(input),
        Error::InterpOutOfBounds => PyValueError::new_err("Interpolation input out of bounds"),
        Error::CurrencyMismatch { expected, actual } => PyValueError::new_err(format!(
            "Currency mismatch: expected {expected}, got {actual}"
        )),
        Error::Calibration { message, category } => {
            PyRuntimeError::new_err(format!("Calibration error ({category}): {message}"))
        }
        Error::Validation(msg) => PyValueError::new_err(msg),
        Error::Internal => PyRuntimeError::new_err("Internal finstack error"),
        _ => PyRuntimeError::new_err(err.to_string()),
    }
}

pub(crate) fn input_to_py(err: InputError) -> PyErr {
    match err {
        InputError::NotFound { id } => PyKeyError::new_err(id),
        InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        } => PyValueError::new_err(format!(
            "Business day adjustment failed for {date} using {convention:?} within {max_days} days"
        )),
        other => PyValueError::new_err(other.to_string()),
    }
}

pub(crate) fn unknown_currency(code: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown currency code: {code}"))
}

pub(crate) fn unknown_rounding_mode(name: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown rounding mode: {name}"))
}

pub(crate) fn unknown_business_day_convention(name: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown business day convention: {name}"))
}

pub(crate) fn calendar_not_found(id: &str) -> PyErr {
    PyKeyError::new_err(format!("Calendar not found: {id}"))
}
