//! Error types for the scenarios crate.

use thiserror::Error;

/// Result type alias using the scenarios error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during scenario execution.
#[derive(Error, Debug)]
pub enum Error {
    /// Market data element not found.
    #[error("Market data not found: {id}")]
    MarketDataNotFound { id: String },

    /// Statement node not found.
    #[error("Statement node not found: {node_id}")]
    NodeNotFound { node_id: String },

    /// Curve type mismatch.
    #[error("Curve type mismatch: expected {expected}, got {actual}")]
    CurveTypeMismatch { expected: String, actual: String },

    /// Unsupported operation for target.
    #[error("Unsupported operation {operation} for target {target}")]
    UnsupportedOperation { operation: String, target: String },

    /// Core library error.
    #[error(transparent)]
    Core(#[from] finstack_core::Error),

    /// Statements library error.
    #[error(transparent)]
    Statements(#[from] finstack_statements::error::Error),

    /// General validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Invalid tenor string.
    #[error("Invalid tenor string: {0}")]
    InvalidTenor(String),

    /// Tenor not found in curve.
    #[error("Tenor not found in curve: {tenor} in {curve_id}")]
    TenorNotFound { tenor: String, curve_id: String },

    /// Invalid time period.
    #[error("Invalid time period: {0}")]
    InvalidPeriod(String),

    /// Instrument not found.
    #[error("Instrument not found: {0}")]
    InstrumentNotFound(String),
}
