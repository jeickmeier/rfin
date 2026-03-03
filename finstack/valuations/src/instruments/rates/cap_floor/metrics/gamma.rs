//! Gamma calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::rates::cap_floor::{CapFloorVolType, InterestRateOption};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Gamma calculator (model-consistent forward gamma, aggregated for caps/floors).
///
/// Dispatches to the appropriate model based on `vol_type`:
/// - `Lognormal`: Black-76 gamma = n(d₁) / (F·σ·√T)
/// - `ShiftedLognormal`: Black-76 gamma on shifted rates
/// - `Normal`: Bachelier gamma = n(d) / (σ·√T)
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let strike = option.strike_f64()?;
        let vol_type = option.vol_type;
        let vol_shift = option.resolved_vol_shift();
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            match vol_type {
                CapFloorVolType::Lognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::gamma(
                        strike, forward, sigma, t_fix,
                    )
                }
                CapFloorVolType::ShiftedLognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::gamma(
                        strike + vol_shift,
                        forward + vol_shift,
                        sigma,
                        t_fix,
                    )
                }
                CapFloorVolType::Normal => {
                    crate::instruments::rates::cap_floor::pricing::normal::gamma(
                        strike, forward, sigma, t_fix,
                    )
                }
            }
        })
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
