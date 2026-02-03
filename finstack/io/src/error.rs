//! Error types for the `finstack-io` crate.

use thiserror::Error;

/// Result alias for `finstack-io`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while persisting or loading data.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// SQLite backend error.
    #[cfg(feature = "sqlite")]
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    /// JSON serialization/deserialization error.
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// I/O error (filesystem, etc).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Core domain error.
    #[error(transparent)]
    Core(#[from] finstack_core::Error),

    /// Portfolio domain error.
    #[error(transparent)]
    Portfolio(#[from] finstack_portfolio::PortfolioError),

    /// Statements domain error.
    #[error(transparent)]
    Statements(#[from] finstack_statements::Error),

    /// Scenarios domain error.
    #[error(transparent)]
    Scenarios(#[from] finstack_scenarios::error::Error),

    /// Requested entity was not found.
    #[error("Not found: {entity} '{id}'")]
    NotFound {
        /// Entity category (e.g. "portfolio", "instrument").
        entity: &'static str,
        /// Identifier.
        id: String,
    },

    /// Storage schema version mismatch.
    #[error("Unsupported schema version: found={found}, expected={expected}")]
    UnsupportedSchema {
        /// Version found in the store.
        found: i64,
        /// Version expected by this crate.
        expected: i64,
    },

    /// Internal invariant violated (bug or corrupted store).
    #[error("Storage invariant violated: {0}")]
    Invariant(String),
}

impl Error {
    /// Convenience constructor for a not-found error.
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }
}
