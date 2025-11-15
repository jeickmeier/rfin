//! Charm calculator for equity options.
//!
//! Computes charm (∂²V/∂S∂t), also known as delta decay.
//! Charm measures how delta changes with time.
//!
//! Charm ≈ (Delta(t+h) - Delta(t)) / h
//!
//! Where Delta(t) is computed by bumping spot at current time,
//! and Delta(t+h) is computed by bumping spot at a later time.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Charm calculator for equity options.
pub struct CharmCalculator;

impl MetricCalculator for CharmCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
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

        // Get current spot
        let spot_scalar = context.curves.price(&option.spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let spot_bump = current_spot * bump_sizes::SPOT;
        let time_bump_days = 1.0; // 1 day

        // Compute delta at current time
        let curves_up = bump_scalar_price(&context.curves, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up = option.npv(&curves_up, as_of)?.amount();
        let curves_down = bump_scalar_price(&context.curves, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down = option.npv(&curves_down, as_of)?.amount();
        let delta_t = (pv_up - pv_down) / (2.0 * spot_bump);

        // Compute delta at time + 1 day
        let rolled_date = as_of + time::Duration::days(time_bump_days as i64);
        let curves_up_future =
            bump_scalar_price(&context.curves, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_future = option.npv(&curves_up_future, rolled_date)?.amount();
        let curves_down_future =
            bump_scalar_price(&context.curves, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_future = option.npv(&curves_down_future, rolled_date)?.amount();
        let delta_t_future = (pv_up_future - pv_down_future) / (2.0 * spot_bump);

        // Charm = (Delta(t+h) - Delta(t)) / h
        // h is in days, convert to years for proper scaling
        let h_years = time_bump_days / 365.25;
        let charm = (delta_t_future - delta_t) / h_years;

        Ok(charm)
    }
}
