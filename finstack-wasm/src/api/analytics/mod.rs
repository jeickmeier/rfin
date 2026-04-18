//! WASM bindings for the `finstack-analytics` crate.
//!
//! The bindings are split by the same domain boundaries as the Rust crate to
//! keep wrapper-only code reviewable and reduce drift pressure.

mod aggregation;
mod backtesting;
mod benchmark;
mod comps;
mod drawdown;
mod lookback;
mod returns;
mod risk_metrics;
mod support;
mod tests;
mod timeseries;

pub use aggregation::*;
pub use backtesting::*;
pub use benchmark::*;
pub use comps::*;
pub use drawdown::*;
pub use lookback::*;
pub use returns::*;
pub use risk_metrics::*;
pub use timeseries::*;
