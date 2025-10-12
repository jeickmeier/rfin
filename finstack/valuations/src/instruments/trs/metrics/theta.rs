//! Theta calculator for total return swaps.
//!
//! Note: TRS has two types (EquityTotalReturnSwap and FIIndexTotalReturnSwap).
//! This provides theta calculators for both.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
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

/// Theta calculator for fixed income index total return swaps.
pub struct FIIndexTrsThetaCalculator;

impl MetricCalculator for FIIndexTrsThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<FIIndexTotalReturnSwap>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
