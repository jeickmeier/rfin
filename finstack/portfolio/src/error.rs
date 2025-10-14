//! Error types for portfolio operations.

use crate::types::{EntityId, PositionId};
use finstack_core::prelude::*;
use thiserror::Error;

/// Convenience result type used throughout the portfolio crate.
///
/// This alias helps reduce boilerplate when returning [`PortfolioError`].
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::Result;
///
/// fn do_work() -> Result<()> {
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, PortfolioError>;

/// Errors that can occur during portfolio operations.
///
/// Each variant captures the context needed to diagnose failures when building,
/// validating, or valuing a portfolio.
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::{PortfolioError, PositionId, EntityId};
///
/// let position_id: PositionId = "POS_1".into();
/// let entity_id: EntityId = "UNKNOWN".into();
/// let error = PortfolioError::UnknownEntity {
///     position_id,
///     entity_id,
/// };
/// assert!(matches!(error, PortfolioError::UnknownEntity { .. }));
/// ```
#[derive(Error, Debug)]
pub enum PortfolioError {
    /// Position references an unknown entity
    #[error("Position '{position_id}' references unknown entity '{entity_id}'")]
    UnknownEntity {
        position_id: PositionId,
        entity_id: EntityId,
    },
    
    /// Portfolio validation failed
    #[error("Portfolio validation failed: {0}")]
    ValidationFailed(String),
    
    /// FX conversion failed
    #[error("FX conversion failed: {from} to {to}")]
    FxConversionFailed { from: Currency, to: Currency },
    
    /// Valuation error
    #[error("Valuation error for position '{position_id}': {message}")]
    ValuationError {
        position_id: PositionId,
        message: String,
    },
    
    /// Scenario application error
    #[cfg(feature = "scenarios")]
    #[error("Scenario application failed: {0}")]
    ScenarioError(String),
    
    /// Missing market data
    #[error("Missing market data: {0}")]
    MissingMarketData(String),
    
    /// Core error
    #[error(transparent)]
    Core(#[from] finstack_core::Error),
}
