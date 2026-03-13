//! Error conversion for statements crate.

use pyo3::PyErr;

use crate::errors::{
    core_to_py, CalibrationError, ComputationError, ConfigurationError, CurrencyMismatchError,
    DateError, FinstackError, InternalError, ParameterError, ValidationError,
};

/// Convert a `finstack-statements` error to an appropriate Python exception.
///
/// Maps each statement variant to the closest exception in the Finstack hierarchy
/// instead of collapsing everything to `RuntimeError`.
pub(crate) fn stmt_to_py(err: finstack_statements::Error) -> PyErr {
    match err {
        finstack_statements::Error::Core(e) => core_to_py(e),
        finstack_statements::Error::CurrencyMismatch(expected, actual) => {
            CurrencyMismatchError::new_err(format!(
                "Currency mismatch: expected {expected}, got {actual}"
            ))
        }
        finstack_statements::Error::NodeNotFound(id) => {
            ValidationError::new_err(format!("Statement node not found: {id}"))
        }
        finstack_statements::Error::CircularDependency(path) => ValidationError::new_err(format!(
            "Circular dependency detected: {}",
            path.join(" -> ")
        )),
        finstack_statements::Error::Build(msg) => {
            ParameterError::new_err(format!("Model build error: {msg}"))
        }
        finstack_statements::Error::FormulaParse(msg) => {
            ParameterError::new_err(format!("Formula parse error: {msg}"))
        }
        finstack_statements::Error::InvalidInput(msg) => {
            ParameterError::new_err(format!("Invalid input: {msg}"))
        }
        finstack_statements::Error::BuilderError(msg) => {
            ParameterError::new_err(format!("Builder error: {msg}"))
        }
        finstack_statements::Error::IndexError(msg) => {
            ParameterError::new_err(format!("Index error: {msg}"))
        }
        finstack_statements::Error::CapitalStructure(msg) => {
            ParameterError::new_err(format!("Capital structure error: {msg}"))
        }
        finstack_statements::Error::Eval(msg) => {
            ComputationError::new_err(format!("Evaluation error: {msg}"))
        }
        finstack_statements::Error::Forecast(msg) => {
            CalibrationError::new_err(format!("Forecast error: {msg}"))
        }
        finstack_statements::Error::Period(msg) => {
            DateError::new_err(format!("Period error: {msg}"))
        }
        finstack_statements::Error::MissingData(msg) => {
            ConfigurationError::new_err(format!("Missing required data: {msg}"))
        }
        finstack_statements::Error::Registry(msg) => {
            ConfigurationError::new_err(format!("Registry error: {msg}"))
        }
        finstack_statements::Error::Serde(msg) => {
            InternalError::new_err(format!("Serialization error: {msg}"))
        }
        finstack_statements::Error::Io(msg) => InternalError::new_err(format!("I/O error: {msg}")),
        _ => FinstackError::new_err(err.to_string()),
    }
}
