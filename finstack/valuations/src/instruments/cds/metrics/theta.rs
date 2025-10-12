//! Theta calculator for credit default swaps.

use crate::instruments::cds::CreditDefaultSwap;
use crate::instruments::common::metrics::theta_utils;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<CreditDefaultSwap>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
