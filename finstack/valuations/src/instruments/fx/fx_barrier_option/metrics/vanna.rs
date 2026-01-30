//! Vanna calculator for FX barrier options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Note: FX barrier options exhibit discontinuous vanna near barrier levels.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_barrier_option::FxBarrierOption;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for FX barrier options.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxBarrierOption = context.instrument_as()?;
        let as_of = context.as_of;

        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let spot_scalar = context.curves.price(&option.fx_spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let spot_bump = current_spot * bump_sizes::SPOT;
        let vol_bump = bump_sizes::VOLATILITY;
        // Bump vol up
        let curves_vol_up =
            bump_surface_vol_absolute(&context.curves, option.fx_vol_id.as_str(), vol_bump)?;
        let curves_up_vol_up =
            bump_scalar_price(&curves_vol_up, &option.fx_spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_up = option.value(&curves_up_vol_up, as_of)?.amount();
        let curves_down_vol_up =
            bump_scalar_price(&curves_vol_up, &option.fx_spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_up = option.value(&curves_down_vol_up, as_of)?.amount();
        let delta_vol_up = (pv_up_vol_up - pv_down_vol_up) / (2.0 * spot_bump);

        // Bump vol down
        let curves_vol_down =
            bump_surface_vol_absolute(&context.curves, option.fx_vol_id.as_str(), -vol_bump)?;
        let curves_up_vol_down =
            bump_scalar_price(&curves_vol_down, &option.fx_spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_down = option.value(&curves_up_vol_down, as_of)?.amount();
        let curves_down_vol_down =
            bump_scalar_price(&curves_vol_down, &option.fx_spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_down = option.value(&curves_down_vol_down, as_of)?.amount();
        let delta_vol_down = (pv_up_vol_down - pv_down_vol_down) / (2.0 * spot_bump);

        let vanna = (delta_vol_up - delta_vol_down) / (2.0 * vol_bump);
        Ok(vanna)
    }
}
