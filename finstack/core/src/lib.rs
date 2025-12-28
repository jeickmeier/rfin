#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
// Safety lints: Defined but temporarily allowed during migration.
// Many uses are intentional patterns (convenience methods, epoch dates, type invariants).
// Functions with intentional uses have local #[allow(...)] with comments.
//
// Remaining work tracked at: https://github.com/..../issues/XXX
// - ~50 expect() calls need review (many are in convenience methods with try_* counterparts)
// - Test modules need #[allow(...)] attributes
//
// For new code: DO NOT use expect() or panic!() in production paths.
// Use try_new() patterns and proper error types instead.
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
//! | Feature       | Purpose                                            |
//! |-------------- |----------------------------------------------------|
//! | `std`         | Required standard library support (always enabled)   |
//! | `serde`       | `Serialize`/`Deserialize` for public types         |
//!
//! # Minimum Supported Rust Version (MSRV)
//! This crate targets **Rust 1.90**.  It is tested in CI and follows the
//! standard *cargo-semver* guideline: MSRV may only bump in a **minor** release.
//!
//! ---
//! _Released under the MIT license.  Contributions welcome!_

// Core modules
pub mod collections;
pub mod config;
pub mod currency;
pub mod error;
/// Market data term‐structure framework (former `curves` module)
pub mod market_data;
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

// Re-export main error type for convenience
pub use error::Error;
/// Convenient alias carrying the crate's unified [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
