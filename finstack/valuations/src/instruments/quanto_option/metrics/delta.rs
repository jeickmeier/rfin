//! Delta calculator for quanto options.
//!
//! Computes delta (equity spot sensitivity) using finite differences:
//! bump equity spot price up and down, reprice, and compute (PV_up - PV_down) / (2 * bump_size).

use crate::metrics::finite_difference::{bump_scalar_price, bump_sizes};
use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Delta calculator for quanto options (equity spot sensitivity).
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &QuantoOption = context.instrument_as()?;
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

        // Get current equity spot for bump size calculation
        let spot_scalar = context.curves.price(&option.spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let bump_size = current_spot * bump_sizes::SPOT;

        // Bump equity spot up
        let curves_up = bump_scalar_price(&context.curves, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up = option.npv(&curves_up, as_of)?.amount();

        // Bump equity spot down
        let curves_down = bump_scalar_price(&context.curves, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down = option.npv(&curves_down, as_of)?.amount();

        // Central difference: delta = (PV_up - PV_down) / (2 * h)
        let delta = (pv_up - pv_down) / (2.0 * bump_size);

        Ok(delta)
    }
}
