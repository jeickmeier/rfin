//! IRS fixed leg PV metric.
//!
//! Discounts fixed coupons on the fixed leg using the discount curve.
//! Only includes future cashflows (payment date > as_of date).

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;

/// PV of the fixed leg of an IRS.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;
        let disc_dc = disc.day_count();

        let sched = crate::cashflow::builder::build_dates(
            irs.fixed.start,
            irs.fixed.end,
            irs.fixed.freq,
            irs.fixed.stub,
            irs.fixed.bdc,
            irs.fixed.calendar_id.as_deref(),
        );
        let dates: Vec<Date> = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Pre-compute as_of discount factor for correct discounting
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
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

            let yf = irs
                .fixed
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let coupon = irs.notional.amount() * irs.fixed.rate * yf;

            // Discount from as_of for correct theta and seasoned swap handling
            let t_d = disc_dc
                .year_fraction(
                    disc.base_date(),
                    d,
                    finstack_core::dates::DayCountCtx::default(),
                )
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
