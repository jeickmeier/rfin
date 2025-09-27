//! Zero-coupon Inflation Swap module.
//!
//! Structure follows the simplified instrument layout:
//! - `types`: instrument data structures with pricing methods
//! - `pricer`: simplified registry pricer
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricer;
mod types;

pub use pricer::SimpleInflationSwapDiscountingPricer;
pub use types::{InflationSwap, PayReceiveInflation};
