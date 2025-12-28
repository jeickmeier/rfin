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
//! # Numerical Stability
//!
//! - Uses Kahan compensated summation for PV calculations
//! - Guards against division by near-zero annuity with ANNUITY_EPSILON threshold
//! - Returns descriptive errors for degenerate cases (expired swaps, invalid schedules)
//!
//! # References
//!
//! - **ISDA 2006 Definitions**: Section 7.1 - Par Swap Rates
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."

use crate::instruments::common::pricing::swap_legs::ANNUITY_EPSILON;
use crate::instruments::irs::{FloatingLegCompounding, ParRateMethod};
use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Returns true if the DiscountRatio identity is valid for this IRS configuration.
///
/// Market-standard prerequisites for using `(DF(start)-DF(end))/annuity`:
/// - Unseasoned: `as_of <= start`
/// - Single-curve: forward curve id == discount curve id
/// - Term-style floating leg (`Simple`) with **no spread**
/// - No payment delays on either leg (otherwise numerator dates don't align with payment DFs)
fn discount_ratio_allowed(irs: &InterestRateSwap, as_of: Date) -> bool {
    if as_of > irs.fixed.start {
        return false;
    }
    if irs.float.forward_curve_id != irs.fixed.discount_curve_id {
        return false;
    }
    if !matches!(irs.float.compounding, FloatingLegCompounding::Simple) {
        return false;
    }
    if !irs.float.spread_bp.is_zero() {
        return false;
    }
    if irs.fixed.payment_delay_days != 0 || irs.float.payment_delay_days != 0 {
        return false;
    }
    true
}

/// Par rate calculator for interest rate swaps.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    #[allow(clippy::expect_used)] // dates.last() is infallible: len >= 2 checked above
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;

        let method = irs.fixed.par_method.unwrap_or(ParRateMethod::ForwardBased);

        // For compounded swaps, we always use the forward-based (PV-based) method
        // to ensure all RFR-specific conventions (lookback, shift) are captured.
        if matches!(
            irs.float.compounding,
            FloatingLegCompounding::CompoundedInArrears { .. }
        ) {
            return par_rate_pv_based(irs, context, &disc);
        }

        match method {
            ParRateMethod::ForwardBased => par_rate_pv_based(irs, context, &disc),
            ParRateMethod::DiscountRatio => {
                let as_of = context.as_of;
                if !discount_ratio_allowed(irs, as_of) {
                    // Safer default: fall back to PV-based par rate when identity prerequisites do not hold.
                    return par_rate_pv_based(irs, context, &disc);
                }
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
                    return Err(finstack_core::error::Error::Validation(
                        "Par rate calculation failed: swap schedule has fewer than 2 dates.".into(),
                    ));
                }

                if as_of > dates[0] {
                    return par_rate_pv_based(irs, context, &disc);
                }

                let p0 = crate::instruments::irs::pricer::relative_df(&disc, as_of, dates[0])?;
                let pn = crate::instruments::irs::pricer::relative_df(
                    &disc,
                    as_of,
                    *dates.last().expect("Dates should not be empty"),
                )?;
                let num = p0 - pn;

                let annuity = context
                    .computed
                    .get(&MetricId::Annuity)
                    .copied()
                    .unwrap_or(0.0);
                if annuity.abs() < ANNUITY_EPSILON {
                    return Err(finstack_core::error::Error::Validation(
                        "Annuity near zero".into(),
                    ));
                }

                // Note: DiscountRatio usually assumes zero spread on the floating leg.
                // If there's a spread, we must use the PV-based method.
                if !irs.float.spread_bp.is_zero() {
                    return par_rate_pv_based(irs, context, &disc);
                }

                Ok(num / annuity)
            }
        }
    }
}

/// Par rate calculation based on PV of the floating leg.
///
/// Refactored to reuse the pricer's own floating leg PV logic for perfect consistency.
fn par_rate_pv_based(
    irs: &InterestRateSwap,
    ctx: &MetricContext,
    disc: &DiscountCurve,
) -> finstack_core::Result<f64> {
    let as_of = ctx.as_of;
    let annuity = ctx.computed.get(&MetricId::Annuity).copied().unwrap_or(0.0);

    if annuity.abs() < ANNUITY_EPSILON {
        return Err(finstack_core::error::Error::Validation(format!(
            "Par rate calculation failed: annuity ({:.2e}) is below numerical stability \
             threshold ({:.2e}).",
            annuity, ANNUITY_EPSILON
        )));
    }

    // Reuse the pricer's PV logic based on compounding type
    let pv_float = match irs.float.compounding {
        FloatingLegCompounding::Simple => {
            let fwd = ctx.curves.get_forward(&irs.float.forward_curve_id)?;
            irs.pv_float_leg(disc, fwd.as_ref(), as_of)?
        }
        FloatingLegCompounding::CompoundedInArrears { .. } => {
            let proj = if irs.is_single_curve_ois() {
                ctx.curves.get_forward(&irs.float.forward_curve_id).ok()
            } else {
                Some(ctx.curves.get_forward(&irs.float.forward_curve_id)?)
            };
            let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
            let fixings = ctx.curves.series(&fixings_id).ok();
            irs.pv_compounded_float_leg(disc, proj.as_deref(), as_of, fixings)?
        }
    };

    // Par rate = float_pv / (notional * annuity)
    Ok(pv_float / (irs.notional.amount() * annuity))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("valid month"), d)
            .expect("valid date")
    }

    #[test]
    fn discount_ratio_allowed_requires_single_curve_no_spread_no_payment_delay_unseasoned() {
        let disc = CurveId::new("DISC");
        let fwd = CurveId::new("FWD");
        let start = date(2024, 1, 10);
        let end = date(2025, 1, 10);
        let as_of = date(2024, 1, 1);

        let irs = InterestRateSwap::builder()
            .id(InstrumentId::new("IRS"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .side(crate::instruments::irs::PayReceive::PayFixed)
            .fixed(crate::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: disc.clone(),
                rate: rust_decimal::Decimal::try_from(0.03).expect("valid"),
                freq: Tenor::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start,
                end,
                par_method: Some(ParRateMethod::DiscountRatio),
                compounding_simple: true,
                payment_delay_days: 0,
            })
            .float(crate::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: disc.clone(),
                forward_curve_id: fwd.clone(), // multi-curve
                spread_bp: rust_decimal::Decimal::ZERO,
                freq: Tenor::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start,
                end,
                compounding: FloatingLegCompounding::Simple,
                fixing_calendar_id: None,
                payment_delay_days: 0,
            })
            .build()
            .expect("irs");

        assert!(
            !discount_ratio_allowed(&irs, as_of),
            "multi-curve not allowed"
        );

        let mut irs2 = irs.clone();
        irs2.float.forward_curve_id = disc.clone();
        assert!(discount_ratio_allowed(&irs2, as_of), "single-curve allowed");

        irs2.float.spread_bp = rust_decimal::Decimal::try_from(5.0).expect("valid");
        assert!(!discount_ratio_allowed(&irs2, as_of), "spread disallowed");

        let mut irs3 = irs2.clone();
        irs3.float.spread_bp = rust_decimal::Decimal::ZERO;
        irs3.float.payment_delay_days = 2;
        assert!(
            !discount_ratio_allowed(&irs3, as_of),
            "payment delay disallowed"
        );

        let seasoned_as_of = date(2024, 2, 1);
        let mut irs4 = irs.clone();
        irs4.float.forward_curve_id = disc.clone();
        assert!(
            !discount_ratio_allowed(&irs4, seasoned_as_of),
            "seasoned disallowed"
        );
    }
}
