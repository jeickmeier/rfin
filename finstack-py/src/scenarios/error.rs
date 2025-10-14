//! Error conversion utilities for scenarios Python bindings.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::PyErr;

/// Convert a finstack-scenarios error to a Python exception.
pub fn scenario_to_py(err: finstack_scenarios::Error) -> PyErr {
    match err {
        finstack_scenarios::Error::MarketDataNotFound { id } => {
            PyValueError::new_err(format!("Market data not found: {}", id))
        }
        finstack_scenarios::Error::NodeNotFound { node_id } => {
            PyValueError::new_err(format!("Statement node not found: {}", node_id))
        }
        finstack_scenarios::Error::CurveTypeMismatch { expected, actual } => {
            PyValueError::new_err(format!(
                "Curve type mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        finstack_scenarios::Error::UnsupportedOperation { operation, target } => {
            PyValueError::new_err(format!(
                "Unsupported operation {} for target {}",
                operation, target
            ))
        }
        finstack_scenarios::Error::Core(e) => PyValueError::new_err(format!("Core error: {}", e)),
        finstack_scenarios::Error::Statements(e) => {
            PyValueError::new_err(format!("Statements error: {}", e))
        }
        finstack_scenarios::Error::Validation(msg) => {
            PyValueError::new_err(format!("Validation error: {}", msg))
        }
        finstack_scenarios::Error::Internal(msg) => {
            PyRuntimeError::new_err(format!("Internal error: {}", msg))
        }
        finstack_scenarios::Error::InvalidTenor(msg) => {
            PyValueError::new_err(format!("Invalid tenor string: {}", msg))
        }
        finstack_scenarios::Error::TenorNotFound { tenor, curve_id } => {
            PyValueError::new_err(format!(
                "Tenor not found in curve: {} in {}",
                tenor, curve_id
            ))
        }
        finstack_scenarios::Error::InvalidPeriod(msg) => {
            PyValueError::new_err(format!("Invalid time period: {}", msg))
        }
        finstack_scenarios::Error::InstrumentNotFound(id) => {
            PyValueError::new_err(format!("Instrument not found: {}", id))
        }
    }
}

