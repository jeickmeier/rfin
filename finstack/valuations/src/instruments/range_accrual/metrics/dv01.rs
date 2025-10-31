//! DV01 calculator for range accrual instruments.
//!
//! Note: Range accrual also has GenericBucketedDv01WithContext registered,
//! which provides bucketed interest rate risk. This calculator provides
//! the parallel shift DV01.

use crate::instruments::range_accrual::RangeAccrual;
use crate::instruments::common::metrics::finite_difference::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for range accrual instruments.
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
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

        // Bump discount curve by 1bp (0.0001)
        let bump_bp = 0.0001;
        let curves_bumped = bump_discount_curve_parallel(&context.curves, instrument.disc_id.as_str(), bump_bp)?;

        // Reprice with bumped curve
        let pv_bumped = instrument.npv(&curves_bumped, as_of)?.amount();

        // DV01 = PV_change per 1bp rate move
        let dv01 = pv_bumped - base_pv;

        Ok(dv01)
    }
}

