//! Theta calculator for private markets funds.

use crate::instruments::equity::pe_fund::PrivateMarketsFund;
use crate::metrics::{generic_theta_calculator, MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        generic_theta_calculator::<PrivateMarketsFund>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
