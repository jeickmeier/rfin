//! Generic metric calculators to reduce duplication across instruments.
//!
//! This module provides generic implementations of common metrics that can be
//! parameterized over different instrument types, eliminating the need for
//! near-identical calculator implementations across instruments.

pub mod bucketed_cs01;
pub mod bucketed_dv01;
pub mod finite_difference;
pub mod has_equity_underlying;
pub mod has_pricing_overrides;
pub mod pv;
pub mod theta_utils;
pub mod vol_expiry_helpers;

#[cfg(test)]
mod tests;

pub mod fd_greeks;

pub use bucketed_cs01::{GenericBucketedCs01, HasHazardCurve};
pub use bucketed_dv01::{GenericBucketedDv01, GenericBucketedDv01WithContext, HasDiscountCurve};
pub use fd_greeks::{GenericFdDelta, GenericFdGamma};
pub use finite_difference::bump_sizes;
pub use has_equity_underlying::HasEquityUnderlying;
pub use has_pricing_overrides::HasPricingOverrides;
pub use pv::GenericPv;
pub use theta_utils::GenericTheta;
