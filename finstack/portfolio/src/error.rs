//! Error types for portfolio operations.

use crate::types::{EntityId, PositionId};
use finstack_core::prelude::*;
use thiserror::Error;

/// Convenience result type used throughout the portfolio crate.
///
/// This alias helps reduce boilerplate when returning [`PortfolioError`].
pub type Result<T> = std::result::Result<T, PortfolioError>;

/// Errors that can occur during portfolio operations.
///
/// Each variant captures the context needed to diagnose failures when building,
/// validating, or valuing a portfolio.
#[derive(Error, Debug)]
pub enum PortfolioError {
    /// Position references an unknown entity
    #[error("Position '{position_id}' references unknown entity '{entity_id}'")]
    UnknownEntity {
        /// Position identifier.
        position_id: PositionId,
        /// Entity identifier that was not found.
        entity_id: EntityId,
    },

    /// Portfolio validation failed
    #[error("Portfolio validation failed: {0}")]
    ValidationFailed(String),

    /// FX conversion failed
    #[error("FX conversion failed: {from} to {to}")]
    FxConversionFailed {
        /// Source currency.
        from: Currency,
        /// Target currency.
        to: Currency,
    },

    /// Valuation error
    #[error("Valuation error for position '{position_id}': {message}")]
    ValuationError {
        /// Position identifier.
        position_id: PositionId,
        /// Error message describing the valuation failure.
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

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Builder construction error
    #[error("Builder error: {0}")]
    BuilderError(String),

    /// Index/collection access error
    #[error("Index error: {0}")]
    IndexError(String),
}

impl PortfolioError {
    /// Create a validation error with context
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationFailed(msg.into())
    }

    /// Create a valuation error with context
    pub fn valuation(position_id: impl Into<PositionId>, msg: impl Into<String>) -> Self {
        Self::ValuationError {
            position_id: position_id.into(),
            message: msg.into(),
        }
    }

    /// Create an FX conversion error
    pub fn fx_conversion(from: Currency, to: Currency) -> Self {
        Self::FxConversionFailed { from, to }
    }

    /// Create a missing market data error
    pub fn missing_market_data(msg: impl Into<String>) -> Self {
        Self::MissingMarketData(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a builder error
    pub fn builder_error(msg: impl Into<String>) -> Self {
        Self::BuilderError(msg.into())
    }

    /// Create an index error
    pub fn index_error(msg: impl Into<String>) -> Self {
        Self::IndexError(msg.into())
    }

    /// Create a scenario error
    #[cfg(feature = "scenarios")]
    pub fn scenario_error(msg: impl Into<String>) -> Self {
        Self::ScenarioError(msg.into())
    }
}
