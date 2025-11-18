// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

use crate::instruments::irs::InterestRateSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::traits::Forward;
use finstack_core::market_data::MarketContext;
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
/// # Errors
///
/// Returns a validation error if:
/// - Year fraction calculation fails
/// - The as_of discount factor is below DF_EPSILON threshold
fn relative_df(
    disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
    as_of: Date,
    target: Date,
) -> Result<f64> {
    let disc_dc = disc.day_count();
    let base = disc.base_date();
    
    let t_as_of = disc_dc.year_fraction(
        base,
        as_of,
        finstack_core::dates::DayCountCtx::default(),
    )?;
    let t_target = disc_dc.year_fraction(
        base,
        target,
        finstack_core::dates::DayCountCtx::default(),
    )?;
    
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

/// IRS discounting pricer using the generic implementation.
pub type SimpleIrsDiscountingPricer =
    GenericDiscountingPricer<crate::instruments::InterestRateSwap>;

impl Default for SimpleIrsDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::IRS)
    }
}

impl InterestRateSwap {
    /// Returns true if this swap should be treated as an overnight index swap (OIS)
    /// for pricing purposes.
    ///
    /// A swap is considered OIS when:
    /// - The floating leg uses an overnight compounding convention
    ///   (`CompoundedInArrears` or `CompoundedDaily`), and
    /// - The floating leg's index (forward curve) is the same as the fixed leg's
    ///   discount curve, so both are tied to the same OIS curve.
    pub(crate) fn is_ois(&self) -> bool {
        matches!(
            self.float.compounding,
            FloatingLegCompounding::CompoundedInArrears { .. }
                | FloatingLegCompounding::CompoundedDaily
        ) && self.float.forward_curve_id == self.fixed.discount_curve_id
    }

    /// Compute PV of the floating leg for OIS swaps using discount-only logic.
    ///
    /// Implements the standard OIS identity:
    /// `PV_float = N × (DF(start) - DF(end)) + spread_annuity`, with all
    /// discounting performed relative to `as_of` so seasoned swaps are handled
    /// consistently with other instruments.
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

            let mut annuity = 0.0;
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
                annuity += alpha * df;
            }

            if annuity != 0.0 {
                pv += self.notional.amount() * (self.float.spread_bp * BP_TO_DECIMAL) * annuity;
            }
        }

        Ok(Money::new(pv, self.notional.currency()))
    }

    /// Compute PV of an overnight-indexed (compounded-in-arrears) floating leg.
    ///
    /// This is a thin wrapper around [`pv_ois_float_leg`] and exists to make the
    /// pricing intent explicit when the floating leg uses an RFR-style
    /// compounding convention (`FloatingLegCompounding::CompoundedInArrears` or
    /// `FloatingLegCompounding::CompoundedDaily`).
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

        // Sum discounted coupon flows from as_of date
        let mut total = Money::new(0.0, self.notional.currency());

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
                let disc_amt = cf.amount * df;
                total = (total + disc_amt)?;
            }
        }
        Ok(total)
    }

    /// Compute PV of floating leg (helper for value calculation).
    ///
    /// # Errors
    ///
    /// Returns a validation error if the valuation date discount factor is below
    /// the numerical stability threshold (DF_EPSILON = 1e-10).
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        fwd: &dyn Forward,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Use shared floating-leg cashflow schedule for dates and accrual metadata
        let sched = crate::instruments::irs::cashflow::float_leg_schedule(self)?;

        // Collect floating coupon flows in chronological order
        let mut float_flows: Vec<&crate::cashflow::primitives::CashFlow> = sched
            .flows
            .iter()
            .filter(|cf| {
                cf.kind == crate::cashflow::primitives::CFKind::FloatReset
                    // Only include cash-paying coupons; PIK (if any) is handled via outstanding
            })
            .collect();

        if float_flows.is_empty() {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Ensure flows are sorted by date in case upstream builders change ordering
        float_flows.sort_by_key(|cf| cf.date);

        let mut total = Money::new(0.0, self.notional.currency());
        let base = disc.base_date();

        for (idx, cf) in float_flows.iter().enumerate() {
            let d = cf.date;

            // Only include future cashflows
            if d <= as_of {
                continue;
            }

            // Determine accrual period start: first coupon uses leg start, others use prior coupon date
            let prev = if idx == 0 {
                self.float.start
            } else {
                float_flows[idx - 1].date
            };

            let t1 = self
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())?;
            let t2 = self
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())?;

            // Use accrual factor from schedule if available, otherwise fall back to recomputation
            let yf = if cf.accrual_factor > 0.0 {
                cf.accrual_factor
            } else {
                self.float
                    .dc
                    .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?
            };

            // Only call rate_period if t1 < t2 to avoid date ordering errors
            let f = if t2 > t1 {
                fwd.rate_period(t1, t2)
            } else {
                0.0
            };
            let rate = f + (self.float.spread_bp * BP_TO_DECIMAL);
            let coupon = self.notional * (rate * yf);

            // Discount from as_of for correct theta
            let df = relative_df(disc, as_of, d)?;
            let disc_amt = coupon * df;
            total = (total + disc_amt)?;
        }
        Ok(total)
    }
}

/// Standalone NPV helper to keep pricing logic in the `pricer` module.
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
