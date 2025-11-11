//! Theta calculator for private markets funds.

use crate::instruments::private_markets_fund::PrivateMarketsFund;
use crate::metrics::theta_utils;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<PrivateMarketsFund>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

