//! Equity option instrument module: structure, pricing, and metrics.
//!
//! This module follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod parameters;
pub mod pricing;
mod types;

pub use types::EquityOption;
