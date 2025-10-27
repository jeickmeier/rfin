//! Generic metric calculators to reduce duplication across instruments.
//!
//! This module provides generic implementations of common metrics that can be
//! parameterized over different instrument types, eliminating the need for
//! near-identical calculator implementations across instruments.

pub mod bucketed_dv01;
pub mod pv;
pub mod theta_utils;

#[cfg(test)]
mod tests;

pub use bucketed_dv01::{GenericBucketedDv01, GenericBucketedDv01WithContext, HasDiscountCurve};
pub use pv::GenericPv;
pub use theta_utils::GenericTheta;
