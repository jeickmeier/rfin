//! Persistence and interop utilities for the Finstack workspace.
//!
//! The primary goal of this crate is to provide a **stable persistence boundary**
//! for domain crates:
//! - market data snapshots (`MarketContext`) for historical lookbacks
//! - instruments, portfolios, scenarios, and statement model specs
//!
//! The recommended default backend is SQLite (embedded, transactional, easy to
//! operate). Backends are designed to be swappable via the [`Store`] trait.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
// Safety lints: Enforced - no expect() or panic!() allowed in this crate.
// Use proper error propagation with Result<T, E> instead.
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

pub mod config;
pub mod error;
pub(crate) mod helpers;
pub(crate) mod sql;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "turso")]
pub mod turso;

pub use config::{open_store_from_env, FinstackIoConfig, IoBackend, StoreHandle};
pub use error::{Error, Result};
pub use store::{
    BulkStore, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, SeriesKey, SeriesKind,
    Store, TimeSeriesPoint, TimeSeriesStore, MAX_BATCH_SIZE,
};

#[cfg(feature = "postgres")]
pub use postgres::{PostgresConfig, PostgresStore, DEFAULT_POOL_SIZE, DEFAULT_STATEMENT_TIMEOUT};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;
#[cfg(feature = "turso")]
pub use turso::TursoStore;
