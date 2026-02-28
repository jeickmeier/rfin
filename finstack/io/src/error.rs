//! Error types for the `finstack-io` crate.
//!
//! All fallible operations in this crate return [`Result<T>`], which is an alias
//! for `std::result::Result<T, Error>`.
//!
//! # Derive policy
//!
//! This error type derives `Clone` (via `Arc`-wrapping non-`Clone` driver
//! errors) but intentionally omits `PartialEq`, `Serialize`, and `Deserialize`.
//! IO errors wrap opaque third-party driver types (`rusqlite::Error`,
//! `tokio_postgres::Error`, etc.) that do not implement those traits.
//!
//! Domain errors (`finstack_core::Error`, `finstack_portfolio::Error`, etc.)
//! **do** derive `Serialize`/`Deserialize`/`PartialEq` so they can cross FFI
//! boundaries cleanly.

use std::sync::Arc;

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
///   when the corresponding feature is enabled. Wrapped in [`Arc`] to enable
///   `Clone`.
/// - **Serialization** — `SerdeJson`: JSON (de)serialization failures,
///   typically when a stored payload does not match the expected Rust type.
/// - **I/O** — `Io`: filesystem errors (e.g., cannot create directory for database file).
/// - **Domain** — `Core`, `Portfolio`, `Statements`, `Scenarios`: errors
///   propagated from Finstack domain crates during hydration or conversion.
/// - **Application** — `NotFound`, `UnsupportedSchema`,
///   `Invariant`, `InvalidSeriesKind`: semantic errors raised by this crate's
///   business logic.
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum Error {
    /// SQLite backend error (synchronous rusqlite).
    #[cfg(feature = "sqlite")]
    #[error("{0}")]
    Sqlite(Arc<rusqlite::Error>),

    /// Async SQLite backend error (tokio-rusqlite).
    #[cfg(feature = "sqlite")]
    #[error("{0}")]
    SqliteAsync(Arc<tokio_rusqlite::Error>),

    /// Postgres backend error (tokio-postgres).
    #[cfg(feature = "postgres")]
    #[error("{0}")]
    Postgres(Arc<tokio_postgres::Error>),

    /// Postgres pool error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres pool error: {0}")]
    PostgresPool(Arc<deadpool_postgres::PoolError>),

    /// Postgres config error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres config error: {0}")]
    PostgresConfig(Arc<deadpool_postgres::ConfigError>),

    /// Postgres build error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres build error: {0}")]
    PostgresBuild(Arc<deadpool_postgres::BuildError>),

    /// Postgres create pool error (deadpool).
    #[cfg(feature = "postgres")]
    #[error("Postgres create pool error: {0}")]
    PostgresCreatePool(Arc<deadpool_postgres::CreatePoolError>),

    /// Turso/libsql backend error.
    #[cfg(feature = "turso")]
    #[error("{0}")]
    Turso(Arc<libsql::Error>),

    /// JSON serialization/deserialization error.
    #[error("{0}")]
    SerdeJson(Arc<serde_json::Error>),

    /// I/O error (filesystem, etc).
    #[error("{0}")]
    Io(Arc<std::io::Error>),

    /// Core domain error.
    #[error(transparent)]
    Core(#[from] finstack_core::Error),

    /// Portfolio domain error.
    #[error(transparent)]
    Portfolio(#[from] finstack_portfolio::Error),

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

    /// Invalid series kind identifier.
    #[error("Invalid series kind: '{0}' (expected one of: quote, metric, result, pnl, risk)")]
    InvalidSeriesKind(String),
}

// ---------------------------------------------------------------------------
// From impls for Arc-wrapped external errors (preserves `?` ergonomics)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(Arc::new(err))
    }
}

#[cfg(feature = "sqlite")]
impl From<tokio_rusqlite::Error> for Error {
    fn from(err: tokio_rusqlite::Error) -> Self {
        Self::SqliteAsync(Arc::new(err))
    }
}

#[cfg(feature = "sqlite")]
impl From<tokio_rusqlite::Error<Error>> for Error {
    fn from(err: tokio_rusqlite::Error<Error>) -> Self {
        match err {
            tokio_rusqlite::Error::Error(e) => e,
            tokio_rusqlite::Error::ConnectionClosed => {
                Self::Invariant("SQLite connection closed".into())
            }
            tokio_rusqlite::Error::Close((_, e)) => Self::Sqlite(Arc::new(e)),
            _ => Self::Invariant("Unknown tokio-rusqlite error".into()),
        }
    }
}

#[cfg(feature = "postgres")]
impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Self::Postgres(Arc::new(err))
    }
}

#[cfg(feature = "postgres")]
impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::PostgresPool(Arc::new(err))
    }
}

#[cfg(feature = "postgres")]
impl From<deadpool_postgres::ConfigError> for Error {
    fn from(err: deadpool_postgres::ConfigError) -> Self {
        Self::PostgresConfig(Arc::new(err))
    }
}

#[cfg(feature = "postgres")]
impl From<deadpool_postgres::BuildError> for Error {
    fn from(err: deadpool_postgres::BuildError) -> Self {
        Self::PostgresBuild(Arc::new(err))
    }
}

#[cfg(feature = "postgres")]
impl From<deadpool_postgres::CreatePoolError> for Error {
    fn from(err: deadpool_postgres::CreatePoolError) -> Self {
        Self::PostgresCreatePool(Arc::new(err))
    }
}

#[cfg(feature = "turso")]
impl From<libsql::Error> for Error {
    fn from(err: libsql::Error) -> Self {
        Self::Turso(Arc::new(err))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJson(Arc::new(err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(Arc::new(err))
    }
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
