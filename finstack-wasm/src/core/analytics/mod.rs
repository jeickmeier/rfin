//! WASM bindings for the performance analytics module.
//!
//! Maps to `finstack_analytics` in Rust, exposed as `analytics` in the
//! flat WASM API surface.

pub mod benchmark;
pub mod consecutive;
pub mod drawdown;
pub mod lookback;
pub mod performance;
pub mod returns;
pub mod risk_metrics;
