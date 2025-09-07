//! Zero-coupon Inflation Swap (boilerplate implementation).
//!
//! This module adds a minimal scaffold for an inflation swap instrument so it
//! can participate in the unified pricing and metrics framework. Valuation
//! logic is intentionally minimal (returns zero) until completed.

mod builder;
pub mod metrics;
mod types;

pub use types::{InflationSwap, PayReceiveInflation};
