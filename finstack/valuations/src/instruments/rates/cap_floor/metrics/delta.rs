//! Delta calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::rates::cap_floor::{InterestRateOption, RateOptionType};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Delta calculator (Black model forward delta, aggregated for caps/floors)
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            let is_cap = matches!(
                option.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            crate::instruments::rates::cap_floor::pricing::black::delta(
                is_cap,
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
