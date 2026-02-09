//! FX touch option calculator using Rubinstein & Reiner (1991) closed-form.
//!
//! Provides pricing for one-touch and no-touch FX options with continuous
//! barrier monitoring. Supports both payout-at-hit and payout-at-expiry timing.
//!
//! # References
//!
//! - Rubinstein, M., & Reiner, E. (1991). "Unscrambling the Binary Code."
//!   *Risk Magazine*, 4(9), 75-83.
//! - Wystup, U. (2006). *FX Options and Structured Products*. Wiley. Chapter 4.

use crate::instruments::fx::fx_touch_option::types::{
    BarrierDirection, FxTouchOption, PayoutTiming, TouchType,
};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

/// FX touch option calculator.
#[derive(Debug, Clone, Default)]
pub struct FxTouchOptionCalculator;

impl FxTouchOptionCalculator {
    /// Compute present value of an FX touch option.
    pub fn npv(&self, inst: &FxTouchOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            // Expired without being exercised - for no-touch, full payout;
            // for one-touch, check if barrier was breached (assume not for static valuation).
            let pv = match inst.touch_type {
                TouchType::OneTouch => 0.0,
                TouchType::NoTouch => inst.payout_amount.amount(),
            };
            return Ok(Money::new(pv, inst.quote_currency));
        }

        let price = price_touch(
            spot,
            inst.barrier_level,
            r_d,
            r_f,
            sigma,
            t,
            inst.touch_type,
            inst.barrier_direction,
            inst.payout_timing,
            inst.payout_amount.amount(),
        );

        Ok(Money::new(price, inst.quote_currency))
    }

    /// Collect market inputs (spot, rates, vol, time).
    pub fn collect_inputs(
        &self,
        inst: &FxTouchOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let df_d = domestic_disc.df(domestic_disc.day_count().year_fraction(
            as_of,
            inst.expiry,
            DayCountCtx::default(),
        )?);
        let df_f = foreign_disc.df(foreign_disc.day_count().year_fraction(
            as_of,
            inst.expiry,
            DayCountCtx::default(),
        )?);

        let r_d = if t_vol > 0.0 { -df_d.ln() / t_vol } else { 0.0 };
        let r_f = if t_vol > 0.0 { -df_f.ln() / t_vol } else { 0.0 };

        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        let sigma = if let Some(impl_vol) = inst.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
            vol_surface.value_clamped(t_vol, inst.barrier_level)
        };

        Ok((spot, r_d, r_f, sigma, t_vol))
    }

    fn collect_inputs_expired(
        &self,
        inst: &FxTouchOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;
        Ok((spot, 0.0, 0.0, 0.0, 0.0))
    }
}

/// Price a one-touch option with continuous barrier monitoring (pay at expiry).
///
/// Uses Rubinstein & Reiner (1991) formula:
/// ```text
/// P = e^{-r_d T} × payout × [(S/H)^{-(μ+λ)} × N(η·z) + (S/H)^{-(μ-λ)} × N(η·z')]
/// ```
///
/// where:
/// - μ = (r_d - r_f - σ²/2) / σ²
/// - λ = sqrt(μ² + 2r_d/σ²)
/// - z = ln(H/S)/(σ√T) + λσ√T
/// - z' = ln(H/S)/(σ√T) - λσ√T
/// - η = +1 for down barrier, -1 for up barrier
#[allow(clippy::too_many_arguments)]
fn price_touch(
    spot: f64,
    barrier: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    touch_type: TouchType,
    barrier_direction: BarrierDirection,
    payout_timing: PayoutTiming,
    payout: f64,
) -> f64 {
    // Check if spot has already breached the barrier.
    // If so, the one-touch has already triggered and the no-touch has expired worthless.
    let already_breached = match barrier_direction {
        BarrierDirection::Down => spot <= barrier,
        BarrierDirection::Up => spot >= barrier,
    };
    if already_breached {
        return match touch_type {
            TouchType::OneTouch => match payout_timing {
                PayoutTiming::AtHit => payout,
                PayoutTiming::AtExpiry => (-r_d * t).exp() * payout,
            },
            TouchType::NoTouch => 0.0,
        };
    }

    let sigma2 = sigma * sigma;
    let sqrt_t = t.sqrt();
    let sigma_sqrt_t = sigma * sqrt_t;

    if sigma_sqrt_t <= 0.0 || t <= 0.0 {
        return 0.0;
    }

    // mu = (r_d - r_f - sigma^2/2) / sigma^2
    let mu = (r_d - r_f - sigma2 / 2.0) / sigma2;

    // For pay-at-expiry: lambda uses 0 to compute the pure hitting probability;
    //   the e^{-r_d T} factor is applied separately when discounting the payout.
    // For pay-at-hit: lambda uses r_d so that the formula directly computes
    //   the expected discounted payout E^Q[e^{-r_d τ} 1_{τ≤T}] via the
    //   Rubinstein-Reiner (1991) Laplace transform of the first passage time.
    let lambda_r = match payout_timing {
        PayoutTiming::AtExpiry => 0.0,
        PayoutTiming::AtHit => r_d,
    };

    let lambda_sq = mu * mu + 2.0 * lambda_r / sigma2;
    if lambda_sq < 0.0 {
        // Shouldn't happen with reasonable inputs, but guard against numerical issues
        return 0.0;
    }
    let lambda = lambda_sq.sqrt();

    let log_hs = (barrier / spot).ln();
    let z = log_hs / sigma_sqrt_t + lambda * sigma_sqrt_t;
    let z_prime = log_hs / sigma_sqrt_t - lambda * sigma_sqrt_t;

    // eta: +1 for down barrier (H < S), -1 for up barrier (H > S)
    let eta = match barrier_direction {
        BarrierDirection::Down => 1.0,
        BarrierDirection::Up => -1.0,
    };

    let s_over_h = spot / barrier;

    // (S/H)^{-(mu+lambda)} and (S/H)^{-(mu-lambda)}
    let power1 = s_over_h.powf(-(mu + lambda));
    let power2 = s_over_h.powf(-(mu - lambda));

    let n_eta_z = finstack_core::math::norm_cdf(eta * z);
    let n_eta_z_prime = finstack_core::math::norm_cdf(eta * z_prime);

    let one_touch_prob = power1 * n_eta_z + power2 * n_eta_z_prime;

    match payout_timing {
        PayoutTiming::AtExpiry => {
            let df = (-r_d * t).exp();
            let one_touch_pv = df * payout * one_touch_prob;
            match touch_type {
                TouchType::OneTouch => one_touch_pv,
                TouchType::NoTouch => df * payout - one_touch_pv,
            }
        }
        PayoutTiming::AtHit => {
            // For pay-at-hit, the formula directly gives the PV since lambda
            // already incorporates the stochastic discounting.
            // The one_touch_prob computed above with r_d in lambda already
            // gives the risk-neutral expected discounted payout.
            let one_touch_pv = payout * one_touch_prob;
            match touch_type {
                TouchType::OneTouch => one_touch_pv,
                // No-touch with pay-at-hit doesn't make financial sense
                // (you'd have to wait until expiry to know it wasn't touched),
                // but we handle it for completeness.
                TouchType::NoTouch => {
                    let df = (-r_d * t).exp();
                    df * payout - one_touch_pv
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/test_utils.rs"
        ));
    }

    use super::*;
    use crate::instruments::fx::fx_touch_option::FxTouchOption;
    use crate::instruments::{common::traits::Attributes, PricingOverrides};
    use finstack_core::{
        currency::Currency,
        dates::{Date, DayCount},
        market_data::context::MarketContext,
        money::{
            fx::{FxConversionPolicy, FxMatrix, FxProvider},
            Money,
        },
        types::{CurveId, InstrumentId},
    };
    use std::sync::Arc;
    use test_utils::{date, flat_discount_with_tenor, flat_vol_surface};

    const BASE: Currency = Currency::EUR;
    const QUOTE: Currency = Currency::USD;
    const DOMESTIC_ID: &str = "USD-OIS";
    const FOREIGN_ID: &str = "EUR-OIS";
    const VOL_ID: &str = "EURUSD-VOL";

    struct StaticFxProvider {
        rate: f64,
    }

    impl FxProvider for StaticFxProvider {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<f64> {
            if from == to {
                return Ok(1.0);
            }
            if from == BASE && to == QUOTE {
                Ok(self.rate)
            } else if from == QUOTE && to == BASE {
                Ok(1.0 / self.rate)
            } else {
                Ok(1.0)
            }
        }
    }

    fn market_context(
        as_of: Date,
        spot: f64,
        vol: f64,
        r_domestic: f64,
        r_foreign: f64,
    ) -> MarketContext {
        let fx = FxMatrix::new(Arc::new(StaticFxProvider { rate: spot }));
        fx.set_quote(BASE, QUOTE, spot);
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [0.8, 0.9, 1.0, 1.1, 1.2];
        MarketContext::new()
            .insert_discount(flat_discount_with_tenor(
                DOMESTIC_ID,
                as_of,
                r_domestic,
                1.0,
            ))
            .insert_discount(flat_discount_with_tenor(FOREIGN_ID, as_of, r_foreign, 1.0))
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, vol))
            .insert_fx(fx)
    }

    fn base_touch_option(
        expiry: Date,
        touch_type: TouchType,
        barrier_direction: BarrierDirection,
        barrier_level: f64,
    ) -> FxTouchOption {
        FxTouchOption::builder()
            .id(InstrumentId::new("EURUSD_TOUCH"))
            .base_currency(BASE)
            .quote_currency(QUOTE)
            .barrier_level(barrier_level)
            .touch_type(touch_type)
            .barrier_direction(barrier_direction)
            .payout_amount(Money::new(100_000.0, QUOTE))
            .payout_timing(PayoutTiming::AtExpiry)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .domestic_discount_curve_id(CurveId::new(DOMESTIC_ID))
            .foreign_discount_curve_id(CurveId::new(FOREIGN_ID))
            .vol_surface_id(CurveId::new(VOL_ID))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed")
    }

    fn approx_eq(actual: f64, expected: f64, tol: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "expected {expected}, got {actual} (diff {diff} > {tol})"
        );
    }

    #[test]
    fn one_touch_down_has_positive_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        // Barrier below spot (down-and-in)
        let option = base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.05);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        assert!(pv.amount() > 0.0, "One-touch should have positive value");
        assert_eq!(pv.currency(), QUOTE);
    }

    #[test]
    fn one_touch_up_has_positive_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        // Barrier above spot (up-and-in)
        let option = base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Up, 1.30);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        assert!(pv.amount() > 0.0, "Up one-touch should have positive value");
    }

    #[test]
    fn one_touch_plus_no_touch_equals_discounted_payout() {
        // One-touch PV + No-touch PV = e^{-r_d T} × payout
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let one_touch =
            base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.05);
        let no_touch = base_touch_option(expiry, TouchType::NoTouch, BarrierDirection::Down, 1.05);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv_ot = calc.npv(&one_touch, &ctx, as_of).expect("should succeed");
        let pv_nt = calc.npv(&no_touch, &ctx, as_of).expect("should succeed");
        let sum = pv_ot.amount() + pv_nt.amount();

        let (_, r_d, _, _, t) = calc
            .collect_inputs(&one_touch, &ctx, as_of)
            .expect("should succeed");
        let discounted_payout = (-r_d * t).exp() * one_touch.payout_amount.amount();

        approx_eq(sum, discounted_payout, 1e-4);
    }

    #[test]
    fn no_touch_value_less_than_discounted_payout() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_touch_option(expiry, TouchType::NoTouch, BarrierDirection::Down, 1.05);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        let (_, r_d, _, _, t) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("should succeed");
        let discounted_payout = (-r_d * t).exp() * option.payout_amount.amount();

        assert!(
            pv.amount() < discounted_payout,
            "No-touch PV ({}) should be less than discounted payout ({})",
            pv.amount(),
            discounted_payout
        );
        assert!(pv.amount() > 0.0, "No-touch should have positive value");
    }

    #[test]
    fn barrier_closer_to_spot_increases_one_touch_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let far_barrier =
            base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 0.90);
        let near_barrier =
            base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.10);

        let pv_far = calc.npv(&far_barrier, &ctx, as_of).expect("should succeed");
        let pv_near = calc
            .npv(&near_barrier, &ctx, as_of)
            .expect("should succeed");

        assert!(
            pv_near.amount() > pv_far.amount(),
            "Closer barrier should give higher one-touch value: near={}, far={}",
            pv_near.amount(),
            pv_far.amount()
        );
    }

    #[test]
    fn higher_vol_increases_one_touch_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.05);
        let calc = FxTouchOptionCalculator;

        let ctx_low_vol = market_context(as_of, 1.18, 0.10, 0.03, 0.01);
        let ctx_high_vol = market_context(as_of, 1.18, 0.30, 0.03, 0.01);

        let pv_low = calc
            .npv(&option, &ctx_low_vol, as_of)
            .expect("should succeed");
        let pv_high = calc
            .npv(&option, &ctx_high_vol, as_of)
            .expect("should succeed");

        assert!(
            pv_high.amount() > pv_low.amount(),
            "Higher vol should increase one-touch value: high={}, low={}",
            pv_high.amount(),
            pv_low.amount()
        );
    }

    #[test]
    fn expired_one_touch_returns_zero() {
        let expiry = date(2025, 1, 3);
        let option = base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.05);
        let ctx = market_context(expiry, 1.18, 0.2, 0.02, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv = calc.npv(&option, &ctx, expiry).expect("should succeed");
        approx_eq(pv.amount(), 0.0, 1e-6);
    }

    #[test]
    fn expired_no_touch_returns_full_payout() {
        let expiry = date(2025, 1, 3);
        let option = base_touch_option(expiry, TouchType::NoTouch, BarrierDirection::Down, 1.05);
        let ctx = market_context(expiry, 1.18, 0.2, 0.02, 0.01);
        let calc = FxTouchOptionCalculator;

        let pv = calc.npv(&option, &ctx, expiry).expect("should succeed");
        approx_eq(pv.amount(), option.payout_amount.amount(), 1e-6);
    }

    #[test]
    fn test_curve_dependencies() {
        use crate::instruments::common_impl::traits::CurveDependencies;

        let option = base_touch_option(
            date(2025, 6, 15),
            TouchType::OneTouch,
            BarrierDirection::Down,
            1.05,
        );
        let deps = option.curve_dependencies().expect("curve_dependencies");
        assert_eq!(deps.discount_curves.len(), 2);
    }

    #[test]
    fn pay_at_hit_one_touch_value_exceeds_pay_at_expiry() {
        // Pay-at-hit should be more valuable than pay-at-expiry
        // because the holder receives the payout earlier.
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxTouchOptionCalculator;

        let at_expiry =
            base_touch_option(expiry, TouchType::OneTouch, BarrierDirection::Down, 1.10);
        let mut at_hit = at_expiry.clone();
        at_hit.payout_timing = PayoutTiming::AtHit;

        let pv_expiry = calc.npv(&at_expiry, &ctx, as_of).expect("should succeed");
        let pv_hit = calc.npv(&at_hit, &ctx, as_of).expect("should succeed");

        assert!(
            pv_hit.amount() >= pv_expiry.amount(),
            "Pay-at-hit ({}) should be >= pay-at-expiry ({})",
            pv_hit.amount(),
            pv_expiry.amount()
        );
    }
}
