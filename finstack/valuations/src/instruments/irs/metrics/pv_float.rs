//! IRS floating leg PV metric.
//!
//! Discounts floating coupons projected from a forward curve, including
//! any quoted spread in basis points.

use crate::instruments::irs::types::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};
use finstack_core::F;

/// PV of the floating leg of an IRS.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs: &InterestRateSwap = context.instrument_as()?;

        let disc = context.curves.get::<DiscountCurve>(irs.float.disc_id.clone())?;
        let fwd = context
            .curves
            .get::<ForwardCurve>(irs.float.fwd_id.clone())?;
        let base = disc.base_date();

        let sched = crate::cashflow::builder::build_dates(
            irs.float.start,
            irs.float.end,
            irs.float.freq,
            irs.float.stub,
            irs.float.bdc,
            irs.float.calendar_id,
        );
        let dates: Vec<Date> = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        let mut pv = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let t1 = irs
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = irs
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = irs
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;
            let df = disc.df_on_date_curve(d);
            pv += coupon * df;
            prev = d;
        }
        Ok(pv)
    }
}
