//! Interest rate swap par rate calculation.
//!
//! Computes the fixed rate that sets the swap PV to zero given market curves.
//!
//! # Calculation Methods
//!
//! ## Forward-Based Method (Default)
//!
//! Computes par rate as:
//! ```text
//! Par Rate = PV_float / (Notional × Annuity)
//! ```
//!
//! where:
//! - `PV_float` = sum of discounted projected floating coupons
//! - `Annuity` = sum of discounted accrual factors on fixed leg
//!
//! This method works for both seasoned and unseasoned swaps.
//!
//! ## Discount Ratio Method
//!
//! Uses the closed-form identity:
//! ```text
//! Par Rate = (DF(start) - DF(end)) / Annuity
//! ```
//!
//! This method is exact only for unseasoned swaps where `as_of <= start_date`.
//! For seasoned swaps, use the forward-based method instead.
//!
//! # References
//!
//! - **ISDA 2006 Definitions**: Section 7.1 - Par Swap Rates
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.

use crate::instruments::{irs::ParRateMethod, InterestRateSwap};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::dates::Date;

/// Basis points to decimal conversion factor.
const BP_TO_DECIMAL: f64 = 1e-4;

/// Par rate calculator for interest rate swaps.
///
/// Computes the fixed rate that makes the swap's net present value equal to zero.
/// Supports both forward-based (works for seasoned swaps) and discount-ratio
/// (exact for unseasoned swaps only) methods.
///
/// # Dependencies
///
/// Requires the `Annuity` metric to be computed first.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;

        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;
        let method = irs.fixed.par_method.unwrap_or(ParRateMethod::ForwardBased);
        match method {
            ParRateMethod::ForwardBased => par_rate_forward_based(irs, &*context, &disc),
            ParRateMethod::DiscountRatio => {
                // (P(as_of,T0) - P(as_of,Tn)) / Sum alpha_i P(as_of,Ti)
                // This formulation is only exact for unseasoned swaps where
                // `as_of` is on or before the fixed leg start date.
                let as_of = context.as_of;
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

                // For seasoned swaps (`as_of` after the start date), fall back to the
                // forward-based method, which is robust for live trades.
                if as_of > dates[0] {
                    return par_rate_forward_based(irs, &*context, &disc);
                }

                // Numerator: P(as_of,T0) - P(as_of,Tn)
                let p0 = crate::instruments::irs::pricer::relative_df(&disc, as_of, dates[0])?;
                let pn = crate::instruments::irs::pricer::relative_df(
                    &disc,
                    as_of,
                    *dates.last().expect("Dates should not be empty"),
                )?;
                let num = p0 - pn;

                // Denominator: Sum alpha_i P(as_of,Ti) for future cashflows
                let mut den = 0.0;
                let mut prev = dates[0];
                for &d in &dates[1..] {
                    // Only include future cashflows
                    if d <= as_of {
                        prev = d;
                        continue;
                    }

                    let alpha = irs
                        .fixed
                        .dc
                        .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
                    let p = crate::instruments::irs::pricer::relative_df(&disc, as_of, d)?;
                    den += alpha * p;
                    prev = d;
                }
                if den == 0.0 {
                    return Ok(0.0);
                }
                Ok(num / den)
            }
        }
    }
}

/// Forward-based par rate calculation used for both the default method and
/// as a fallback when the discount-ratio method is not applicable (e.g. for
/// seasoned swaps where `as_of` is after the fixed leg start date).
fn par_rate_forward_based(
    irs: &InterestRateSwap,
    ctx: &MetricContext,
    disc: &DiscountCurve,
) -> finstack_core::Result<f64> {
    // float PV / (N * annuity)
    let fwd = ctx.curves.get_forward(&irs.float.forward_curve_id)?;
    let as_of = ctx.as_of;
    let base = disc.base_date();

    // Annuity is sum(yf*df) in years
    let annuity = ctx
        .computed
        .get(&MetricId::Annuity)
        .copied()
        .unwrap_or(0.0); // This is fine - it's from a hashmap, not a calculation
    if annuity == 0.0 {
        return Ok(0.0);
    }

    let fs = crate::cashflow::builder::build_dates(
        irs.float.start,
        irs.float.end,
        irs.float.freq,
        irs.float.stub,
        irs.float.bdc,
        irs.float.calendar_id.as_deref(),
    );
    let schedule: Vec<Date> = fs.dates;
    if schedule.len() < 2 {
        return Ok(0.0);
    }

    let mut pv = 0.0;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        // Only include future cashflows
        if d <= as_of {
            prev = d;
            continue;
        }

        // Times to accrual boundaries measured from curve base date. Clamp to
        // zero if the boundary is on or before the base date to avoid invalid
        // date ranges when the curve is built with `base_date = as_of`.
        let t1 = if prev <= base {
            0.0
        } else {
            irs.float.dc.year_fraction(
                base,
                prev,
                finstack_core::dates::DayCountCtx::default(),
            )?
        };
        let t2 = if d <= base {
            0.0
        } else {
            irs.float.dc.year_fraction(
                base,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )?
        };

        let yf = irs
            .float
            .dc
            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;

        // Only call rate_period if t1 < t2 to avoid date ordering errors
        let f = if t2 > t1 {
            fwd.rate_period(t1, t2)
        } else {
            0.0
        };
        let rate = f + (irs.float.spread_bp * BP_TO_DECIMAL);
        let coupon = irs.notional.amount() * rate * yf;

        // Use shared helper - handles epsilon validation and relative DF calculation
        let df = crate::instruments::irs::pricer::relative_df(disc, as_of, d)?;

        pv += coupon * df;
        prev = d;
    }

    // Par rate = float_pv / (notional * annuity)
    // Annuity is sum(yf*df), so this gives: pv / (notional * sum(yf*df))
    Ok(pv / (irs.notional.amount() * annuity))
}
