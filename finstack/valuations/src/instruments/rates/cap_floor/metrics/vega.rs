//! Vega calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::rates::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Vega calculator (Black model vega per 1% vol, aggregated for caps/floors)
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let strike = option.strike_rate_f64()?;
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                strike, forward, sigma, t_fix,
            )
        })
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
