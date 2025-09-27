//! FX Spot module: structure, pricing, metrics, and type re-exports.
//!
//! Simplified structure after refactoring:
//! - `types`: instrument data structures with integrated pricing methods
//! - `pricer`: registry integration (moved from pricing subdirectory)
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricer;
mod types;

pub use pricer::SimpleFxSpotDiscountingPricer;
pub use types::FxSpot;
