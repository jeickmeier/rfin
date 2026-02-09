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
//! | Feature       | Sub-crate              |
//! |---------------|------------------------|
//! | `core`        | [`finstack-core`]      |
//! | `valuations`  | [`finstack-valuations`]|
//! | `statements`  | [`finstack-statements`]|
//! | `portfolio`   | [`finstack-portfolio`] |
//! | `scenarios`   | [`finstack-scenarios`] |
//! | `io`          | [`finstack-io`]        |
//!
//! Enable `all` to pull in every sub-crate at once.

#[cfg(feature = "core")]
pub use finstack_core as core;

#[cfg(feature = "valuations")]
pub use finstack_valuations as valuations;

#[cfg(feature = "statements")]
pub use finstack_statements as statements;

#[cfg(feature = "portfolio")]
pub use finstack_portfolio as portfolio;

#[cfg(feature = "scenarios")]
pub use finstack_scenarios as scenarios;

#[cfg(feature = "io")]
pub use finstack_io as io;

// Bridge modules that wire multiple subcrates together
#[cfg(all(feature = "valuations", feature = "statements"))]
pub use finstack_statements::analysis::covenants;
