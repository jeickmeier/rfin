//! Volga calculator for quanto options.
//!
//! Computes volga (equity volatility sensitivity) using finite differences.

use crate::instruments::common::traits::Instrument;
use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for quanto options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &QuantoOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump = bump_sizes::VOLATILITY;

        let curves_vol_up =
            bump_surface_vol_absolute(&context.curves, option.vol_surface_id.as_str(), vol_bump)?;
        let pv_vol_up = option.value(&curves_vol_up, as_of)?.amount();

        let curves_vol_down =
            bump_surface_vol_absolute(&context.curves, option.vol_surface_id.as_str(), -vol_bump)?;
        let pv_vol_down = option.value(&curves_vol_down, as_of)?.amount();

        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump * vol_bump);
        Ok(volga)
    }
}
