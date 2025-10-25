//! Theta calculator for total return swaps.
//!
//! Note: TRS has two types (EquityTotalReturnSwap and FIIndexTotalReturnSwap).
//! This provides theta calculators for both.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::trs::EquityTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Theta calculator for equity total return swaps.
pub struct EquityTrsThetaCalculator;

impl MetricCalculator for EquityTrsThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<EquityTotalReturnSwap>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

// Theta for fixed income index TRS is not yet registered; implement when needed.
