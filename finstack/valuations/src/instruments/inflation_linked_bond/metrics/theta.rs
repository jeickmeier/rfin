//! Theta calculator for inflation-linked bonds.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<InflationLinkedBond>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
