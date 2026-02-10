//! Errors emitted by the scenarios crate.
//!
//! Most adapter functions and engine methods return the [`Result`] alias which
//! wraps this [`Error`] type. Variants attempt to surface actionable messages so
//! callers can decide whether to retry, skip, or abort a scenario application.

use thiserror::Error;

/// Convenient result alias used across the crate.
///
/// Returning this type ensures downstream callers can pattern match on
/// [`enum@Error`] without importing `std::result::Result`.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::error::{Error, Result};
///
/// fn compute(flag: bool) -> Result<()> {
///     if flag {
///         Ok(())
///     } else {
///         Err(Error::Validation("flag must be true".into()))
///     }
/// }
///
/// assert!(compute(true).is_ok());
/// assert!(compute(false).is_err());
/// ```
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during scenario execution.
///
/// The variants are intentionally granular so adapters can convey the precise
/// failure reason (missing market data, invalid tenor, unsupported operation,
/// and so on).
///
/// # Examples
/// ```rust
/// use finstack_scenarios::error::Error;
///
/// fn classify(err: Error) -> &'static str {
///     match err {
///         Error::MarketDataNotFound { .. } => "market",
///         Error::NodeNotFound { .. } => "statements",
///         _ => "other",
///     }
/// }
///
/// assert_eq!(classify(Error::NodeNotFound { node_id: "Revenue".into() }), "statements");
/// ```
/// # Derive policy
///
/// All Finstack domain error types that may cross FFI boundaries (Python/WASM)
/// derive `Serialize`/`Deserialize`. `PartialEq` is included for ergonomic
/// assertions in tests. Infrastructure errors (e.g. `finstack_io::Error`) that
/// wrap opaque driver types may opt out of `Serialize` and `PartialEq`.
#[derive(Debug, Clone, PartialEq, Error, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Error {
    /// Market data element not found.
    #[error("Market data not found: {id}")]
    MarketDataNotFound {
        /// Identifier of the missing market data element.
        id: String,
    },

    /// Statement node not found.
    #[error("Statement node not found: {node_id}")]
    NodeNotFound {
        /// Identifier of the missing statement node.
        node_id: String,
    },

    /// Curve type mismatch.
    #[error("Curve type mismatch: expected {expected}, got {actual}")]
    CurveTypeMismatch {
        /// Expected curve type.
        expected: String,
        /// Actual curve type encountered.
        actual: String,
    },

    /// Unsupported operation for target.
    #[error("Unsupported operation {operation} for target {target}")]
    UnsupportedOperation {
        /// Operation being attempted.
        operation: String,
        /// Target on which the operation is unsupported.
        target: String,
    },

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
    TenorNotFound {
        /// Tenor string that was not found.
        tenor: String,
        /// Identifier of the curve.
        curve_id: String,
    },

    /// Invalid time period.
    #[error("Invalid time period: {0}")]
    InvalidPeriod(String),

    /// Instrument not found.
    #[error("Instrument not found: {0}")]
    InstrumentNotFound(String),
}

impl Error {
    /// Create a market data not found error
    pub fn market_data_not_found(id: impl Into<String>) -> Self {
        Self::MarketDataNotFound { id: id.into() }
    }

    /// Create a node not found error
    pub fn node_not_found(node_id: impl Into<String>) -> Self {
        Self::NodeNotFound {
            node_id: node_id.into(),
        }
    }

    /// Create a curve type mismatch error
    pub fn curve_type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::CurveTypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported_operation(operation: impl Into<String>, target: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
            target: target.into(),
        }
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create an invalid tenor error
    pub fn invalid_tenor(tenor: impl Into<String>) -> Self {
        Self::InvalidTenor(tenor.into())
    }

    /// Create a tenor not found error
    pub fn tenor_not_found(tenor: impl Into<String>, curve_id: impl Into<String>) -> Self {
        Self::TenorNotFound {
            tenor: tenor.into(),
            curve_id: curve_id.into(),
        }
    }

    /// Create an invalid period error
    pub fn invalid_period(period: impl Into<String>) -> Self {
        Self::InvalidPeriod(period.into())
    }

    /// Create an instrument not found error
    pub fn instrument_not_found(instrument: impl Into<String>) -> Self {
        Self::InstrumentNotFound(instrument.into())
    }
}
