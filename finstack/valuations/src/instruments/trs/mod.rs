//! Total Return Swap instruments for equity and fixed income indices.
//!
//! Provides implementations for TRS on equity indices and fixed income indices,
//! including builders, pricing engines, and risk metrics.

mod equity;
mod fixed_income_index;
pub mod metrics;
pub mod pricing;
mod types;

// Re-export main types
pub use equity::EquityTotalReturnSwap;
pub use fixed_income_index::FIIndexTotalReturnSwap;
pub use pricing::engine::TrsEngine;
pub use types::{
    FinancingLegSpec, IndexUnderlyingParams, TotalReturnLegSpec, TrsScheduleSpec, TrsSide,
};

// Note: TRS helpers module removed - was empty
