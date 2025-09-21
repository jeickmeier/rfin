//! Equity spot instrument implementation.
//!
//! Layout follows the standard instrument structure used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing engine and facade
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricing;
mod types;
pub mod underlying;

pub use types::Equity;
pub use types::Ticker;
