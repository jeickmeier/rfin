//! IRS annuity metric.
//!
//! Computes the sum of discounted accrual factors on the fixed leg, commonly
//! used for par rate calculations and risk analytics.

use crate::instruments::irs::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::F;

/// Calculates the fixed-leg annuity for an IRS.
pub struct AnnuityCalculator;

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs: &InterestRateSwap = context.instrument_as()?;

        let disc = context.curves.get::<DiscountCurve>(irs.fixed.disc_id)?;
        let _ = disc.base_date();

        let sched = crate::cashflow::builder::build_dates(
            irs.fixed.start,
            irs.fixed.end,
            irs.fixed.schedule.freq,
            irs.fixed.schedule.stub,
            irs.fixed.schedule.bdc,
            irs.fixed.schedule.calendar_id,
        );
        let dates: Vec<Date> = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        let mut annuity = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let yf = irs
                .fixed
                .schedule
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let df = disc.df_on_date_curve(d);
            if irs.fixed.compounding_simple {
                annuity += yf * df;
            } else {
                // Treat each period as compounded: accumulate (1 + r*alpha) weights approximated via DF spacing
                annuity += yf * df; // keep same weight; compounding affects coupon accrual, not DF weight here
            }
            prev = d;
        }
        Ok(annuity)
    }
}
