//! Interest Rate Swap (IRS) instrument module.
//!
//! Mirrors the modern instrument layout used across valuations with clearly
//! separated `types`, `pricing`, and `metrics` modules. Public re‑exports keep
//! the external API surface stable while enabling internal evolution.

pub mod metrics;
pub mod pricing;
mod risk;
mod types;

pub use types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};
pub use pricing::engine::IrsEngine;
