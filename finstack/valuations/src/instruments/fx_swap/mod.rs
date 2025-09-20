//! FX Swap instrument: types, pricing, and metrics modules.
//!
//! This module follows the instrument code standards used across valuations:
//! - Types and builders live in `types.rs` and `parameters.rs`
//! - Pricing entrypoints are implemented under `pricing/`
//! - Metrics are split into focused calculators under `metrics/`

pub mod metrics;
pub mod parameters;
pub mod pricing;
mod types;

pub use types::{FxSwap, FxUnderlyingParams};

// Builder provided by FinancialBuilder derive
