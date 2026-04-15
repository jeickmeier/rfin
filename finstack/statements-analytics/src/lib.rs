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
//! - **Extensions** — concrete analytics extensions (corkscrew, credit
//!   scorecard) called directly via inherent methods
//! - **Templates** — real estate, roll-forward, and vintage model builders

/// Analysis tools for financial statement models.
pub mod analysis;

/// Concrete extension implementations (corkscrew, credit scorecard).
pub mod extensions;

/// Templates for common financial model structures.
pub mod templates;

/// Convenient re-exports for common analytics types.
pub mod prelude;
