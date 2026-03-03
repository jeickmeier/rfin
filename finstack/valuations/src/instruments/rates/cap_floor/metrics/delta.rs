//! Delta calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::rates::cap_floor::{CapFloorVolType, InterestRateOption, RateOptionType};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Delta calculator (model-consistent forward delta, aggregated for caps/floors).
///
/// Dispatches to the appropriate model based on `vol_type`:
/// - `Lognormal`: Black-76 delta = N(d₁)
/// - `ShiftedLognormal`: Black-76 delta on shifted rates
/// - `Normal`: Bachelier delta = N(d)
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let strike = option.strike_f64()?;
        let vol_type = option.vol_type;
        let vol_shift = option.resolved_vol_shift();
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            let is_cap = matches!(
                option.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            match vol_type {
                CapFloorVolType::Lognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::delta(
                        is_cap, strike, forward, sigma, t_fix,
                    )
                }
                CapFloorVolType::ShiftedLognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::delta(
                        is_cap,
                        strike + vol_shift,
                        forward + vol_shift,
                        sigma,
                        t_fix,
                    )
                }
                CapFloorVolType::Normal => {
                    crate::instruments::rates::cap_floor::pricing::normal::delta(
                        is_cap, strike, forward, sigma, t_fix,
                    )
                }
            }
        })
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
