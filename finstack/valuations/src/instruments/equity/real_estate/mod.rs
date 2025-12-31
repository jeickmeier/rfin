//! Real estate asset valuation instruments.
//!
//! Supports DCF and direct capitalization approaches for single-asset valuation.

/// Pricer for real estate assets.
pub(crate) mod metrics;
pub(crate) mod pricer;
mod types;

pub use pricer::RealEstateAssetDiscountingPricer;
pub use types::{RealEstateAsset, RealEstateValuationMethod};
