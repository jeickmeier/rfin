//! Delta calculator for commodity options (Black-76).

use crate::instruments::commodity_option::CommodityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::math::special_functions::norm_cdf;
use finstack_core::Result;

/// Delta calculator for commodity options.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;

        let t = option
            .day_count
            .year_fraction(as_of, option.expiry, DayCountCtx::default())?
            .max(0.0);
        if t <= 0.0 {
            let forward = option.forward_price(&context.curves, as_of)?;
            let intrinsic = match option.option_type {
                crate::instruments::OptionType::Call => {
                    if forward > option.strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                crate::instruments::OptionType::Put => {
                    if forward < option.strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
            return Ok(intrinsic * option.quantity * option.multiplier);
        }

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let surface = context.curves.surface_ref(option.vol_surface_id.as_str())?;
            surface.value_clamped(t, option.strike)
        };
        if sigma <= 0.0 {
            return Ok(0.0);
        }

        let forward = option.forward_price(&context.curves, as_of)?;
        let disc = context
            .curves
            .get_discount_ref(option.discount_curve_id.as_str())?;
        let df = disc.try_df_between_dates(as_of, option.expiry)?;

        let d1 = crate::instruments::common::models::d1_black76(forward, option.strike, sigma, t);
        let nd1 = norm_cdf(d1);
        let unit_delta = match option.option_type {
            crate::instruments::OptionType::Call => df * nd1,
            crate::instruments::OptionType::Put => df * (nd1 - 1.0),
        };

        Ok(unit_delta * option.quantity * option.multiplier)
    }
}
