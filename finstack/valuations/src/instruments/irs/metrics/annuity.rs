//! IRS annuity metric.
//!
//! Computes the sum of discounted accrual factors on the fixed leg, commonly
//! used for par rate calculations and risk analytics.
//!
//! The annuity represents sum(year_fraction[i] * discount_factor[i]) for future
//! cashflows only, with discount factors computed relative to the valuation date.

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;

/// Calculates the fixed-leg annuity for an IRS.
pub struct AnnuityCalculator;

impl MetricCalculator for AnnuityCalculator {
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

        let mut annuity = 0.0;
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

            if irs.fixed.compounding_simple {
                annuity += yf * df;
            } else {
                // Treat each period as compounded: accumulate (1 + r*alpha) weights approximated via DF spacing
                annuity += yf * df; // keep same weight; compounding affects coupon accrual, not DF weight here
            }
            prev = d;
        }
        // Return annuity in dollar terms
        // Note: Just return sum(yf * df) without scaling - the raw sum is what's needed
        // for par rate calculations and other metrics
        Ok(annuity)
    }
}
