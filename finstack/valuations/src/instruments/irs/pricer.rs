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
use finstack_core::math::kahan_sum;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::instruments::irs::FloatingLegCompounding;

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
    let disc_dc = disc.day_count();
    let base = disc.base_date();

    let t_as_of =
        disc_dc.year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())?;
    let t_target =
        disc_dc.year_fraction(base, target, finstack_core::dates::DayCountCtx::default())?;

    let df_as_of = disc.df(t_as_of);

    // Guard against near-zero discount factors for numerical stability
    if df_as_of.abs() < DF_EPSILON {
        return Err(finstack_core::error::Error::Validation(format!(
            "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
             This may indicate extreme rate scenarios or very long time horizons.",
            df_as_of, DF_EPSILON
        )));
    }

    let df_target = disc.df(t_target);
    Ok(df_target / df_as_of)
}

impl InterestRateSwap {
    /// Returns true if this swap should be treated as an overnight index swap (OIS)
    /// for pricing purposes.
    ///
    /// A swap is considered OIS when:
    /// - The floating leg uses an overnight compounding convention
    ///   (`CompoundedInArrears`), and
    /// - The floating leg's index (forward curve) is the same as the fixed leg's
    ///   discount curve, so both are tied to the same OIS curve.
    ///
    /// # Returns
    ///
    /// `true` if the swap uses overnight compounding with matching discount/forward
    /// curves, `false` otherwise (indicating a term-rate swap requiring separate
    /// forward curve projection).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Note: is_ois is a private helper method used internally for pricing logic
    /// use finstack_valuations::instruments::irs::{InterestRateSwap, FloatingLegCompounding};
    ///
    /// let mut irs = InterestRateSwap::example()?;
    ///
    /// // Default example is a term-rate swap (not OIS)
    /// assert!(!irs.is_ois());
    ///
    /// // Convert to OIS by using overnight compounding and matching curves
    /// irs.float.compounding = FloatingLegCompounding::sofr();
    /// irs.float.forward_curve_id = irs.fixed.discount_curve_id.clone();
    /// assert!(irs.is_ois());
    /// ```
    pub(crate) fn is_ois(&self) -> bool {
        matches!(
            self.float.compounding,
            FloatingLegCompounding::CompoundedInArrears { .. }
        ) && self.float.forward_curve_id == self.fixed.discount_curve_id
    }

    /// Compute PV of the floating leg for OIS swaps using discount-only logic.
    ///
    /// Implements the standard OIS identity:
    /// `PV_float = N × (DF(start) - DF(end)) + spread_annuity`, with all
    /// discounting performed relative to `as_of` so seasoned swaps are handled
    /// consistently with other instruments.
    ///
    /// # Numerical Stability
    ///
    /// Uses Kahan compensated summation for the spread annuity calculation,
    /// ensuring accurate results for long-dated OIS swaps with many periods.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10), which can occur
    /// in extreme rate scenarios.
    pub(crate) fn pv_ois_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> Result<Money> {
        // Start and end discount factors for the OIS leg (relative to as_of)
        let df_start = relative_df(disc, as_of, self.float.start)?;
        let df_end = relative_df(disc, as_of, self.float.end)?;

        let mut pv = self.notional.amount() * (df_start - df_end);

        // Add spread contribution if any: N × sum_i( spread × alpha_i × DF(T_i) )
        if self.float.spread_bp != 0.0 {
            // Use shared float-leg schedule to build spread annuity
            let sched = crate::instruments::irs::cashflow::float_leg_schedule(self)?;

            // Collect terms for Kahan summation
            let mut terms = Vec::with_capacity(sched.flows.len());
            for cf in &sched.flows {
                if cf.kind != crate::cashflow::primitives::CFKind::FloatReset {
                    continue;
                }
                // Only include future cashflows
                if cf.date <= as_of {
                    continue;
                }

                let alpha = cf.accrual_factor;
                let df = relative_df(disc, as_of, cf.date)?;
                terms.push(alpha * df);
            }

            // Use Kahan compensated summation for numerical stability
            let annuity = kahan_sum(terms);

            if annuity.abs() > f64::EPSILON {
                pv += self.notional.amount() * (self.float.spread_bp * BP_TO_DECIMAL) * annuity;
            }
        }

        Ok(Money::new(pv, self.notional.currency()))
    }

    /// Compute PV of an overnight-indexed (compounded-in-arrears) floating leg.
    ///
    /// This is a thin wrapper around [`pv_ois_float_leg`] and exists to make the
    /// pricing intent explicit when the floating leg uses an RFR-style
    /// compounding convention (`FloatingLegCompounding::CompoundedInArrears`).
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for discounting cashflows
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value of the compounded floating leg in the swap's notional currency.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10).
    #[inline]
    pub(crate) fn pv_compounded_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> Result<Money> {
        self.pv_ois_float_leg(disc, as_of)
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

                // Discount from as_of for correct theta
                let df = relative_df(disc, as_of, cf.date)?;
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

        for cf in schedule.flows.iter().filter(|cf| {
            cf.kind == crate::cashflow::primitives::CFKind::FloatReset
        }) {
            let accrual_end = cf.date;

            // Skip settled cashflows
            if accrual_end <= as_of {
                accrual_start = accrual_end;
                continue;
            }

            let reset_date = cf.reset_date.unwrap_or(accrual_start);

            let forward_rate =
                crate::cashflow::builder::rate_helpers::project_floating_rate_detailed(
                    reset_date,
                    accrual_end,
                    fwd,
                    &rate_params,
                )?;

            // Use the builder's accrual factor (floating leg day count + stub rules).
            let yf = cf.accrual_factor;
            let coupon_amount = self.notional.amount() * forward_rate * yf;

            // Discount from as_of for correct theta
            let df = relative_df(disc, as_of, accrual_end)?;
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
    let pv_float = if irs.is_ois() {
        // OIS / compounded RFR swap: use discount-only method for accurate pricing.
        irs.pv_compounded_float_leg(disc, as_of)?
    } else {
        // Non-OIS swap: requires forward curve for float leg pricing
        match context.get_forward_ref(irs.float.forward_curve_id.as_ref()) {
            Ok(fwd) => irs.pv_float_leg(disc, fwd, as_of)?,
            Err(_) => {
                // Forward curve missing: return error to guide callers
                return Err(context
                    .get_forward_ref(irs.float.forward_curve_id.as_ref())
                    .err()
                    .unwrap_or(finstack_core::error::InputError::Invalid.into()));
            }
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

    #[test]
    fn is_ois_classification_uses_compounding_and_curve_ids() {
        // Start from the example vanilla IRS (term-rate style) which should
        // not be classified as OIS even though both legs are discounted on OIS.
        let mut irs = InterestRateSwap::example().expect("Example should construct successfully");
        assert!(
            !irs.is_ois(),
            "Vanilla term-rate IRS with Simple compounding must not be OIS"
        );

        // Turn it into an OIS-style swap: use overnight compounding and align
        // the floating index with the fixed-leg discount curve.
        irs.float.compounding = FloatingLegCompounding::sofr();
        irs.float.forward_curve_id = irs.fixed.discount_curve_id.clone();

        assert!(
            irs.is_ois(),
            "Swap with overnight compounding and matching index/discount curves \
             should be classified as OIS"
        );
    }
}
