#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Umbrella crate for the **Finstack** quantitative-finance toolkit.
//!
//! Re-exports each sub-crate behind a Cargo feature flag so downstream
//! consumers can pull in only the pieces they need:
//!
//! | Feature       | Sub-crate                      |
//! |---------------|--------------------------------|
//! | `core`        | [`finstack_core`]              |
//! | `analytics`   | [`finstack_analytics`]         |
//! | `margin`      | [`finstack_margin`]            |
//! | `monte_carlo` | [`finstack_monte_carlo`]       |
//! | `valuations`  | [`finstack_valuations`]        |
//! | `statements`  | [`finstack_statements`]        |
//! | `portfolio`   | [`finstack_portfolio`]         |
//! | `scenarios`   | [`finstack_scenarios`]         |
//!
//! Credit correlation infrastructure (copulas, factor models, stochastic
//! recovery) now lives in [`finstack_valuations::correlation`] and is enabled
//! via the `valuations` feature.
//!
//! Enable `all` to pull in every sub-crate at once.

#[cfg(feature = "core")]
pub use finstack_core as core;

#[cfg(feature = "analytics")]
pub use finstack_analytics as analytics;

#[cfg(feature = "margin")]
pub use finstack_margin as margin;

#[cfg(feature = "valuations")]
pub use finstack_valuations as valuations;

#[cfg(feature = "statements")]
pub use finstack_statements as statements;

#[cfg(feature = "statements")]
pub use finstack_statements_analytics as statements_analytics;

#[cfg(feature = "portfolio")]
pub use finstack_portfolio as portfolio;

#[cfg(feature = "scenarios")]
pub use finstack_scenarios as scenarios;

/// Credit correlation infrastructure (copulas, factor models, stochastic recovery).
///
/// Re-export of [`finstack_valuations::correlation`], which absorbed the
/// former standalone `finstack-correlation` crate.
#[cfg(feature = "valuations")]
pub use finstack_valuations::correlation;

#[cfg(feature = "monte_carlo")]
pub use finstack_monte_carlo as monte_carlo;

// Bridge modules that wire multiple subcrates together
#[cfg(all(feature = "valuations", feature = "statements"))]
pub use finstack_statements_analytics::analysis::covenants;
