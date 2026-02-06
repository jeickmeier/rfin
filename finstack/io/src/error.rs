//! Error types for the `finstack-io` crate.
//!
//! All fallible operations in this crate return [`Result<T>`], which is an alias
//! for `std::result::Result<T, Error>`.

use thiserror::Error;

/// Result alias for `finstack-io`.
///
/// All public methods in this crate return this type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while persisting or loading data.
///
/// Error variants are grouped into categories:
///
/// - **Backend errors** — `Sqlite`, `SqliteAsync`, `Postgres*`, `Turso`:
///   low-level driver errors from the selected database provider. Only present
///   when the corresponding feature is enabled.
/// - **Serialization** — `SerdeJson`: JSON (de)serialization failures,
///   typically when a stored payload does not match the expected Rust type.
/// - **I/O** — `Io`: filesystem errors (e.g., cannot create directory for database file).
/// - **Domain** — `Core`, `Portfolio`, `Statements`, `Scenarios`: errors
///   propagated from Finstack domain crates during hydration or conversion.
/// - **Application** — `NotFound`, `PermissionDenied`, `UnsupportedSchema`,
///   `Invariant`, `InvalidSeriesKind`: semantic errors raised by this crate's
///   business logic.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// SQLite backend error (synchronous rusqlite).
    #[cfg(feature = "sqlite")]
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    /// Async SQLite backend error (tokio-rusqlite).
    #[cfg(feature = "sqlite")]
    #[error(transparent)]
    SqliteAsync(#[from] tokio_rusqlite::Error),

    /// Postgres backend error (tokio-postgres).
    #[cfg(feature = "postgres")]
    #[error(transparent)]
    Postgres(#[from] tokio_postgres::Error),

    /// Postgres pool error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres pool error: {0}")]
    PostgresPool(#[from] deadpool_postgres::PoolError),

    /// Postgres config error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres config error: {0}")]
    PostgresConfig(#[from] deadpool_postgres::ConfigError),

    /// Postgres build error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres build error: {0}")]
    PostgresBuild(#[from] deadpool_postgres::BuildError),

    /// Postgres create pool error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres create pool error: {0}")]
    PostgresCreatePool(#[from] deadpool_postgres::CreatePoolError),

    /// Turso/libsql backend error.
    #[cfg(feature = "turso")]
    #[error(transparent)]
    Turso(#[from] libsql::Error),

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

    /// Permission denied for an action.
    #[error("Permission denied: {action} on {resource_type} '{resource_id}'")]
    PermissionDenied {
        /// Action attempted.
        action: &'static str,
        /// Resource type.
        resource_type: String,
        /// Resource id or identifier.
        resource_id: String,
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

    /// Invalid series kind identifier.
    #[error("Invalid series kind: '{0}' (expected one of: quote, metric, result, pnl, risk)")]
    InvalidSeriesKind(String),
}

impl Error {
    /// Convenience constructor for a not-found error.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_io::Error;
    ///
    /// let err = Error::not_found("instrument", "DEPO-001");
    /// assert_eq!(err.to_string(), "Not found: instrument 'DEPO-001'");
    /// ```
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }

    /// Convenience constructor for an invalid series kind error.
    pub fn invalid_series_kind(value: impl Into<String>) -> Self {
        Self::InvalidSeriesKind(value.into())
    }
}
