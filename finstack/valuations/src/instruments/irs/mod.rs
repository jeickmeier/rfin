//! Interest Rate Swap (IRS) instrument module.
//!
//! Follows the simplified instrument layout with `types`, `pricer`, and `metrics` modules.
//! Public re‑exports keep the external API surface stable while enabling internal evolution.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, ParRateMethod, PayReceive};
