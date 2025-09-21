//! FX Spot module: structure, pricing, metrics, and type re-exports.
//!
//! Layout follows the standard instrument structure used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing engine and facade
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricing;
mod types;

pub use types::FxSpot;
