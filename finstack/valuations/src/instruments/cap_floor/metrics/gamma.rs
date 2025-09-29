//! Gamma calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result};

/// Gamma calculator (Black model forward gamma, aggregated for caps/floors)
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            crate::instruments::cap_floor::pricing::black::gamma(
                option.strike_rate,
                forward,
                sigma,
                t_fix,
            )
        })
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
