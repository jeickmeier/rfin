//! Error handling for portfolio bindings.

use finstack_portfolio::{
    optimization::ConstraintValidationError as PortfolioConstraintValidationError, Error,
};
use pyo3::PyErr;

/// Convert a portfolio error into a Python exception.
pub(crate) fn portfolio_to_py(err: Error) -> PyErr {
    match err {
        Error::UnknownEntity {
            position_id,
            entity_id,
        } => crate::errors::ConfigurationError::new_err(format!(
            "Position '{}' references unknown entity '{}'",
            position_id, entity_id
        )),
        Error::ValidationFailed(msg) => {
            crate::errors::ValidationError::new_err(format!("Portfolio validation failed: {}", msg))
        }
        Error::FxConversionFailed { from, to } => crate::errors::ComputationError::new_err(
            format!("FX conversion failed: {} to {}", from, to),
        ),
        Error::ValuationError {
            position_id,
            message,
        } => crate::errors::PricingError::new_err(format!(
            "Valuation error for position '{}': {}",
            position_id, message
        )),
        Error::ScenarioError(msg) => crate::errors::ComputationError::new_err(format!(
            "Scenario application failed: {}",
            msg
        )),
        Error::MissingMarketData(msg) => {
            crate::errors::ConfigurationError::new_err(format!("Missing market data: {}", msg))
        }
        Error::InvalidInput(msg) => {
            crate::errors::ParameterError::new_err(format!("Invalid input: {}", msg))
        }
        Error::Core(err) => crate::errors::core_to_py(err),
        // Handle unknown variants due to #[non_exhaustive]
        _ => crate::errors::FinstackError::new_err(format!("Portfolio error: {}", err)),
    }
}

/// Convert a constraint validation error into a typed Python exception.
pub(crate) fn constraint_validation_to_py(err: PortfolioConstraintValidationError) -> PyErr {
    crate::errors::ConstraintValidationError::new_err(err.to_string())
}
