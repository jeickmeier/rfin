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
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."

// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

use crate::instruments::irs::InterestRateSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::math::kahan_sum;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::instruments::irs::FloatingLegCompounding;
use crate::instruments::irs::dates::add_payment_delay;
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::dates::{DayCountCtx, DateExt};

/// Minimum threshold for discount factor values to avoid numerical instability.
///
/// Set to 1e-10 to protect against division by near-zero discount factors
/// that can arise from extreme rate scenarios or very long time horizons.
/// This aligns with ISDA stress testing requirements for rates ranging
/// from -10% to +50%.
const DF_EPSILON: f64 = 1e-10;

/// Basis points to decimal conversion factor.
const BP_TO_DECIMAL: f64 = 1e-4;

/// Compute discount factor at `target` relative to `as_of`, with numerical stability guard.
///
/// This helper centralizes the pattern of:
/// 1. Computing year fractions from base_date to as_of and target
/// 2. Getting absolute discount factors
/// 3. Validating as_of DF against DF_EPSILON
/// 4. Returning relative DF = DF(target) / DF(as_of)
///
/// # Arguments
///
/// * `disc` - Discount curve for pricing
/// * `as_of` - Valuation date (denominator for relative discounting)
/// * `target` - Target payment date (numerator for relative discounting)
///
/// # Returns
///
/// Discount factor from `as_of` to `target` (DF(target) / DF(as_of)).
/// For seasoned instruments this represents the proper discount factor for
/// cashflows occurring after the valuation date.
///
/// # Errors
///
/// Returns a validation error if:
/// - Year fraction calculation fails
/// - The as_of discount factor is below DF_EPSILON threshold (1e-10),
///   which can occur in extreme rate scenarios or very long time horizons
///
/// # Examples
///
/// ```ignore
/// // Note: relative_df is a private helper function used internally
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_valuations::instruments::irs::pricer::relative_df;
///
/// let curve = build_test_curve();
/// let as_of = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
/// let target = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
///
/// let df = relative_df(&curve, as_of, target)?;
/// assert!(df > 0.0 && df <= 1.0);
/// ```
pub(in crate::instruments::irs) fn relative_df(
    disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
    as_of: Date,
    target: Date,
) -> Result<f64> {
    let df_as_of = disc.try_df_on_date_curve(as_of)?;

    // Guard against invalid/near-zero discount factors for numerical stability and no-arb.
    if !df_as_of.is_finite() {
        return Err(finstack_core::error::Error::Validation(
            "Valuation date discount factor is not finite.".into(),
        ));
    }
    // Discount factors must be strictly positive under standard discounting assumptions.
    if df_as_of <= DF_EPSILON {
        return Err(finstack_core::error::Error::Validation(format!(
            "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
             This may indicate extreme rate scenarios or very long time horizons.",
            df_as_of, DF_EPSILON
        )));
    }

    let df = disc.try_df_between_dates(as_of, target)?;
    if !df.is_finite() {
        return Err(finstack_core::error::Error::Validation(
            "Discount factor between dates is not finite.".into(),
        ));
    }
    if df <= 0.0 {
        return Err(finstack_core::error::Error::Validation(format!(
            "Discount factor between dates is non-positive (df={:.3e}) which is non-physical.",
            df
        )));
    }
    Ok(df)
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
    ) -> Result<Money> {
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
    /// - The valuation date falls inside an accrual period (seasoned swaps not yet supported for compounding)
    /// - Calendar or date calculations fail
    /// - Numerical stability thresholds are breached
    pub(crate) fn pv_compounded_in_arrears_float_leg(
        &self,
        disc: &DiscountCurve,
        proj: Option<&ForwardCurve>,
        as_of: Date,
        fixings: Option<&ScalarTimeSeries>,
    ) -> Result<Money> {
        let schedule = crate::instruments::irs::cashflow::float_leg_schedule(self)?;
        let payment_delay = self.float.payment_delay_days;
        let calendar_id = self.float.calendar_id.as_deref();
        let fixing_calendar_id = self.float.fixing_calendar_id.as_deref().or(calendar_id);

        // Resolve fixing calendar for daily stepping.
        //
        // Default behavior: if the calendar is missing, fall back to weekday-only stepping
        // (Mon-Fri). This avoids silently switching to calendar-day arithmetic, while still
        // allowing pricing to proceed in environments where holiday calendars are not loaded.
        let cal = fixing_calendar_id.and_then(|id| CalendarRegistry::global().resolve_str(id));
        if cal.is_none() {
            tracing::warn!(
                swap_id = %self.id.as_str(),
                calendar_id = fixing_calendar_id.unwrap_or(""),
                "Fixing calendar not found for compounding; falling back to weekday-only stepping (Mon-Fri)"
            );
        }

        let total_shift = self.compounded_total_shift_days();

        let mut terms = Vec::new();
        let mut accrual_start = self.float.start;

        for cf in schedule.flows.iter().filter(|cf| cf.kind == crate::cashflow::primitives::CFKind::FloatReset) {
            let accrual_end = cf.date;

            // Skip settled cashflows
            if accrual_end <= as_of {
                accrual_start = accrual_end;
                continue;
            }

            // Daily compounding logic
            let compound_factor = if proj.is_none() && total_shift == 0 {
                // Single-curve discount-only fast path when no observation shifting:
                // Product of (1 + r_i * dcf_i) is exactly DF(S)/DF(E).
                1.0 / relative_df(disc, accrual_start, accrual_end)?
            } else if proj.is_some()
                && disc.id() == proj.expect("checked").id()
                && total_shift == 0
            {
                // Fast path for single-curve OIS without lookback/shift:
                1.0 / relative_df(disc, accrual_start, accrual_end)?
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
                    let step_end = if next_d > accrual_end { accrual_end } else { next_d };
                    
                    let dcf = self.float.dc.year_fraction(d, step_end, DayCountCtx::default())?;
                    
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
                    
                    // Seasoned compounded swaps: for observation dates before `as_of`,
                    // require explicit fixings (do not silently extrapolate).
                    let r = if obs_start < as_of {
                        let series = fixings.ok_or_else(|| {
                            finstack_core::error::Error::Validation(format!(
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
                        if t1 > t0 {
                            proj.rate_period(t0, t1)
                        } else {
                            proj.rate(t0)
                        }
                    } else {
                        // Single-curve discount-only projection: derive the implied
                        // simple rate for [obs_start, obs_end] from discount factors.
                        let df_between = disc.try_df_between_dates(obs_start, obs_end)?;
                        if !df_between.is_finite() || df_between <= 0.0 {
                            return Err(finstack_core::error::Error::Validation(format!(
                                "Invalid discount factor between observation dates ({} -> {}): df={:.3e}",
                                obs_start, obs_end, df_between
                            )));
                        }
                        let comp = 1.0 / df_between; // DF(obs_start)/DF(obs_end)
                        if dcf <= 0.0 {
                            return Err(finstack_core::error::Error::Validation(
                                "Non-positive day-count fraction encountered in compounding step.".into(),
                            ));
                        }
                        (comp - 1.0) / dcf
                    };
                    acc *= 1.0 + r * dcf;
                    d = step_end;
                }
                acc
            };

            // Coupon amount: N * [(compound_factor - 1) + spread * total_dcf]
            // Note: alpha_total is cf.accrual_factor from builder
            let interest = self.notional.amount() * (compound_factor - 1.0);
            let spread_contrib = self.notional.amount() * (self.float.spread_bp * BP_TO_DECIMAL) * cf.accrual_factor;
            
            // Discount to payment date (holiday-aware)
            let payment_date = add_payment_delay(accrual_end, payment_delay, calendar_id);
            let df = relative_df(disc, as_of, payment_date)?;
            
            terms.push((interest + spread_contrib) * df);
            accrual_start = accrual_end;
        }

        let total_pv = kahan_sum(terms);
        Ok(Money::new(total_pv, self.notional.currency()))
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
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10).
    pub(crate) fn pv_fixed_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let sched = crate::instruments::irs::cashflow::fixed_leg_schedule(self)?;

        // Payment delay in business days (typically 2 for Bloomberg OIS swaps)
        let payment_delay = self.fixed.payment_delay_days;

        // Collect discounted flows for Kahan summation
        let mut terms = Vec::with_capacity(sched.flows.len());

        for cf in &sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                // Only include future cashflows
                if cf.date <= as_of {
                    continue;
                }

                // Apply payment delay: actual payment occurs payment_delay_days after period end.
                // Use holiday-aware business days when a calendar is available.
                let payment_date =
                    add_payment_delay(cf.date, payment_delay, self.fixed.calendar_id.as_deref());

                // Discount from as_of for correct theta
                let df = relative_df(disc, as_of, payment_date)?;
                terms.push(cf.amount.amount() * df);
            }
        }

        // Use Kahan compensated summation for numerical stability
        let total = kahan_sum(terms);
        Ok(Money::new(total, self.notional.currency()))
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
    ///
    /// # Numerical Stability
    ///
    /// Uses Kahan compensated summation for accurate PV calculation on
    /// long-dated swaps with many periods (30Y+ = 120+ quarterly payments).
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10).
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        fwd: &finstack_core::market_data::term_structures::forward_curve::ForwardCurve,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Build the floating-leg schedule via the shared cashflow builder so reset
        // lags, calendars, and stub handling stay centralized.
        let schedule = crate::instruments::irs::cashflow::float_leg_schedule(self)?;

        // IRS legs here do not expose caps/floors; keep parameters centralized.
        let rate_params = crate::cashflow::builder::rate_helpers::FloatingRateParams {
            spread_bp: self.float.spread_bp,
            gearing: 1.0,
            gearing_includes_spread: true,
            index_floor_bp: None,
            index_cap_bp: None,
            all_in_floor_bp: None,
            all_in_cap_bp: None,
        };

        let mut terms = Vec::with_capacity(schedule.flows.len());
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

            let reset_date = cf.reset_date.unwrap_or(accrual_start);

            let forward_rate = crate::cashflow::builder::rate_helpers::project_floating_rate(
                reset_date,
                accrual_end,
                fwd,
                &rate_params,
            )?;

            // Use the builder's accrual factor (floating leg day count + stub rules).
            let yf = cf.accrual_factor;
            let coupon_amount = self.notional.amount() * forward_rate * yf;

            // Apply payment delay: actual payment occurs payment_delay_days after period end.
            // Use holiday-aware business days when a calendar is available.
            let payment_date = add_payment_delay(accrual_end, self.float.payment_delay_days, self.float.calendar_id.as_deref());

            // Discount from as_of for correct theta
            let df = relative_df(disc, as_of, payment_date)?;
            terms.push(coupon_amount * df);

            accrual_start = accrual_end;
        }

        let total = kahan_sum(terms);
        Ok(Money::new(total, self.notional.currency()))
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
/// ```no_run
/// use finstack_valuations::instruments::irs::{InterestRateSwap, pricer};
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
///     .map_err(|e| finstack_core::error::Error::Validation(format!("{}", e)))?;
///
/// let npv = pricer::npv(&irs, &context, as_of)?;
/// println!("Swap NPV: {}", npv);
/// # Ok(())
/// # }
/// ```
pub fn npv(irs: &InterestRateSwap, context: &MarketContext, as_of: Date) -> Result<Money> {
    let disc = context.get_discount_ref(irs.fixed.discount_curve_id.as_ref())?;
    let pv_fixed = irs.pv_fixed_leg(disc, as_of)?;

    let pv_float = match irs.float.compounding {
        FloatingLegCompounding::Simple => {
            // Term-rate swap: requires forward curve for float leg pricing
            let fwd = context.get_forward_ref(irs.float.forward_curve_id.as_ref())?;
            irs.pv_float_leg(disc, fwd, as_of)?
        }
        FloatingLegCompounding::CompoundedInArrears { .. } => {
            // Compounded RFR swap (single-curve or multi-curve).
            //
            // For single-curve setups it is common to have only a discount curve loaded;
            // in that case we derive implied overnight forwards from the discount curve.
            let proj = if irs.is_single_curve_ois() {
                context.get_forward_ref(irs.float.forward_curve_id.as_ref()).ok()
            } else {
                Some(context.get_forward_ref(irs.float.forward_curve_id.as_ref())?)
            };
            let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
            let fixings = context.series(&fixings_id).ok();
            irs.pv_compounded_float_leg(disc, proj, as_of, fixings)?
        }
    };

    let npv = match irs.side {
        crate::instruments::irs::PayReceive::PayFixed => (pv_float - pv_fixed)?,
        crate::instruments::irs::PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
    };
    Ok(npv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use crate::instruments::common::traits::Instrument;
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
            .side(crate::instruments::irs::PayReceive::PayFixed)
            .fixed(crate::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: disc_id.clone(),
                rate: 0.03,
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
            })
            .float(crate::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: disc_id.clone(),
                forward_curve_id: disc_id.clone(), // single-curve: same id as discount
                spread_bp: 0.0,
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
            })
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
}
