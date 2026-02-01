//! Interest Rate Swap pricing implementation.
//!
//! Provides NPV calculation for vanilla IRS and OIS swaps using multi-curve
//! pricing framework (separate discount and forward curves).
//!
//! # Numerical Stability
//!
//! All PV summations use Kahan compensated summation to minimize floating-point
//! rounding errors, which is critical for long-dated swaps (30Y+) with many
//! periods. This ensures deterministic, accurate results across platforms.
//!
//! # Shared Infrastructure
//!
//! This module delegates to the shared swap leg pricing infrastructure in
//! [`crate::instruments::common_impl::pricing::swap_legs`] for the core pricing logic.
//! The shared module provides Bloomberg-validated discount factor calculations
//! and Kahan summation.
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."

// Using generic pricer implementation to eliminate boilerplate

// Re-export shared swap leg pricing utilities for internal use and backward compatibility
use crate::instruments::common_impl::pricing::swap_legs::{
    add_payment_delay, robust_relative_df, FloatingLegParams, LegPeriod, BP_TO_DECIMAL,
};

// Re-export for backward compatibility with IRS metrics modules
pub(crate) use crate::instruments::common_impl::pricing::swap_legs::robust_relative_df as relative_df;

use crate::instruments::rates::irs::InterestRateSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::math::kahan_sum;
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;

use crate::instruments::rates::irs::FloatingLegCompounding;
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::{DateExt, DayCountCtx};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;

/// Convert Decimal to f64 with proper error handling.
///
/// Returns an error if the Decimal value cannot be represented as f64,
/// rather than silently defaulting to 0.0 which could mask configuration errors.
fn decimal_to_f64(value: rust_decimal::Decimal, field_name: &str) -> finstack_core::Result<f64> {
    value.to_f64().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "{} value {} cannot be converted to f64",
            field_name, value
        ))
    })
}

impl InterestRateSwap {
    /// Returns true if this swap is configured as *single-curve* compounded RFR:
    /// compounded-in-arrears and the floating index id matches the discount curve id.
    ///
    /// Note: this does **not** imply the OIS identity fast path is valid; lookback
    /// and observation shift can still require full daily compounding logic.
    pub(crate) fn is_single_curve_ois(&self) -> bool {
        matches!(
            self.float.compounding,
            FloatingLegCompounding::CompoundedInArrears { .. }
        ) && self.float.forward_curve_id == self.fixed.discount_curve_id
    }

    /// Total observation shift (business days) for compounded RFR conventions.
    ///
    /// Convention: lookback shifts observations *back* (negative), observation_shift
    /// can shift forward/back. Total shift = -lookback + observation_shift.
    fn compounded_total_shift_days(&self) -> i32 {
        match self.float.compounding {
            FloatingLegCompounding::CompoundedInArrears {
                lookback_days,
                observation_shift,
            } => -lookback_days + observation_shift.unwrap_or(0),
            _ => 0,
        }
    }

    /// Compute PV of an overnight-indexed (compounded-in-arrears) floating leg.
    #[inline]
    pub(crate) fn pv_compounded_float_leg(
        &self,
        disc: &DiscountCurve,
        proj: Option<&ForwardCurve>,
        as_of: Date,
        fixings: Option<&ScalarTimeSeries>,
    ) -> Result<f64> {
        self.pv_compounded_in_arrears_float_leg(disc, proj, as_of, fixings)
    }

    /// Compute PV of an overnight-indexed (compounded-in-arrears) floating leg.
    ///
    /// This method implements market-standard compounded RFR accrual with support
    /// for lookback and observation shift. It can be used for both single-curve
    /// OIS (where `proj == disc`) and multi-curve compounded swaps.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for discounting coupon payments
    /// * `proj` - Projection curve (forward curve or discount curve for OIS)
    /// * `as_of` - Valuation date
    ///
    /// # Errors
    ///
    /// Returns a validation error if:
    /// - Historical fixings are required but missing for observation dates before `as_of`
    /// - Calendar or date calculations fail
    /// - Numerical stability thresholds are breached
    pub(crate) fn pv_compounded_in_arrears_float_leg(
        &self,
        disc: &DiscountCurve,
        proj: Option<&ForwardCurve>,
        as_of: Date,
        fixings: Option<&ScalarTimeSeries>,
    ) -> Result<f64> {
        let schedule = crate::instruments::rates::irs::cashflow::float_leg_schedule(self)?;
        let payment_delay = self.float.payment_delay_days;
        let calendar_id = self.float.calendar_id.as_deref();
        let fixing_calendar_id = self.float.fixing_calendar_id.as_deref().or(calendar_id);

        // Resolve fixing calendar for daily stepping.
        //
        // When a calendar ID is explicitly provided, we require it to resolve successfully.
        // Silent fallback to weekday-only stepping would produce incorrect RFR accrual weights
        // (SOFR, ESTR) and mask configuration errors. If no calendar is specified, weekday
        // stepping is intentional and allowed.
        let cal = if let Some(id) = fixing_calendar_id {
            Some(CalendarRegistry::global().resolve_str(id).ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Fixing calendar '{}' not found in registry for compounded RFR swap '{}'. \
                     Load the calendar or remove fixing_calendar_id to use weekday stepping.",
                    id,
                    self.id.as_str()
                ))
            })?)
        } else {
            // No calendar specified - weekday stepping is intentional
            None
        };

        let total_shift = self.compounded_total_shift_days();

        let mut terms = Vec::new();
        let mut accrual_start = self.float.start;

        for cf in schedule
            .flows
            .iter()
            .filter(|cf| cf.kind == crate::cashflow::primitives::CFKind::FloatReset)
        {
            let accrual_end = cf.date;

            // Skip settled cashflows
            if accrual_end <= as_of {
                accrual_start = accrual_end;
                continue;
            }

            // Daily compounding logic
            let allow_fast_path = as_of <= accrual_start
                && total_shift == 0
                && proj.is_none_or(|p| disc.id() == p.id());

            let compound_factor = if allow_fast_path && proj.is_none() {
                // Single-curve discount-only fast path when no observation shifting:
                // Product of (1 + r_i * dcf_i) is exactly DF(S)/DF(E).
                1.0 / robust_relative_df(disc, accrual_start, accrual_end)?
            } else if allow_fast_path {
                // Fast path for single-curve OIS without lookback/shift:
                1.0 / robust_relative_df(disc, accrual_start, accrual_end)?
            } else {
                let mut acc = 1.0;
                let mut d = accrual_start;

                // Step through business days in the accrual period
                while d < accrual_end {
                    let next_d = if let Some(cal) = cal {
                        d.add_business_days(1, cal)?
                    } else {
                        d.add_weekdays(1)
                    };
                    let step_end = if next_d > accrual_end {
                        accrual_end
                    } else {
                        next_d
                    };

                    let dcf = self
                        .float
                        .dc
                        .year_fraction(d, step_end, DayCountCtx::default())?;

                    let obs_start = if total_shift == 0 {
                        d
                    } else if let Some(cal) = cal {
                        d.add_business_days(total_shift, cal)?
                    } else {
                        d.add_weekdays(total_shift)
                    };
                    let obs_end = if total_shift == 0 {
                        step_end
                    } else if let Some(cal) = cal {
                        step_end.add_business_days(total_shift, cal)?
                    } else {
                        step_end.add_weekdays(total_shift)
                    };

                    // Validate observation period ordering after applying shifts.
                    // Large negative shifts (e.g., lookback > period length) could invert the
                    // observation window, leading to negative year fractions or invalid rate lookups.
                    if obs_end <= obs_start {
                        return Err(finstack_core::Error::Validation(format!(
                            "Invalid observation period after applying shift: obs_start={}, obs_end={}, \
                             total_shift={} days. This may indicate lookback exceeds the daily step size \
                             or an invalid observation_shift configuration.",
                            obs_start, obs_end, total_shift
                        )));
                    }

                    // Seasoned compounded swaps: for observation dates before `as_of`,
                    // require explicit fixings (do not silently extrapolate).
                    let r = if obs_start < as_of {
                        let series = fixings.ok_or_else(|| {
                            finstack_core::Error::Validation(format!(
                                "Seasoned compounded swap requires RFR fixings for dates before as_of (missing series). \
                                 Provide ScalarTimeSeries id='FIXING:{}' with business-day observations.",
                                self.float.forward_curve_id.as_str()
                            ))
                        })?;
                        series.value_on_exact(obs_start)?
                    } else if let Some(proj) = proj {
                        let t0 = if obs_start <= proj.base_date() {
                            0.0
                        } else {
                            proj.day_count().year_fraction(
                                proj.base_date(),
                                obs_start,
                                DayCountCtx::default(),
                            )?
                        };
                        let t1 = if obs_end <= proj.base_date() {
                            0.0
                        } else {
                            proj.day_count().year_fraction(
                                proj.base_date(),
                                obs_end,
                                DayCountCtx::default(),
                            )?
                        };
                        // Use rate_period only when the time interval is meaningful.
                        // For very small intervals (< 1 day in year fraction terms), the
                        // interpolation may be noisy; fall back to spot rate.
                        const MIN_PERIOD_YF: f64 = 1.0 / 366.0; // ~1 day minimum
                        if t1 - t0 >= MIN_PERIOD_YF {
                            proj.rate_period(t0, t1)
                        } else {
                            proj.rate(t0)
                        }
                    } else {
                        // Single-curve discount-only projection: derive the implied
                        // simple rate for [obs_start, obs_end] from discount factors.
                        let df_between = disc.df_between_dates(obs_start, obs_end)?;
                        if !df_between.is_finite() || df_between <= 0.0 {
                            return Err(finstack_core::Error::Validation(format!(
                                "Invalid discount factor between observation dates ({} -> {}): df={:.3e}",
                                obs_start, obs_end, df_between
                            )));
                        }
                        let comp = 1.0 / df_between; // DF(obs_start)/DF(obs_end)

                        // Guard against pathological DCF values that could cause numerical issues.
                        // A minimum threshold of 1e-8 corresponds to ~0.3 seconds in ACT/365,
                        // which catches same-day observation scenarios from calendar misconfiguration.
                        const MIN_DCF_THRESHOLD: f64 = 1e-8;
                        if dcf < MIN_DCF_THRESHOLD {
                            return Err(finstack_core::Error::Validation(format!(
                                "Day-count fraction {:.2e} is below minimum threshold ({:.0e}). \
                                 This may indicate calendar misconfiguration causing same-day observations \
                                 or invalid date ordering ({} -> {}).",
                                dcf, MIN_DCF_THRESHOLD, d, step_end
                            )));
                        }
                        (comp - 1.0) / dcf
                    };
                    acc *= 1.0 + r * dcf;
                    d = step_end;
                }
                acc
            };

            // Coupon amount: N * [(compound_factor - 1) + spread * total_dcf]
            //
            // Per ISDA 2021 Definitions Section 4.5 (Compounding), the spread is applied as
            // simple interest on the full accrual period day count fraction, NOT compounded
            // with the index rate. This is the market standard for SOFR, SONIA, and €STR swaps.
            // Some legacy conventions (pre-2021 SONIA) applied spread on compounded amounts,
            // but ISDA 2021 standardized the simple spread approach.
            //
            // Note: alpha_total is cf.accrual_factor from builder
            let interest = self.notional.amount() * (compound_factor - 1.0);
            let spread_bp_f64 = decimal_to_f64(self.float.spread_bp, "float leg spread_bp")?;
            let spread_contrib =
                self.notional.amount() * (spread_bp_f64 * BP_TO_DECIMAL) * cf.accrual_factor;

            // Discount to payment date (holiday-aware, strict) using shared helper
            let payment_date = add_payment_delay(accrual_end, payment_delay, calendar_id)?;
            let df = robust_relative_df(disc, as_of, payment_date)?;

            terms.push((interest + spread_contrib) * df);
            accrual_start = accrual_end;
        }

        let total_pv = kahan_sum(terms);
        Ok(total_pv)
    }

    /// Compute PV of fixed leg (helper for value calculation).
    ///
    /// Discounts all future fixed coupon payments using the discount curve,
    /// applying the fixed leg's day count convention and payment schedule.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for discounting cashflows
    /// * `as_of` - Valuation date (only future cashflows are included)
    ///
    /// # Returns
    ///
    /// Present value of the fixed leg in the swap's notional currency.
    ///
    /// # Numerical Stability
    ///
    /// Uses Kahan compensated summation for accurate PV calculation on
    /// long-dated swaps with many periods (30Y+ = 60+ semi-annual payments).
    /// Delegates to the shared swap leg pricing infrastructure.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10).
    pub(crate) fn pv_fixed_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let sched = crate::instruments::rates::irs::cashflow::fixed_leg_schedule(self)?;

        // Convert cashflow schedule to LegPeriod iterator for shared pricing.
        //
        // Note: For fixed legs, `accrual_start` and `accrual_end` are both set to the payment
        // date because the actual year fraction is pre-computed by the cashflow builder and
        // stored in `cf.accrual_factor`. The shared pricing function uses `period.year_fraction`
        // directly rather than recomputing from dates. This avoids redundant date arithmetic
        // while preserving the `LegPeriod` interface for consistency with floating leg pricing.
        let periods = sched
            .flows
            .iter()
            .filter(|cf| {
                cf.kind == crate::cashflow::primitives::CFKind::Fixed
                    || cf.kind == crate::cashflow::primitives::CFKind::Stub
            })
            .map(|cf| LegPeriod {
                accrual_start: cf.date, // Placeholder; year_fraction is pre-computed
                accrual_end: cf.date,   // Placeholder; year_fraction is pre-computed
                reset_date: None,
                year_fraction: cf.accrual_factor,
            });

        // Build fixed leg params
        let params = crate::instruments::common_impl::pricing::swap_legs::FixedLegParams {
            rate: decimal_to_f64(self.fixed.rate, "fixed leg rate")?,
            day_count: self.fixed.dc,
            payment_delay_days: self.fixed.payment_delay_days,
            calendar_id: self.fixed.calendar_id.clone(),
        };

        // Use shared pricing function
        crate::instruments::common_impl::pricing::swap_legs::pv_fixed_leg(
            periods,
            self.notional.amount(),
            &params,
            disc,
            as_of,
        )
    }

    /// Compute PV of floating leg (helper for value calculation).
    ///
    /// Projects floating rate coupons using the forward curve, applies any
    /// quoted spread, and discounts to present value. This method is used for
    /// term-rate swaps (LIBOR-style, SOFR 3M) where the floating leg requires
    /// forward rate projection.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for discounting cashflows
    /// * `fwd` - Forward curve for projecting floating rates
    /// * `as_of` - Valuation date (only future cashflows are included)
    /// * `fixings` - Optional historical fixings for seasoned swaps. Required when
    ///   `reset_date < as_of` for any period; if missing, returns an error.
    ///
    /// # Returns
    ///
    /// Present value of the floating leg in the swap's notional currency.
    ///
    /// # Market Standards (ISDA 2006)
    ///
    /// - Reset dates are computed as `accrual_start - reset_lag_days`, adjusted
    ///   using the fixing calendar if specified (otherwise the payment calendar).
    /// - Forward rates are projected using the forward curve's day count convention
    ///   and base date, ensuring consistency with curve construction.
    /// - Accrual fractions use the floating leg's day count convention (e.g., ACT/360).
    /// - For seasoned swaps with past resets, historical fixings are used instead of
    ///   forward projection.
    ///
    /// # Numerical Stability
    ///
    /// Uses Kahan compensated summation for accurate PV calculation on
    /// long-dated swaps with many periods (30Y+ = 120+ quarterly payments).
    /// Delegates to the shared swap leg pricing infrastructure.
    ///
    /// # Errors
    ///
    /// Returns a validation error if:
    /// - The valuation date discount factor is below the numerical stability threshold
    /// - Historical fixings are required but not provided or missing for a reset date
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        fwd: &finstack_core::market_data::term_structures::ForwardCurve,
        as_of: Date,
        fixings: Option<&ScalarTimeSeries>,
    ) -> finstack_core::Result<f64> {
        // Build the floating-leg schedule via the shared cashflow builder so reset
        // lags, calendars, and stub handling stay centralized.
        let schedule = crate::instruments::rates::irs::cashflow::float_leg_schedule(self)?;

        // Track accrual start for period construction
        let mut accrual_start = self.float.start;

        // Convert cashflow schedule to LegPeriod iterator for shared pricing
        let periods: Vec<LegPeriod> = schedule
            .flows
            .iter()
            .filter(|cf| cf.kind == crate::cashflow::primitives::CFKind::FloatReset)
            .map(|cf| {
                let period = LegPeriod {
                    accrual_start,
                    accrual_end: cf.date,
                    reset_date: cf.reset_date,
                    year_fraction: cf.accrual_factor,
                };
                accrual_start = cf.date; // Update for next iteration
                period
            })
            .collect();

        // Build floating leg params using shared type
        let params = FloatingLegParams::full(
            decimal_to_f64(self.float.spread_bp, "float leg spread_bp")?,
            1.0,  // gearing
            true, // gearing_includes_spread
            None, // index_floor_bp
            None, // index_cap_bp
            None, // all_in_floor_bp
            None, // all_in_cap_bp
            self.float.payment_delay_days,
            self.float.calendar_id.clone(),
        );

        // Use shared pricing function
        crate::instruments::common_impl::pricing::swap_legs::pv_floating_leg(
            periods.into_iter(),
            self.notional.amount(),
            &params,
            disc,
            fwd,
            as_of,
            fixings,
        )
    }
}

/// Compute the net present value (NPV) of an interest rate swap.
///
/// Calculates the swap's mark-to-market value by computing the present value
/// of both fixed and floating legs, then taking their difference according to
/// the swap's pay/receive direction.
///
/// For OIS swaps (overnight-indexed with compounding), uses the discount-only
/// method. For term-rate swaps, projects floating rates from the forward curve.
///
/// # Arguments
///
/// * `irs` - The interest rate swap to value
/// * `context` - Market context containing discount and forward curves
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Net present value of the swap in the notional currency. Positive values
/// indicate the swap is in-the-money for the holder (based on pay/receive side).
///
/// # Errors
///
/// Returns an error if:
/// - Required curves (discount or forward) are not found in the market context
/// - Discount factor calculations fail due to numerical instability
/// - Date calculations fail
///
/// # Examples
///
/// ```text
/// use finstack_valuations::instruments::rates::irs::{InterestRateSwap, pricer};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// # use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// let irs = InterestRateSwap::example()?;
/// // Build market context with required curves
/// let mut context = MarketContext::new();
/// // ... add USD-OIS and USD-SOFR-3M curves ...
///
/// let as_of = Date::from_calendar_date(2024, Month::January, 1)
///     .map_err(|e| finstack_core::Error::Validation(format!("{}", e)))?;
///
/// let npv = pricer::compute_pv(&irs, &context, as_of)?;
/// println!("Swap NPV: {}", npv);
/// # Ok(())
/// # }
/// ```
pub(crate) fn compute_pv(
    irs: &InterestRateSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let npv_val = compute_pv_raw(irs, context, as_of)?;
    Ok(Money::new(npv_val, irs.notional.currency()))
}

/// Compute the raw Net Present Value (f64) without rounding.
pub(crate) fn compute_pv_raw(
    irs: &InterestRateSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    let disc = context.get_discount(irs.fixed.discount_curve_id.as_ref())?;
    let pv_fixed = irs.pv_fixed_leg(disc.as_ref(), as_of)?;

    // Look up historical fixings for seasoned swaps (both Simple and Compounded paths)
    let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
    let fixings = context.series(&fixings_id).ok();

    let pv_float = match irs.float.compounding {
        FloatingLegCompounding::Simple => {
            // Term-rate swap: requires forward curve for float leg pricing.
            // For seasoned swaps with past resets, historical fixings are used.
            let fwd = context.get_forward(irs.float.forward_curve_id.as_ref())?;
            irs.pv_float_leg(disc.as_ref(), fwd.as_ref(), as_of, fixings)?
        }
        FloatingLegCompounding::CompoundedInArrears { .. } => {
            // Compounded RFR swap (single-curve or multi-curve).
            //
            // For single-curve setups it is common to have only a discount curve loaded;
            // in that case we derive implied overnight forwards from the discount curve.
            let proj = if irs.is_single_curve_ois() {
                context
                    .get_forward(irs.float.forward_curve_id.as_ref())
                    .ok()
            } else {
                Some(context.get_forward(irs.float.forward_curve_id.as_ref())?)
            };
            irs.pv_compounded_float_leg(disc.as_ref(), proj.as_deref(), as_of, fixings)?
        }
    };

    let npv = match irs.side {
        crate::instruments::rates::irs::PayReceive::PayFixed => pv_float - pv_fixed,
        crate::instruments::rates::irs::PayReceive::ReceiveFixed => pv_fixed - pv_float,
    };
    Ok(npv)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCountCtx;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::ScalarTimeSeries;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::ForwardCurve;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    #[test]
    fn is_single_curve_ois_classification() {
        // Start from the example vanilla IRS (term-rate style)
        let mut irs = InterestRateSwap::example().expect("Example should construct successfully");
        assert!(
            !irs.is_single_curve_ois(),
            "Vanilla term-rate IRS with Simple compounding must not be OIS"
        );

        // Turn it into an OIS-style swap: use overnight compounding and align
        // the floating index with the fixed-leg discount curve.
        irs.float.compounding = FloatingLegCompounding::sofr();
        irs.float.forward_curve_id = irs.fixed.discount_curve_id.clone();

        assert!(
            irs.is_single_curve_ois(),
            "Swap with overnight compounding and matching index/discount curves \
             should be classified as OIS"
        );
    }

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("valid month"), d)
            .expect("valid date")
    }

    #[test]
    fn compounded_ois_seasoned_uses_fixings_and_projection() {
        use finstack_core::dates::{BusinessDayConvention, DateExt, DayCount, Tenor};

        let as_of = date(2024, 1, 10);
        let start = date(2024, 1, 2);
        let end = date(2024, 2, 2);

        let disc_id = CurveId::new("USD-OIS");
        let fwd_id = CurveId::new("USD-OIS-FWD");
        let disc_rate: f64 = 0.02;
        let df_1y = (-disc_rate).exp();
        let disc = DiscountCurve::builder(disc_id.clone())
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, df_1y)])
            .build()
            .expect("discount curve");

        let fwd_rate = 0.03;
        let fwd = ForwardCurve::builder(fwd_id.clone(), 1.0 / 12.0)
            .base_date(as_of)
            .day_count(DayCount::Act360)
            .knots(vec![(0.0, fwd_rate), (1.0, fwd_rate)])
            .build()
            .expect("forward curve");

        let fixing_rate = 0.05;
        let mut obs = Vec::new();
        let mut d = start;
        while d < as_of {
            obs.push((d, fixing_rate));
            d = d.add_weekdays(1);
        }
        let fixings = ScalarTimeSeries::new(format!("FIXING:{}", fwd_id.as_str()), obs, None)
            .expect("fixings series");

        let ctx = MarketContext::new()
            .insert_discount(disc.clone())
            .insert_forward(fwd)
            .insert_series(fixings);

        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new("OIS-SEASONED"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .side(crate::instruments::rates::irs::PayReceive::PayFixed)
            .fixed(
                crate::instruments::common_impl::parameters::legs::FixedLegSpec {
                    discount_curve_id: disc_id.clone(),
                    rate: rust_decimal::Decimal::ZERO,
                    freq: Tenor::monthly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    start,
                    end,
                    par_method: None,
                    compounding_simple: true,
                    payment_delay_days: 0,
                },
            )
            .float(
                crate::instruments::common_impl::parameters::legs::FloatLegSpec {
                    discount_curve_id: disc_id.clone(),
                    forward_curve_id: fwd_id.clone(),
                    spread_bp: rust_decimal::Decimal::ZERO,
                    freq: Tenor::monthly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    reset_lag_days: 0,
                    fixing_calendar_id: None,
                    start,
                    end,
                    compounding: FloatingLegCompounding::fedfunds(),
                    payment_delay_days: 0,
                },
            )
            .build()
            .expect("swap");

        let pv = swap.value(&ctx, as_of).expect("seasoned OIS PV");

        let mut acc = 1.0;
        let mut day = start;
        while day < end {
            let next = day.add_weekdays(1);
            let step_end = if next > end { end } else { next };
            let dcf = DayCount::Act360
                .year_fraction(day, step_end, DayCountCtx::default())
                .expect("dcf");
            let r = if day < as_of { fixing_rate } else { fwd_rate };
            acc *= 1.0 + r * dcf;
            day = step_end;
        }

        let payment_date = add_payment_delay(end, 0, None).expect("payment delay");
        let df = robust_relative_df(&disc, as_of, payment_date).expect("df");
        let expected = 1_000_000.0 * (acc - 1.0) * df;

        let diff = (pv.amount() - expected).abs();
        // Allow small tolerance for day count/business day handling differences
        assert!(
            diff < 0.01,
            "Seasoned OIS PV should match fixing/projection compounding, diff={}",
            diff
        );
    }

    #[test]
    fn compounded_ois_with_lookback_uses_discount_only_projection_when_forward_missing() {
        // Single-curve OIS setups often only load the discount curve. We still want
        // lookback/shift to take effect (i.e., not be silently ignored).
        let as_of = date(2024, 1, 1);
        let start = date(2024, 2, 1);
        let end = date(2024, 5, 1);

        let disc_id = CurveId::new("USD-OIS");
        let disc = DiscountCurve::builder(disc_id.clone())
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)])
            .build()
            .expect("discount curve");

        let ctx = MarketContext::new().insert_discount(disc);

        let swap_no_lookback = InterestRateSwap::builder()
            .id(InstrumentId::new("OIS-NO-LOOKBACK"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(crate::instruments::rates::irs::PayReceive::PayFixed)
            .fixed(
                crate::instruments::common_impl::parameters::legs::FixedLegSpec {
                    discount_curve_id: disc_id.clone(),
                    rate: rust_decimal::Decimal::try_from(0.03).expect("valid"),
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    start,
                    end,
                    par_method: None,
                    compounding_simple: true,
                    payment_delay_days: 0,
                },
            )
            .float(
                crate::instruments::common_impl::parameters::legs::FloatLegSpec {
                    discount_curve_id: disc_id.clone(),
                    forward_curve_id: disc_id.clone(), // single-curve: same id as discount
                    spread_bp: rust_decimal::Decimal::ZERO,
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    reset_lag_days: 0,
                    start,
                    end,
                    compounding: FloatingLegCompounding::fedfunds(), // lookback=0
                    fixing_calendar_id: None,
                    payment_delay_days: 0,
                },
            )
            .build()
            .expect("swap");

        let mut swap_lookback = swap_no_lookback.clone();
        swap_lookback.id = InstrumentId::new("OIS-LOOKBACK-2D");
        swap_lookback.float.compounding = FloatingLegCompounding::sofr(); // lookback=2

        // Both should price without a forward curve present.
        let pv0 = swap_no_lookback.value(&ctx, as_of).expect("pv no lookback");
        let pv2 = swap_lookback.value(&ctx, as_of).expect("pv lookback");

        // The lookback should have a non-zero effect under a non-flat curve.
        assert!(
            (pv0.amount() - pv2.amount()).abs() > 1e-8,
            "Expected PV to differ with lookback; pv0={}, pv2={}",
            pv0.amount(),
            pv2.amount()
        );
    }

    /// Tests that compounded OIS pricing fails when fixing_calendar_id is specified but not
    /// found in the CalendarRegistry.
    ///
    /// This validates the fix for the silent fallback to weekday stepping issue identified
    /// in the quant code review. For RFR compounding (SOFR, ESTR), incorrect calendar
    /// handling produces material accrual errors.
    #[test]
    fn compounded_ois_fails_when_calendar_missing() {
        let as_of = date(2024, 1, 1);
        let start = date(2024, 2, 1);
        let end = date(2024, 5, 1);

        let disc_id = CurveId::new("USD-OIS");
        let disc = DiscountCurve::builder(disc_id.clone())
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)])
            .build()
            .expect("discount curve");

        let ctx = MarketContext::new().insert_discount(disc);

        // Create swap with an explicitly specified but non-existent fixing calendar
        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new("OIS-MISSING-CAL"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(crate::instruments::rates::irs::PayReceive::PayFixed)
            .fixed(
                crate::instruments::common_impl::parameters::legs::FixedLegSpec {
                    discount_curve_id: disc_id.clone(),
                    rate: rust_decimal::Decimal::try_from(0.03).expect("valid"),
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    start,
                    end,
                    par_method: None,
                    compounding_simple: true,
                    payment_delay_days: 0,
                },
            )
            .float(
                crate::instruments::common_impl::parameters::legs::FloatLegSpec {
                    discount_curve_id: disc_id.clone(),
                    forward_curve_id: disc_id.clone(),
                    spread_bp: rust_decimal::Decimal::ZERO,
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    reset_lag_days: 0,
                    start,
                    end,
                    compounding: FloatingLegCompounding::sofr(),
                    // This calendar ID does not exist in the registry
                    fixing_calendar_id: Some("NONEXISTENT-CALENDAR-XYZ".to_string()),
                    payment_delay_days: 0,
                },
            )
            .build()
            .expect("swap");

        // Pricing should fail with a validation error about the missing calendar
        let result = swap.value(&ctx, as_of);
        assert!(
            result.is_err(),
            "Expected pricing to fail when fixing_calendar_id is specified but not found"
        );

        let err = result.expect_err("Expected pricing to fail");
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("NONEXISTENT-CALENDAR-XYZ") || err_msg.contains("calendar"),
            "Error message should mention the missing calendar ID, got: {}",
            err_msg
        );
    }

    /// Tests that compounded OIS pricing succeeds with weekday stepping when no fixing
    /// calendar is specified.
    ///
    /// When fixing_calendar_id is None, weekday-only stepping (Mon-Fri) is intentional
    /// and the pricing should proceed without error.
    #[test]
    fn compounded_ois_succeeds_with_weekday_stepping_when_no_calendar() {
        let as_of = date(2024, 1, 1);
        let start = date(2024, 2, 1);
        let end = date(2024, 5, 1);

        let disc_id = CurveId::new("USD-OIS");
        let disc = DiscountCurve::builder(disc_id.clone())
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)])
            .build()
            .expect("discount curve");

        let ctx = MarketContext::new().insert_discount(disc);

        // Create swap with NO fixing_calendar_id (intentional weekday stepping)
        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new("OIS-NO-CALENDAR"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(crate::instruments::rates::irs::PayReceive::PayFixed)
            .fixed(
                crate::instruments::common_impl::parameters::legs::FixedLegSpec {
                    discount_curve_id: disc_id.clone(),
                    rate: rust_decimal::Decimal::try_from(0.03).expect("valid"),
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    start,
                    end,
                    par_method: None,
                    compounding_simple: true,
                    payment_delay_days: 0,
                },
            )
            .float(
                crate::instruments::common_impl::parameters::legs::FloatLegSpec {
                    discount_curve_id: disc_id.clone(),
                    forward_curve_id: disc_id.clone(),
                    spread_bp: rust_decimal::Decimal::ZERO,
                    freq: finstack_core::dates::Tenor::quarterly(),
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    reset_lag_days: 0,
                    start,
                    end,
                    compounding: FloatingLegCompounding::sofr(),
                    // No fixing_calendar_id - intentional weekday stepping
                    fixing_calendar_id: None,
                    payment_delay_days: 0,
                },
            )
            .build()
            .expect("swap");

        // Pricing should succeed when no calendar is specified
        let result = swap.value(&ctx, as_of);
        assert!(
            result.is_ok(),
            "Expected pricing to succeed with weekday stepping when no calendar specified, got: {:?}",
            result.err()
        );

        // Verify we get a reasonable PV (non-zero, finite)
        let pv = result.expect("Pricing should succeed");
        assert!(
            pv.amount().is_finite(),
            "PV should be finite, got: {}",
            pv.amount()
        );
    }

    /// Test OIS floating leg PV matches analytical identity: PV_float = N × (DF(start) - DF(end))
    ///
    /// For single-curve OIS with no lookback, no observation shift, and no spread,
    /// the compounded floating leg PV should exactly equal the discount factor identity.
    /// This is a fundamental property of OIS pricing that must hold.
    ///
    /// # References
    ///
    /// - Hull, J.C. "Options, Futures, and Other Derivatives", Chapter 7
    /// - The identity follows from: ∏(1 + r_i × dcf_i) = DF(start)/DF(end)
    ///   when r_i is derived from the discount curve.
    #[test]
    fn ois_floating_leg_matches_analytical_identity() {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};

        let as_of = date(2024, 1, 1);
        let start = date(2024, 3, 1);
        let end = date(2024, 6, 1);

        // Create a non-flat curve to make the test meaningful
        let disc_id = CurveId::new("USD-OIS");
        let disc = DiscountCurve::builder(disc_id.clone())
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.9925), // ~3% rate
                (0.5, 0.9850),  // ~3% rate
                (1.0, 0.9650),  // ~3.5% rate
            ])
            .build()
            .expect("discount curve");

        let _ctx = MarketContext::new().insert_discount(disc.clone());

        // Create an OIS swap with NO lookback, NO observation shift, NO spread.
        // These are the conditions under which the identity is exact.
        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new("OIS-IDENTITY-TEST"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(crate::instruments::rates::irs::PayReceive::PayFixed)
            .fixed(
                crate::instruments::common_impl::parameters::legs::FixedLegSpec {
                    discount_curve_id: disc_id.clone(),
                    rate: rust_decimal::Decimal::ZERO, // Zero fixed rate for this test
                    freq: Tenor::quarterly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    start,
                    end,
                    par_method: None,
                    compounding_simple: true,
                    payment_delay_days: 0, // No payment delay for exact identity
                },
            )
            .float(
                crate::instruments::common_impl::parameters::legs::FloatLegSpec {
                    discount_curve_id: disc_id.clone(),
                    forward_curve_id: disc_id.clone(), // Single-curve: forward = discount
                    spread_bp: rust_decimal::Decimal::ZERO, // No spread
                    freq: Tenor::quarterly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                    reset_lag_days: 0,
                    start,
                    end,
                    compounding: FloatingLegCompounding::CompoundedInArrears {
                        lookback_days: 0,        // No lookback
                        observation_shift: None, // No observation shift
                    },
                    fixing_calendar_id: None,
                    payment_delay_days: 0, // No payment delay
                },
            )
            .build()
            .expect("swap");

        // Calculate floating leg PV using the pricer
        let pv_float = swap
            .pv_compounded_float_leg(&disc, None, as_of, None)
            .expect("float leg PV");

        // Calculate analytical identity: N × (DF(start) - DF(end))
        // Note: This is the PV of receiving the floating leg payments
        let df_start = disc.df_between_dates(as_of, start).expect("df_start");
        let df_end = disc.df_between_dates(as_of, end).expect("df_end");
        let expected_pv = swap.notional.amount() * (df_start - df_end);

        // The identity should hold to high precision (< 1 currency unit on 10MM)
        let error = (pv_float - expected_pv).abs();
        assert!(
            error < 1.0,
            "OIS floating leg identity violated!\n\
             Computed PV:  {:.6}\n\
             Expected PV:  {:.6}\n\
             Error:        {:.6}\n\
             DF(start):    {:.6}\n\
             DF(end):      {:.6}",
            pv_float,
            expected_pv,
            error,
            df_start,
            df_end
        );

        // Additional check: error should be < 0.01% of notional
        let relative_error = error / swap.notional.amount();
        assert!(
            relative_error < 1e-6,
            "OIS identity relative error too large: {:.2e}",
            relative_error
        );
    }
}
