//! Rho calculator for range accrual instruments.
//!
//! Computes rho (interest rate sensitivity) using finite differences:
//! bump discount curve, reprice, and compute (PV_rate_up - PV_base) / bump_size.
//! Rho is per 1% rate move.

use crate::instruments::range_accrual::RangeAccrual;
use crate::instruments::common::metrics::finite_difference::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Rho calculator for range accrual instruments.
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &RangeAccrual = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        let final_date = instrument.observation_dates.last().copied().unwrap_or(as_of);
        let t = instrument
            .day_count
            .year_fraction(as_of, final_date, finstack_core::dates::DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Bump discount curve by 1% (100bp)
        let bump_bp = 0.01;
        let curves_bumped = bump_discount_curve_parallel(&context.curves, instrument.disc_id.as_str(), bump_bp)?;

        // Reprice with bumped curve
        let pv_bumped = instrument.npv(&curves_bumped, as_of)?.amount();

        // Rho = (PV_bumped - PV_base) / bump_size
        let rho = (pv_bumped - base_pv) / bump_bp;

        Ok(rho)
    }
}

