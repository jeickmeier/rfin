//! Total Return Swap instruments for equity and fixed income indices.
//!
//! Provides implementations for TRS on equity indices and fixed income indices,
//! including builders, pricing engines, and risk metrics.

mod equity;
mod fixed_income_index;
pub mod metrics;
mod types;
pub mod parameters;

// Re-export main types
pub use equity::EquityTotalReturnSwap;
pub use fixed_income_index::FIIndexTotalReturnSwap;
pub use types::{FinancingLegSpec, TotalReturnLegSpec, TrsEngine, TrsScheduleSpec, TrsSide};
