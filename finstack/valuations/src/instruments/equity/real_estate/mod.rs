//! Real estate asset valuation instruments.
//!
//! Supports DCF and direct capitalization approaches for single-asset valuation.

/// Pricer for real estate assets.
pub mod metrics;
pub mod pricer;
mod types;

pub use pricer::RealEstateAssetDiscountingPricer;
pub use types::{RealEstateAsset, RealEstateValuationMethod};
