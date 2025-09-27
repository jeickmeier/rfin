//! Equity spot instrument implementation.
//!
//! Layout follows the standard instrument structure used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricer`: pricing implementation and engine
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricer;
mod types;

pub use types::Equity;
pub use types::Ticker;
