//! IRS floating leg PV metric.
//!
//! Discounts floating coupons projected from a forward curve, including
//! any quoted spread in basis points.
//! Only includes future cashflows (payment date > as_of date).

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;

/// PV of the floating leg of an IRS.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;

        let disc = context.curves.get_discount(&irs.float.disc_id)?;
        let fwd = context.curves.get_forward(&irs.float.fwd_id)?;
        let base = disc.base_date();
        let disc_dc = disc.day_count();

        let sched = crate::cashflow::builder::build_dates(
            irs.float.start,
            irs.float.end,
            irs.float.freq,
            irs.float.stub,
            irs.float.bdc,
            irs.float.calendar_id.as_deref(),
        );
        let dates: Vec<Date> = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Pre-compute as_of discount factor for correct discounting
        let t_as_of = disc_dc
            .year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        let mut pv = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            // Only include future cashflows
            if d <= as_of {
                prev = d;
                continue;
            }

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

            // Only call rate_period if t1 < t2 to avoid date ordering errors
            let f = if t2 > t1 {
                fwd.rate_period(t1, t2)
            } else {
                0.0
            };
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;

            // Discount from as_of for correct theta and seasoned swap handling
            let t_d = disc_dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let df_d_abs = disc.df(t_d);
            let df = if df_as_of != 0.0 {
                df_d_abs / df_as_of
            } else {
                1.0
            };

            pv += coupon * df;
            prev = d;
        }
        Ok(pv)
    }
}
