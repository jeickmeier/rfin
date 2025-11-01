//! DV01 calculator for FX barrier options.
//!
//! Computes DV01 (dollar value of a basis point) using finite differences:
//! bump discount curve by 1bp, reprice, and compute PV_change.

use crate::instruments::common::metrics::finite_difference::bump_discount_curve_parallel;
use crate::instruments::fx_barrier_option::FxBarrierOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for FX barrier options.
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxBarrierOption = context.instrument_as()?;
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

        // Bump discount curve by 1bp (0.0001)
        let bump_bp = 0.0001;
        let curves_bumped =
            bump_discount_curve_parallel(&context.curves, &option.disc_id, bump_bp)?;

        // Reprice with bumped curve
        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();

        // DV01 = PV_change per 1bp rate move
        let dv01 = pv_bumped - base_pv;

        Ok(dv01)
    }
}
