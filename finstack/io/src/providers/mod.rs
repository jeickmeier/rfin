//! Database provider implementations for the [`Store`](crate::Store) trait.
//!
//! Each provider is feature-gated and implements all four persistence traits:
//! [`Store`](crate::Store), [`BulkStore`](crate::BulkStore),
//! [`LookbackStore`](crate::LookbackStore), and
//! [`TimeSeriesStore`](crate::TimeSeriesStore).
//!
//! | Provider | Feature | Crate | Use Case |
//! |----------|---------|-------|----------|
//! | [`SqliteStore`](sqlite::SqliteStore) | `sqlite` (default) | `rusqlite` + `tokio-rusqlite` | Embedded, single-process, zero setup |
//! | `PostgresStore` | `postgres` | `deadpool-postgres` | Multi-process, connection pooling, scale-out |
//! | `TursoStore` | `turso` | `libsql` | Embedded, native async, encryption at rest |
//!
//! All providers share the same SQL schema (defined in `sql`) and
//! use `sea-query` for backend-portable statement generation. Migrations run
//! automatically on first connection.

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "turso")]
pub mod turso;
