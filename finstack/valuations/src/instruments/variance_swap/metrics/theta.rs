//! Theta calculator for variance swaps.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::variance_swap::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<VarianceSwap>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
