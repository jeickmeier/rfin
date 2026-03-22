#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! # Finstack Statements Analytics
//!
//! Higher-level analysis, reporting, and extension implementations that build
//! on the core [`finstack_statements`] evaluation engine.
//!
//! This crate provides:
//!
//! - **Analysis** — sensitivity, scenario sets, variance, DCF, goal seek,
//!   covenants, backtesting, Monte Carlo, and introspection
//! - **Extensions** — concrete `Extension` implementations (corkscrew,
//!   credit scorecard)
//! - **Templates** — real estate, roll-forward, and vintage model builders

/// Analysis tools for financial statement models.
pub mod analysis;

/// Convenient re-exports for common analytics types.
pub mod prelude;
