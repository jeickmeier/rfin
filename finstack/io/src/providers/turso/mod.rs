//! Turso backend for `finstack-io`.
//!
//! This module provides a Turso-backed store that uses the same schema as SQLite
//! with JSON payload blobs for domain objects, indexed by `(id, as_of)` where applicable.
//!
//! Turso is an in-process SQL database engine compatible with SQLite, offering
//! features like native JSON support, encryption at rest, and modern async I/O.

mod bulk_store;
mod core_store;
mod lookback_store;
mod store;
mod timeseries_store;

pub use store::{TursoConfig, TursoStore};
