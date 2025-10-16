//! Error handling for portfolio bindings.

use finstack_portfolio::PortfolioError;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::PyErr;

/// Convert a portfolio error into a Python exception.
pub(crate) fn portfolio_to_py(err: PortfolioError) -> PyErr {
    match err {
        PortfolioError::UnknownEntity {
            position_id,
            entity_id,
        } => PyValueError::new_err(format!(
            "Position '{}' references unknown entity '{}'",
            position_id, entity_id
        )),
        PortfolioError::ValidationFailed(msg) => {
            PyValueError::new_err(format!("Portfolio validation failed: {}", msg))
        }
        PortfolioError::FxConversionFailed { from, to } => {
            PyRuntimeError::new_err(format!("FX conversion failed: {} to {}", from, to))
        }
        PortfolioError::ValuationError {
            position_id,
            message,
        } => PyRuntimeError::new_err(format!(
            "Valuation error for position '{}': {}",
            position_id, message
        )),
        #[cfg(feature = "scenarios")]
        PortfolioError::ScenarioError(msg) => {
            PyRuntimeError::new_err(format!("Scenario application failed: {}", msg))
        }
        PortfolioError::MissingMarketData(msg) => {
            PyRuntimeError::new_err(format!("Missing market data: {}", msg))
        }
        PortfolioError::Core(err) => PyRuntimeError::new_err(format!("Core error: {}", err)),
    }
}
