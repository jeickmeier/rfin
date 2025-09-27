//! CDS Option instrument module: structure, pricing, and metrics.
//!
//! Follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricer`: pricing implementation and engine
//! - `metrics`: metric calculators and registry hook (split by file)

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use types::CdsOption;
