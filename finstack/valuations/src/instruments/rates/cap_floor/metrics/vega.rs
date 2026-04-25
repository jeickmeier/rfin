//! Vega calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::instruments::rates::cap_floor::{CapFloor, CapFloorVolType};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator (model-consistent vega per 1% vol, aggregated for caps/floors).
///
/// Dispatches to the appropriate model based on `vol_type`:
/// - `Lognormal`: Black-76 vega = F·n(d₁)·√T / 100
/// - `ShiftedLognormal`: Black-76 vega on shifted rates
/// - `Normal`: Bachelier vega = n(d)·√T / 100
///
/// For Normal vol, the 1% bump is in absolute rate terms (e.g., 1bp normal vol).
pub(crate) struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CapFloor = context.instrument_as()?;
        let strike = option.strike_f64()?;
        let vol_type = option.vol_type;
        let vol_shift = option.resolved_vol_shift();
        super::common::aggregate_over_caplets(option, context, |forward, sigma, t_fix| {
            match vol_type {
                CapFloorVolType::Lognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                        strike, forward, sigma, t_fix,
                    )
                }
                CapFloorVolType::ShiftedLognormal => {
                    crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                        strike + vol_shift,
                        forward + vol_shift,
                        sigma,
                        t_fix,
                    )
                }
                CapFloorVolType::Normal => {
                    crate::instruments::rates::cap_floor::pricing::normal::vega_per_pct(
                        strike, forward, sigma, t_fix,
                    )
                }
                CapFloorVolType::Auto => {
                    if forward > 0.0 && strike > 0.0 {
                        crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                            strike, forward, sigma, t_fix,
                        )
                    } else {
                        crate::instruments::rates::cap_floor::pricing::normal::vega_per_pct(
                            strike, forward, sigma, t_fix,
                        )
                    }
                }
            }
        })
    }
}
