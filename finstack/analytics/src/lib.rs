#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Performance analytics on numeric slices and `finstack_core::dates::Date`.
//!
//! Start with [`crate::performance::Performance`] when you want a stateful,
//! benchmark-aware facade over a full panel of ticker returns. Reach for the
//! individual modules when you want standalone, allocation-light functions on
//! pre-computed return or drawdown slices.
//!
//! Key conventions:
//! - returns are simple decimal returns unless a function explicitly says otherwise
//! - annualization uses the caller-supplied or [`crate::dates::PeriodKind`]-derived
//!   periods-per-year factor
//! - drawdown depths are non-positive fractions such as `-0.25` for a 25% loss
//! - benchmark-relative metrics operate on return series, not fill-forwarded prices
//!
//! See [`crate::risk_metrics`] for return- and tail-based ratios,
//! [`crate::drawdown`] for drawdown path analytics, and
//! [`crate::benchmark`] for benchmark-relative regressions and attribution.

pub(crate) use finstack_core::{dates, error, math};
type Result<T> = finstack_core::Result<T>;

pub mod aggregation;
pub mod backtesting;
pub mod benchmark;
pub mod comps;
pub mod drawdown;
pub mod lookback;
pub mod performance;
pub mod returns;
pub mod risk_metrics;
pub mod timeseries;

pub use aggregation::PeriodStats;
pub use backtesting::BacktestResult;
pub use performance::{LookbackReturns, Performance};
