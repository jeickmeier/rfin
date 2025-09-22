//! IRS fixed leg PV metric.
//!
//! Discounts fixed coupons on the fixed leg using the discount curve.

use crate::instruments::irs::types::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::F;

/// PV of the fixed leg of an IRS.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let disc = context.curves.get::<DiscountCurve>(irs.fixed.disc_id)?;

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

        let mut pv = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let yf = irs
                .fixed
                .schedule
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let coupon = irs.notional.amount() * irs.fixed.rate * yf;
            let df = disc.df_on_date_curve(d);
            pv += coupon * df;
            prev = d;
        }
        Ok(pv)
    }
}
