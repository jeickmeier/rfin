//! Error handling for portfolio bindings.

use finstack_portfolio::Error;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::PyErr;

/// Convert a portfolio error into a Python exception.
pub(crate) fn portfolio_to_py(err: Error) -> PyErr {
    match err {
        Error::UnknownEntity {
            position_id,
            entity_id,
        } => PyValueError::new_err(format!(
            "Position '{}' references unknown entity '{}'",
            position_id, entity_id
        )),
        Error::ValidationFailed(msg) => {
            PyValueError::new_err(format!("Portfolio validation failed: {}", msg))
        }
        Error::FxConversionFailed { from, to } => {
            PyRuntimeError::new_err(format!("FX conversion failed: {} to {}", from, to))
        }
        Error::ValuationError {
            position_id,
            message,
        } => PyRuntimeError::new_err(format!(
            "Valuation error for position '{}': {}",
            position_id, message
        )),
        Error::ScenarioError(msg) => {
            PyRuntimeError::new_err(format!("Scenario application failed: {}", msg))
        }
        Error::MissingMarketData(msg) => {
            PyRuntimeError::new_err(format!("Missing market data: {}", msg))
        }
        Error::InvalidInput(msg) => PyValueError::new_err(format!("Invalid input: {}", msg)),
        Error::BuilderError(msg) => PyValueError::new_err(format!("Builder error: {}", msg)),
        Error::IndexError(msg) => PyValueError::new_err(format!("Index error: {}", msg)),
        Error::Core(err) => PyRuntimeError::new_err(format!("Core error: {}", err)),
        // Handle unknown variants due to #[non_exhaustive]
        _ => PyRuntimeError::new_err(format!("Portfolio error: {}", err)),
    }
}
