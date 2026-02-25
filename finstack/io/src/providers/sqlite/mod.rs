//! SQLite backend for `finstack-io`.
//!
//! This module provides a minimal, predictable schema with JSON payload blobs
//! for domain objects, indexed by `(id, as_of)` where applicable.

mod bulk_store;
mod core_store;
mod lookback_store;
mod store;
mod timeseries_store;

pub use store::{SqliteConfig, SqliteStore};
