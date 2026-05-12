#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Performance analytics on numeric slices and `finstack_core::dates::Date`.
//!
//! [`crate::performance::Performance`] is the canonical entry point: construct
//! it from a price panel or a return panel, then every analytic — return /
//! risk scalars, drawdown statistics, rolling windows, periodic returns
//! (MTD / QTD / YTD / FYTD), benchmark alpha / beta, basic factor models — is
//! a method on the resulting instance.
//!
//! The per-module functions exposed below are the building blocks
//! `Performance` is composed of. They remain `pub` to keep the crate testable
//! and to support narrow callers that already hold a clean return slice, but
//! new callers should reach for `Performance`.
//!
//! Key conventions:
//! - returns are simple decimal returns unless a function explicitly says otherwise
//! - annualization is derived from `finstack_core::dates::PeriodKind` when called
//!   through [`crate::performance::Performance`]
//! - drawdown depths are non-positive fractions such as `-0.25` for a 25% loss
//! - benchmark inputs are assumed pre-aligned to the panel's date grid
//! - rolling series are right-labeled: each output value is dated by the last
//!   observation in its window
//!
//! Module map:
//! - [`crate::performance`] — stateful `Performance` facade over a price/return panel
//! - [`crate::returns`] — return transforms and compounding
//! - [`crate::risk_metrics`] — return- and tail-based ratios + rolling kernels
//! - [`crate::drawdown`] — drawdown paths, episodes, and drawdown-derived ratios
//! - [`crate::benchmark`] — greeks, capture / batting, multi-factor regression
//! - [`crate::aggregation`] — period grouping and trading statistics
//! - [`crate::lookback`] — MTD / QTD / YTD / FYTD index selectors

// Internal re-exports of frequently used `finstack-core` modules.
// Kept `pub(crate)` so they don't leak into the public API; downstream callers
// should import from `finstack_core` directly.
pub(crate) use finstack_core::{dates, error, math};

type Result<T> = finstack_core::Result<T>;

pub mod aggregation;
pub mod benchmark;
pub mod drawdown;
pub mod lookback;
pub mod performance;
pub mod returns;
pub mod risk_metrics;

pub use aggregation::PeriodStats;
pub use performance::{LookbackReturns, Performance};
