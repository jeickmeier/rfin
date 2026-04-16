//! Residual Risk Add-On (RRAO) computation.
//!
//! RRAO = sum_i(notional_i * weight_i)
//!
//! Exotic underlyings (longevity, weather, correlation): 1.0% of notional.
//! Other residual risks (gap, behavioral): 0.1% of notional.

use super::types::RraoPosition;

/// Weight applied to exotic underlying instruments.
pub const RRAO_EXOTIC_WEIGHT: f64 = 0.01;

/// Weight applied to other residual risk instruments.
pub const RRAO_OTHER_WEIGHT: f64 = 0.001;

/// Compute the Residual Risk Add-On.
pub fn rrao_charge(positions: &[RraoPosition]) -> f64 {
    positions
        .iter()
        .map(|p| {
            let weight = if p.is_exotic {
                RRAO_EXOTIC_WEIGHT
            } else {
                RRAO_OTHER_WEIGHT
            };
            p.notional.abs() * weight
        })
        .sum()
}
