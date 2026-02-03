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

pub mod error;
pub mod store;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use error::{Error, Result};
pub use store::{BulkStore, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Store};

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;
