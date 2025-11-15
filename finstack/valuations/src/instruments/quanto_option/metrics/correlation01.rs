//! Correlation sensitivity calculator for quanto options.
//!
//! Computes correlation sensitivity using finite differences:
//! bump correlation between equity and FX, reprice, and compute (PV_corr_up - PV_base) / bump_size.
//! Correlation01 is per 1% correlation move.

use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Correlation01 calculator for quanto options.
pub struct Correlation01Calculator;

impl MetricCalculator for Correlation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &QuantoOption = context.instrument_as()?;
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

        // Bump correlation by 1%
        let bump = bump_sizes::CORRELATION;
        let new_correlation = (option.correlation + bump).clamp(-1.0, 1.0);

        // Create option with bumped correlation
        let mut option_bumped = option.clone();
        option_bumped.correlation = new_correlation;

        // Reprice with bumped correlation
        let pv_bumped = option_bumped.npv(&context.curves, as_of)?.amount();

        // Correlation01 = (PV_bumped - PV_base) / bump_size
        let correlation01 = (pv_bumped - base_pv) / bump;

        Ok(correlation01)
    }
}
