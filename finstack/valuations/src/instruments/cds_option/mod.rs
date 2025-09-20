//! CDS Option instrument module: structure, pricing, and metrics.
//!
//! Follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook (split by file)

pub mod pricing;
pub mod metrics;
pub mod parameters;
mod types;

pub use types::CdsOption;
