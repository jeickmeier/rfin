//! Foreign Rho calculator for quanto options.
//!
//! Computes foreign rho (foreign interest rate sensitivity) via finite differences:
//! bump foreign discount curve by +1bp, reprice, and return PV_change.
//!
//! Units & sign:
//! - Rho is per +1bp parallel discount move
//! - Rho = PV(rate + 1bp) − PV(base)
//! - Positive Rho means the instrument gains value when rates go up

use crate::instruments::common::traits::Instrument;
use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Foreign Rho calculator for quanto options.
pub struct ForeignRhoCalculator;

impl MetricCalculator for ForeignRhoCalculator {
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

        // Bump discount curve. Default to 1bp (0.0001) if not overridden.
        let bump_bp = option.pricing_overrides.rho_bump_decimal.unwrap_or(0.0001);
        let curves_bumped = bump_discount_curve_parallel(
            &context.curves,
            &option.foreign_discount_curve_id,
            bump_bp,
        )?;

        // Reprice with bumped curve
        let pv_bumped = option.value(&curves_bumped, as_of)?.amount();

        // Rho = PV(rate + 1bp) − PV(base)
        let rho = pv_bumped - base_pv;

        Ok(rho)
    }
}
