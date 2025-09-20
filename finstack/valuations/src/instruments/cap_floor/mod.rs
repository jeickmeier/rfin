//! Interest rate option (Cap/Floor/Caplet/Floorlet) module.
//!
//! Follows the standard instrument layout used across valuations (see `cds` as a reference):
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod parameters;
pub mod pricing;
mod types;

pub use types::{InterestRateOption, RateOptionType};
