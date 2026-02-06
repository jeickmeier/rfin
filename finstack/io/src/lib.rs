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
//! │  GovernedHandle  (optional row-level permissions & workflow)     │
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
//! # Modules
//!
//! - [`store`] — Core persistence traits ([`Store`], [`BulkStore`], [`LookbackStore`],
//!   [`TimeSeriesStore`]) and associated types.
//! - [`governance`] — Optional enterprise governance layer with row-level
//!   permissions, change proposals, and configurable approval workflows.
//!   See [`GovernedHandle`] for the primary API.
//! - [`config`] — Environment-based configuration for backend selection
//!   ([`open_store_from_env`], [`FinstackIoConfig`]).
//! - [`error`] — Error types ([`Error`], [`Result`]).
//! - [`providers`] — Backend implementations (SQLite, Postgres, Turso).

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
pub mod governance;
pub(crate) mod helpers;
pub(crate) mod sql;
pub mod store;

pub mod providers;

#[cfg(feature = "postgres")]
pub use providers::postgres;
#[cfg(feature = "sqlite")]
pub use providers::sqlite;
#[cfg(feature = "turso")]
pub use providers::turso;

pub use config::{open_store_from_env, FinstackIoConfig, IoBackend, StoreHandle};
pub use error::{Error, Result};
pub use governance::{
    ActorContext, ActorKind, ChangeKind, GovernanceConfig, GovernedHandle, ResourceChange,
    ResourceChangeInsert, ResourceEntity, ResourceShare, SharePermission, ShareType, UserRole,
    VisibilityScope,
};
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
