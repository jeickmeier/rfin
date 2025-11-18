// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

use crate::instruments::irs::InterestRateSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::traits::Forward;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::instruments::irs::FloatingLegCompounding;

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
    pub(crate) fn pv_ois_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> Result<Money> {
        let disc_dc = disc.day_count();

        // Discount factor at valuation date for correct theta / seasoned handling
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        // Start and end discount factors for the OIS leg
        let t_start = disc_dc
            .year_fraction(
                disc.base_date(),
                self.float.start,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_end = disc_dc
            .year_fraction(
                disc.base_date(),
                self.float.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let df_start_abs = disc.df(t_start);
        let df_end_abs = disc.df(t_end);
        let df_start = if df_as_of != 0.0 {
            df_start_abs / df_as_of
        } else {
            1.0
        };
        let df_end = if df_as_of != 0.0 {
            df_end_abs / df_as_of
        } else {
            1.0
        };

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
                let t_d = disc_dc
                    .year_fraction(
                        disc.base_date(),
                        cf.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df_d_abs = disc.df(t_d);
                let df = if df_as_of != 0.0 {
                    df_d_abs / df_as_of
                } else {
                    1.0
                };
                annuity += alpha * df;
            }

            if annuity != 0.0 {
                pv += self.notional.amount() * (self.float.spread_bp * 1e-4) * annuity;
            }
        }

        Ok(Money::new(pv, self.notional.currency()))
    }

    /// Compute PV of fixed leg (helper for value calculation).
    pub(crate) fn pv_fixed_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let sched = crate::instruments::irs::cashflow::fixed_leg_schedule(self)?;

        // Sum discounted coupon flows from as_of date
        let mut total = Money::new(0.0, self.notional.currency());
        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        for cf in &sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                // Only include future cashflows
                if cf.date <= as_of {
                    continue;
                }

                // Discount from as_of for correct theta
                let t_cf = disc_dc
                    .year_fraction(
                        disc.base_date(),
                        cf.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df_cf_abs = disc.df(t_cf);
                let df = if df_as_of != 0.0 {
                    df_cf_abs / df_as_of
                } else {
                    1.0
                };
                let disc_amt = cf.amount * df;
                total = (total + disc_amt)?;
            }
        }
        Ok(total)
    }

    /// Compute PV of floating leg (helper for value calculation).
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        fwd: &dyn Forward,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Use shared pricing grid from cashflow module to build date schedule
        let sched_dates = crate::instruments::irs::cashflow::float_pricing_grid(self)?;

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let mut prev = sched_dates[0];
        let mut total = Money::new(0.0, self.notional.currency());

        // Pre-compute as_of discount factor
        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        for &d in &sched_dates[1..] {
            // Only include future cashflows
            if d <= as_of {
                prev = d;
                continue;
            }

            let base = disc.base_date();
            let t1 = self
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = self
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = self
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);

            // Only call rate_period if t1 < t2 to avoid date ordering errors
            let f = if t2 > t1 {
                fwd.rate_period(t1, t2)
            } else {
                0.0
            };
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);

            // Discount from as_of for correct theta
            let t_cf = disc_dc
                .year_fraction(
                    disc.base_date(),
                    d,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let df_cf_abs = disc.df(t_cf);
            let df = if df_as_of != 0.0 {
                df_cf_abs / df_as_of
            } else {
                1.0
            };
            let disc_amt = coupon * df;
            total = (total + disc_amt)?;
            prev = d;
        }
        Ok(total)
    }
}

/// Standalone NPV helper to keep pricing logic in the `pricer` module.
pub fn npv(irs: &InterestRateSwap, context: &MarketContext, as_of: Date) -> Result<Money> {
    let disc = context.get_discount_ref(irs.fixed.discount_curve_id.as_ref())?;
    let pv_fixed = irs.pv_fixed_leg(disc, as_of)?;
    let pv_float = if irs.is_ois() {
        // OIS swap: use discount-only method for accurate pricing.
        irs.pv_ois_float_leg(disc, as_of)?
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
        let mut irs = InterestRateSwap::example();
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
