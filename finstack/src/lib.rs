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
//! | `core`        | [`finstack-core`]              |
//! | `analytics`   | [`finstack-analytics`]         |
//! | `correlation` | [`finstack-correlation`]       |
//! | `margin`      | [`finstack-margin`]            |
//! | `monte_carlo` | [`finstack-monte-carlo`]       |
//! | `valuations`  | [`finstack-valuations`]        |
//! | `statements`  | [`finstack-statements`]        |
//! | `portfolio`   | [`finstack-portfolio`]         |
//! | `scenarios`   | [`finstack-scenarios`]         |
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

#[cfg(feature = "correlation")]
pub use finstack_correlation as correlation;

#[cfg(feature = "monte_carlo")]
pub use finstack_monte_carlo as monte_carlo;

// Bridge modules that wire multiple subcrates together
#[cfg(all(feature = "valuations", feature = "statements"))]
pub use finstack_statements_analytics::analysis::covenants;
