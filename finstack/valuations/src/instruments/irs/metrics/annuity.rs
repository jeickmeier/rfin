//! Fixed leg annuity calculation for interest rate swaps.
//!
//! Computes the sum of discounted accrual factors on the fixed leg, which
//! represents the present value of receiving $1 per unit coupon on each
//! fixed payment date.
//!
//! # Definition
//!
//! ```text
//! Annuity = Σ α_i × DF(T_i)
//! ```
//!
//! where:
//! - `α_i` = accrual factor for period i (from day count convention)
//! - `DF(T_i)` = discount factor to payment date i (relative to valuation date)
//! - Sum includes only future cashflows (T_i > as_of)
//!
//! # Applications
//!
//! The annuity is a fundamental quantity used in:
//! - **Par rate calculation**: `Par Rate = PV_float / (Notional × Annuity)`
//! - **DV01 approximation**: Change in swap value for 1bp rate change
//! - **Duration metrics**: Effective duration and modified duration
//! - **Risk scaling**: Converting PV sensitivities to rate sensitivities
//!
//! # Implementation Notes
//!
//! For IRS fixed legs we always treat coupons as simple interest; the
//! compounding configuration affects coupon accrual (handled in cashflow
//! builders), not the annuity weight calculation itself.
//!
//! # Numerical Stability
//!
//! Uses Kahan compensated summation to minimize floating-point rounding errors,
//! which is critical for long-dated swaps (30Y+) with many periods. This ensures
//! deterministic, accurate results across platforms.
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.
//! - Tuckman, B., & Serrat, A. (2011). *Fixed Income Securities*. Chapter 4.
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::math::kahan_sum;

/// Fixed-leg annuity calculator for interest rate swaps.
///
/// Computes the present value of $1 paid per unit coupon on each fixed
/// payment date, discounted to the valuation date. This is a fundamental
/// quantity used in par rate and risk calculations.
pub struct AnnuityCalculator;

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;

        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;

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

        // Collect terms for Kahan summation to ensure numerical stability
        // for long-dated swaps with many periods (30Y+ = 120+ quarterly payments)
        let mut terms = Vec::with_capacity(dates.len());
        let mut prev = dates[0];

        for &d in &dates[1..] {
            // Only include future cashflows
            if d <= as_of {
                prev = d;
                continue;
            }

            let yf = irs.fixed.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            // Use shared helper - handles epsilon validation and relative DF calculation
            let df = crate::instruments::irs::pricer::relative_df(&disc, as_of, d)?;

            // For IRS fixed legs we always treat coupons as simple interest; the
            // compounding configuration affects coupon accrual, not the annuity
            // weight, so the annuity is just sum(alpha * DF).
            terms.push(yf * df);
            prev = d;
        }

        // Use Kahan compensated summation for numerical stability
        // This is critical for 30Y swaps where naive summation can accumulate
        // significant floating-point errors across 120+ periods
        let annuity = kahan_sum(terms);

        // Return annuity in dollar terms
        // Note: Just return sum(yf * df) without scaling - the raw sum is what's needed
        // for par rate calculations and other metrics
        Ok(annuity)
    }
}
