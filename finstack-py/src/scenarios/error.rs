//! Error conversion utilities for scenarios Python bindings.

use pyo3::PyErr;

use crate::errors::{
    map_error, ConfigurationError, DateError, FinstackError, InternalError, ParameterError,
    ValidationError,
};

/// Convert a finstack-scenarios error to a Python exception.
pub fn scenario_to_py(err: finstack_scenarios::Error) -> PyErr {
    match err {
        finstack_scenarios::Error::MarketDataNotFound { id } => {
            ConfigurationError::new_err(format!("Market data not found: {}", id))
        }
        finstack_scenarios::Error::NodeNotFound { node_id } => {
            ValidationError::new_err(format!("Statement node not found: {}", node_id))
        }
        finstack_scenarios::Error::CurveTypeMismatch { expected, actual } => {
            ParameterError::new_err(format!(
                "Curve type mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        finstack_scenarios::Error::UnsupportedOperation { operation, target } => {
            ParameterError::new_err(format!(
                "Unsupported operation {} for target {}",
                operation, target
            ))
        }
        finstack_scenarios::Error::Core(e) => map_error(e),
        finstack_scenarios::Error::Statements(e) => {
            FinstackError::new_err(format!("Statements error: {}", e))
        }
        finstack_scenarios::Error::Validation(msg) => ValidationError::new_err(msg),
        finstack_scenarios::Error::Internal(msg) => InternalError::new_err(msg),
        finstack_scenarios::Error::InvalidTenor(msg) => {
            ParameterError::new_err(format!("Invalid tenor string: {}", msg))
        }
        finstack_scenarios::Error::TenorNotFound { tenor, curve_id } => ParameterError::new_err(
            format!("Tenor not found in curve: {} in {}", tenor, curve_id),
        ),
        finstack_scenarios::Error::InvalidPeriod(msg) => {
            DateError::new_err(format!("Invalid time period: {}", msg))
        }
        finstack_scenarios::Error::InstrumentNotFound(id) => {
            ConfigurationError::new_err(format!("Instrument not found: {}", id))
        }
        other => FinstackError::new_err(format!("Scenario error: {}", other)),
    }
}
