//! Theta calculator for structured credit instruments.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<StructuredCredit>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
