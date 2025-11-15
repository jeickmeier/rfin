//! Delta calculator for FX barrier options.
//!
//! Computes delta (FX spot sensitivity) using finite differences:
//! bump FX spot rate up and down, reprice, and compute (PV_up - PV_down) / (2 * bump_size).
//!
//! # Note on Barrier Discontinuities
//!
//! FX barrier options exhibit discontinuous deltas near the barrier level,
//! similar to standard barrier options.

use crate::instruments::fx_barrier_option::FxBarrierOption;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Delta calculator for FX barrier options.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxBarrierOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Get current FX spot for bump size calculation
        let spot_scalar = context.curves.price(&option.fx_spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let bump_size = current_spot * bump_sizes::SPOT;

        // Bump FX spot up
        let curves_up = bump_scalar_price(&context.curves, &option.fx_spot_id, bump_sizes::SPOT)?;
        let pv_up = option.npv(&curves_up, as_of)?.amount();

        // Bump FX spot down
        let curves_down =
            bump_scalar_price(&context.curves, &option.fx_spot_id, -bump_sizes::SPOT)?;
        let pv_down = option.npv(&curves_down, as_of)?.amount();

        // Central difference: delta = (PV_up - PV_down) / (2 * h)
        let delta = (pv_up - pv_down) / (2.0 * bump_size);

        Ok(delta)
    }
}
