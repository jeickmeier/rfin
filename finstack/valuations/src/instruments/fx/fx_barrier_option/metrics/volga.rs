//! Volga calculator for FX barrier options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.

use crate::instruments::fx_barrier_option::FxBarrierOption;
use crate::metrics::bump_sizes;
use crate::metrics::scale_surface;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for FX barrier options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxBarrierOption = context.instrument_as()?;
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
            scale_surface(&context.curves, option.fx_vol_id.as_str(), 1.0 + vol_bump)?;
        let pv_vol_up = option.npv(&curves_vol_up, as_of)?.amount();

        let curves_vol_down =
            scale_surface(&context.curves, option.fx_vol_id.as_str(), 1.0 - vol_bump)?;
        let pv_vol_down = option.npv(&curves_vol_down, as_of)?.amount();

        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump * vol_bump);
        Ok(volga)
    }
}
