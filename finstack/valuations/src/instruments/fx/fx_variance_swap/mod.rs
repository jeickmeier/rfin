//! FX variance swap instrument and replication pricer.
//!
//! Prices variance swaps using standard replication with OTM FX options,
//! accounting for domestic/foreign rate differentials.

/// Pricer implementations for FX variance swaps.
pub mod pricer;
mod types;

pub use types::{FxVarianceSwap, FxVarianceSwapBuilder, PayReceive};
