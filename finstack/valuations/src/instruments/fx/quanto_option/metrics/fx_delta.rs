//! FX Delta calculator for quanto options.
//!
//! Computes FX delta (FX rate sensitivity) using finite differences:
//! bump FX exchange rate, reprice, and compute (PV_up - PV_down) / (2 * bump_size).
//!
//! # Note
//!
//! Quanto options have payoffs dependent on an equity in one currency but
//! settled in another currency. FX delta measures sensitivity to changes
//! in the FX exchange rate between these currencies.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::quanto_option::QuantoOption;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// FX Delta calculator for quanto options.
pub struct FxDeltaCalculator;

impl MetricCalculator for FxDeltaCalculator {
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

        // Get FX rate ID (if provided)
        let fx_rate_id = option.fx_rate_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "fx_rate_id not provided for quanto option".to_string(),
            })
        })?;

        // Get current FX rate for bump size calculation
        let fx_scalar = context.curves.get_price(fx_rate_id)?;
        let current_fx = match fx_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let bump_size = current_fx * bump_sizes::SPOT;

        // Bump FX rate up
        let curves_up = bump_scalar_price(context.curves.as_ref(), fx_rate_id, bump_sizes::SPOT)?;
        let pv_up = option.value(&curves_up, as_of)?.amount();

        // Bump FX rate down
        let curves_down =
            bump_scalar_price(context.curves.as_ref(), fx_rate_id, -bump_sizes::SPOT)?;
        let pv_down = option.value(&curves_down, as_of)?.amount();

        // Central difference: fx_delta = (PV_up - PV_down) / (2 * h)
        let fx_delta = (pv_up - pv_down) / (2.0 * bump_size);

        Ok(fx_delta)
    }
}
