#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Persistence and interop utilities for the Finstack workspace.
//!
//! `finstack-io` provides a **stable persistence boundary** for domain crates,
//! storing and retrieving:
//!
//! - **Market data** — [`MarketContext`](finstack_core::market_data::context::MarketContext)
//!   snapshots keyed by `(market_id, as_of)` for historical lookbacks
//! - **Instruments** — [`InstrumentJson`](finstack_valuations::instruments::InstrumentJson)
//!   definitions (bonds, deposits, swaps, etc.)
//! - **Portfolios** — [`PortfolioSpec`](finstack_portfolio::PortfolioSpec) snapshots
//!   keyed by `(portfolio_id, as_of)`
//! - **Scenarios** — [`ScenarioSpec`](finstack_scenarios::ScenarioSpec) definitions
//! - **Statement models** — [`FinancialModelSpec`](finstack_statements::FinancialModelSpec)
//!   specifications
//! - **Metric registries** — [`MetricRegistry`](finstack_statements::registry::MetricRegistry)
//!   namespaced metric definitions
//! - **Time-series** — quote, metric, result, PnL, and risk series via [`TimeSeriesStore`]
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                          Application                             │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  Store trait  │  BulkStore  │  LookbackStore  │ TimeSeriesStore  │
//! ├──────────────────────────────────────────────────────────────────┤
//! │                sql/statements.rs  (sea-query builders)           │
//! ├──────────────────┬──────────────────┬────────────────────────────┤
//! │   SqliteStore    │  PostgresStore   │       TursoStore           │
//! │ (default, embed) │ (scale-out)      │  (embedded, async)         │
//! └──────────────────┴──────────────────┴────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_io::{SqliteStore, Store};
//!
//! # async fn example() -> finstack_io::Result<()> {
//! // Open (or create) a SQLite database — migrations run automatically
//! let store = SqliteStore::open("finstack.db").await?;
//!
//! // Store an instrument
//! # let instrument = todo!();
//! store.put_instrument("DEPO-001", &instrument, None).await?;
//!
//! // Load it back
//! let loaded = store.get_instrument("DEPO-001").await?;
//! assert!(loaded.is_some());
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Flags
//!
//! | Feature    | Default | Backend                                         |
//! |------------|---------|--------------------------------------------------|
//! | `sqlite`   | **yes** | Embedded SQLite via `rusqlite` / `tokio-rusqlite` |
//! | `postgres` | no      | Postgres via `deadpool-postgres`                  |
//! | `turso`    | no      | Turso/libsql (SQLite-compatible, async I/O)       |
//!
//! Enable additional backends in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! finstack-io = { version = "0.4", features = ["postgres", "turso"] }
//! ```
//!
//! # External Schema Management
//!
//! By default, stores run built-in migrations automatically on open/connect.
//! If your schema is managed by an external tool (Liquibase, Flyway, etc.),
//! disable auto-migration via the per-backend config:
//!
//! ```rust,no_run
//! # use finstack_io::{SqliteStore, SqliteConfig};
//! # async fn example() -> finstack_io::Result<()> {
//! let store = SqliteStore::open_with_config(
//!     "data/finstack.db",
//!     SqliteConfig::new().without_migrations(),
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Or via environment variable when using [`open_store_from_env`]:
//!
//! ```bash
//! FINSTACK_AUTO_MIGRATE=false
//! ```
//!
//! You can always run migrations later via the public `migrate()` method
//! on any store.
//!
//! # Modules
//!
//! - [`store`] — Core persistence traits ([`Store`], [`BulkStore`], [`LookbackStore`],
//!   [`TimeSeriesStore`]) and associated types.
//! - [`config`] — Environment-based configuration for backend selection
//!   ([`open_store_from_env`], [`FinstackIoConfig`]).
//! - [`error`] — Error types ([`Error`], [`Result`]).
//! - [`providers`] — Backend implementations (SQLite, Postgres, Turso).

/// Backend configuration and environment helpers.
pub mod config;
/// Error types for persistence operations.
pub mod error;
pub(crate) mod helpers;
pub(crate) mod sql;
/// Store traits and persistence primitives.
pub mod store;

/// Backend provider implementations (SQLite, Postgres, Turso).
pub mod providers;

#[cfg(feature = "postgres")]
pub use providers::postgres;
#[cfg(feature = "sqlite")]
pub use providers::sqlite;
#[cfg(feature = "turso")]
pub use providers::turso;

pub use config::{open_store_from_env, FinstackIoConfig, IoBackend, StoreHandle};
pub use error::{Error, Result};
pub use sql::schema::TableNaming;
pub use store::{
    BulkStore, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, SeriesKey, SeriesKind,
    Store, TimeSeriesPoint, TimeSeriesStore, MAX_BATCH_SIZE,
};

#[cfg(feature = "postgres")]
pub use postgres::{PostgresConfig, PostgresStore, DEFAULT_POOL_SIZE, DEFAULT_STATEMENT_TIMEOUT};
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteConfig, SqliteStore};
#[cfg(feature = "turso")]
pub use turso::{TursoConfig, TursoStore};
