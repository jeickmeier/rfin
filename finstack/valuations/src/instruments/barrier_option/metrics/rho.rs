//! Rho calculator for barrier options.
//!
//! Computes rho (interest rate sensitivity) using finite differences:
//! bump discount curve, reprice, and compute (PV_rate_up - PV_base) / bump_size.
//! Rho is per 1% rate move.

use crate::instruments::barrier_option::BarrierOption;
use crate::instruments::common::metrics::finite_difference::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Rho calculator for barrier options.
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &BarrierOption = context.instrument_as()?;
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

        // Bump discount curve by 1% (100bp)
        let bump_bp = 0.01; // 1% = 100bp
        let curves_bumped =
            bump_discount_curve_parallel(&context.curves, option.disc_id.as_str(), bump_bp)?;

        // Reprice with bumped curve
        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();

        // Rho = (PV_bumped - PV_base) / bump_size
        // Since we bumped by 1% (0.01), rho is per 1% rate move
        let rho = (pv_bumped - base_pv) / bump_bp;

        Ok(rho)
    }
}
