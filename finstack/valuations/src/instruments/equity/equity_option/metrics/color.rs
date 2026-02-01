//! Color calculator for equity options.
//!
//! Computes color (∂³V/∂S²∂t), also known as gamma decay.
//! Color measures how gamma changes with time.
//!
//! Color ≈ (Gamma(t+h) - Gamma(t)) / h
//!
//! Where Gamma(t) is computed at current time, and Gamma(t+h) at a later time.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_option::EquityOption;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Color calculator for equity options.
pub struct ColorCalculator;

impl MetricCalculator for ColorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

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

        // Compute gamma at current time
        let curves_up = bump_scalar_price(&context.curves, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up = option.value(&curves_up, as_of)?.amount();
        let curves_down = bump_scalar_price(&context.curves, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down = option.value(&curves_down, as_of)?.amount();
        let gamma_t = (pv_up - 2.0 * base_pv + pv_down) / (spot_bump * spot_bump);

        // Compute gamma at time + 1 day
        let rolled_date = as_of + time::Duration::days(time_bump_days as i64);
        let base_pv_future = option.value(&context.curves, rolled_date)?.amount();
        let curves_up_future =
            bump_scalar_price(&context.curves, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_future = option.value(&curves_up_future, rolled_date)?.amount();
        let curves_down_future =
            bump_scalar_price(&context.curves, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_future = option.value(&curves_down_future, rolled_date)?.amount();
        let gamma_t_future =
            (pv_up_future - 2.0 * base_pv_future + pv_down_future) / (spot_bump * spot_bump);

        // Color = (Gamma(t+h) - Gamma(t)) / h
        let h_years = time_bump_days / 365.25;
        let color = (gamma_t_future - gamma_t) / h_years;

        Ok(color)
    }
}
