#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Financial primitives & date utilities for the **Finstack** ecosystem.
//!
//! This crate exposes lightweight, composable building-blocks that are
//! commonly required in pricing engines and risk systems:
//!
//! * [`currency::Currency`] – ISO-4217 codes with numeric identifiers and metadata
//! * [`money::Money`] – type-safe monetary amounts that refuse to mix currencies
//! * [`dates`] – date/time scaffolding (business calendars, day-count, schedules)
//!
//! Note: This crate relies on the Rust standard library. Previous `no_std` claims
//! have been removed.
//!
//! # Quick start
//! ```
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! # fn main() -> finstack_core::Result<()> {
//!
//! // Parse ISO-4217 codes (case-insensitive)
//! let eur = "eur"
//!     .parse::<Currency>()
//!     .expect("valid ISO-4217 currency");
//!
//! // Perform arithmetic that refuses to mix currencies
//! let subtotal = Money::new(49.50, Currency::EUR);
//! let tax      = Money::new( 9.90, Currency::EUR);
//! let total    = (subtotal + tax)?;
//! assert_eq!(format!("{}", total), "EUR 59.40");
//! # Ok(())
//! # }
//! ```
//!
//! # Cargo features
//! Serde support is always enabled in this crate; no feature flags are required.
//!
//! # Minimum Supported Rust Version (MSRV)
//! This crate targets **Rust 1.90**.  It is tested in CI and follows the
//! standard *cargo-semver* guideline: MSRV may only bump in a **minor** release.
//!
//! ---
//! _Released under the MIT license.  Contributions welcome!_

// Core modules
//
// API note: `collections` is intentionally kept as an internal module to avoid
// committing to a public submodule layout. Downstream crates should import the
// aliases directly from the crate root (`finstack_core::HashMap`).
pub(crate) mod collections;
pub mod config;
pub mod currency;
pub(crate) mod error;
/// Market data term‐structure framework (former `curves` module)
pub mod market_data;
/// Currency-tagged monetary amounts with safe arithmetic
pub mod money;
/// Explainability infrastructure (opt-in tracing)
pub mod explain;
/// Date & calendar helpers (facade over the `time` crate)
pub mod dates;
/// Numerical helpers (root finding, summation, stats)
pub mod math;
/// Expression engine (AST, evaluator, Polars lowering)
pub mod expr;
/// Core type definitions (phantom-typed IDs, rates, etc.)
pub mod types;
/// Foundational cashflow primitives and discounting helpers.
pub mod cashflow;

/// Hash map type alias used across Finstack.
///
/// Uses `rustc_hash::FxHashMap` for fast deterministic hashing.
pub use collections::HashMap;
/// Hash set type alias used across Finstack.
///
/// Uses `rustc_hash::FxHashSet` for fast deterministic hashing.
pub use collections::HashSet;

// Re-export main error types for convenience.
pub use error::{Error, InputError};
/// Convenient alias carrying the crate's unified [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
