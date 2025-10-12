//! Theta metric for `CdsOption`.

use crate::instruments::cds_option::CdsOption;
use crate::instruments::common::metrics::theta_utils;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Theta calculator for credit options on CDS spreads.
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<CdsOption>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
