//! IRS annuity metric.
//!
//! Computes the sum of discounted accrual factors on the fixed leg, commonly
//! used for par rate calculations and risk analytics.
//!
//! The annuity represents `sum(alpha_i * DF_i)` for future cashflows only, with
//! discount factors computed relative to the valuation date. For IRS fixed legs
//! we always treat coupons as simple interest; fixed-leg compounding settings do
//! not change the annuity weights.

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;

/// Minimum threshold for discount factor values to avoid numerical instability.
/// Same as in pricer.rs to ensure consistency across IRS calculations.
const DF_EPSILON: f64 = 1e-10;

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

        // Guard against near-zero discount factors for numerical stability
        if df_as_of.abs() < DF_EPSILON {
            return Err(finstack_core::error::Error::Validation(format!(
                "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
                 This may indicate extreme rate scenarios or very long time horizons.",
                df_as_of, DF_EPSILON
            )));
        }

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
            // df_as_of already validated above, safe to divide
            let df = df_d_abs / df_as_of;

            // For IRS fixed legs we always treat coupons as simple interest; the
            // compounding configuration affects coupon accrual, not the annuity
            // weight, so the annuity is just sum(alpha * DF).
            annuity += yf * df;
            prev = d;
        }
        // Return annuity in dollar terms
        // Note: Just return sum(yf * df) without scaling - the raw sum is what's needed
        // for par rate calculations and other metrics
        Ok(annuity)
    }
}
