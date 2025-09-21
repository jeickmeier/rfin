//! Zero-coupon Inflation Swap module.
//!
//! Structure follows the standard instrument layout across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod pricing;
mod types;

pub use types::{InflationSwap, PayReceiveInflation};
