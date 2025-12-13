#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]

//! Financial primitives & date utilities for the **Finstack** ecosystem.
//!
//! This crate exposes lightweight, composable building-blocks that are
//! commonly required in pricing engines and risk systems:
//!
//! * [`Currency`] – ISO-4217 codes with numeric identifiers and metadata
//! * [`Money`] – type-safe monetary amounts that refuse to mix currencies
//! * [`time`] – date/time scaffolding (business calendars, day-count, schedules)
//!
//! Note: This crate relies on the Rust standard library. Previous `no_std` claims
//! have been removed.
//!
//! # Quick start
//! ```
//! use finstack_core::prelude::*;
//!
//! // Parse ISO-4217 codes (case-insensitive)
//! let eur = "eur".parse::<Currency>().unwrap();
//!
//! // Perform arithmetic that refuses to mix currencies
//! let subtotal = Money::new(49.50, Currency::EUR);
//! let tax      = Money::new( 9.90, Currency::EUR);
//! let total    = (subtotal + tax).unwrap();
//! assert_eq!(format!("{}", total), "EUR 59.40");
//! ```
//!
//! See also: the [`prelude`] module for a curated set of commonly used types.
//!
//! # Cargo features
//! | Feature       | Purpose                                            |
//! |-------------- |----------------------------------------------------|
//! | `std`         | Required standard library support (always enabled)   |
//! | `serde`       | `Serialize`/`Deserialize` for public types         |
//!
//! # Minimum Supported Rust Version (MSRV)
//! This crate targets **Rust 1.75**.  It is tested in CI and follows the
//! standard *cargo-semver* guideline: MSRV may only bump in a **minor** release.
//!
//! ---
//! _Released under the MIT license.  Contributions welcome!_

// Core modules
pub mod config;
pub mod currency;
pub mod error;
/// Market data term‐structure framework (former `curves` module)
pub mod market_data;
pub mod money;
pub mod prelude;

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

/// Volatility conventions and conversion utilities.
pub mod volatility;

/// Solver configuration for numerical root-finding algorithms.
pub mod solver_config;

/// XIRR (Extended Internal Rate of Return) configuration.
pub mod xirr_config;

// Re-export main error type for convenience
pub use error::Error;
/// Convenient alias carrying the crate's unified [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
