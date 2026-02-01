//! Volga calculator for CMS options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.
//! Volga measures how vega changes with volatility.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cms_option::CmsOption;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for CMS options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CmsOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        let final_date = option.fixing_dates.last().copied().unwrap_or(as_of);
        let t = option.day_count.year_fraction(
            as_of,
            final_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_surface_id = match option.vol_surface_id.as_ref() {
            Some(id) => id,
            None => {
                return Err(finstack_core::Error::from(
                    finstack_core::InputError::NotFound {
                        id: "vol_surface_id not provided for CMS option".to_string(),
                    },
                ));
            }
        };

        let vol_bump = bump_sizes::VOLATILITY;

        let curves_vol_up =
            bump_surface_vol_absolute(&context.curves, vol_surface_id.as_str(), vol_bump)?;
        let pv_vol_up = option.value(&curves_vol_up, as_of)?.amount();

        let curves_vol_down =
            bump_surface_vol_absolute(&context.curves, vol_surface_id.as_str(), -vol_bump)?;
        let pv_vol_down = option.value(&curves_vol_down, as_of)?.amount();

        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump * vol_bump);
        Ok(volga)
    }
}
