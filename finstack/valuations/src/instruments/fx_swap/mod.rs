//! FX Swap instrument: types, pricing, and metrics modules.
//!
//! This module follows the refactored instrument code standards:
//! - Types and builders live in `types.rs` and `parameters.rs`
//! - Pricing logic is implemented directly in the instrument
//! - Simple pricer for registry integration is in `pricer.rs`
//! - Metrics are split into focused calculators under `metrics/`

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use crate::instruments::common::parameters::FxUnderlyingParams;
pub use types::FxSwap;

// Builder provided by FinancialBuilder derive
