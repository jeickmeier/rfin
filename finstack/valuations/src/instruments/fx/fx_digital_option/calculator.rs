//! FX digital option calculator implementing Garman-Kohlhagen adapted pricing.
//!
//! Provides closed-form pricing for European digital (binary) FX options:
//! - Cash-or-nothing: pays fixed amount if ITM at expiry
//! - Asset-or-nothing: pays one unit of foreign currency if ITM at expiry
//!
//! # References
//!
//! - Reiner, E., & Rubinstein, M. (1991). "Unscrambling the Binary Code."
//!   *Risk Magazine*, 4(9), 75-83.

use crate::instruments::common_impl::models::volatility::black::d1_d2;
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::fx::fx_digital_option::types::{DigitalPayoutType, FxDigitalOption};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

/// FX digital option calculator.
#[derive(Debug, Clone)]
pub struct FxDigitalOptionCalculator {
    /// Days per year for theta scaling.
    pub theta_days_per_year: f64,
}

impl Default for FxDigitalOptionCalculator {
    fn default() -> Self {
        Self {
            theta_days_per_year: 365.0,
        }
    }
}

impl FxDigitalOptionCalculator {
    /// Compute present value of an FX digital option.
    pub fn npv(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            // Expired: check if ITM
            let itm = match inst.option_type {
                OptionType::Call => spot > inst.strike,
                OptionType::Put => spot < inst.strike,
            };
            return if itm {
                match inst.payout_type {
                    DigitalPayoutType::CashOrNothing => Ok(inst.payout_amount),
                    DigitalPayoutType::AssetOrNothing => Ok(Money::new(
                        spot * inst.notional.amount(),
                        inst.quote_currency,
                    )),
                }
            } else {
                Ok(Money::new(0.0, inst.quote_currency))
            };
        }

        let price = price_digital(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            inst.payout_type,
            inst.payout_amount.amount(),
            inst.notional.amount(),
        );

        Ok(Money::new(price, inst.quote_currency))
    }

    /// Compute Greeks for an FX digital option.
    pub fn compute_greeks(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<FxDigitalOptionGreeks> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            return Ok(FxDigitalOptionGreeks::default());
        }

        let greeks = greeks_digital(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            inst.payout_type,
            inst.payout_amount.amount(),
            inst.notional.amount(),
            self.theta_days_per_year,
        );

        Ok(greeks)
    }

    /// Collect market inputs (spot, rates, vol, time).
    pub fn collect_inputs(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        let _t_disc_dom =
            inst.day_count
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        let t_disc_for =
            foreign_disc
                .day_count()
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        // Get discount factors using curve-native time
        let df_d = domestic_disc.df(domestic_disc.day_count().year_fraction(
            as_of,
            inst.expiry,
            DayCountCtx::default(),
        )?);
        let df_f = foreign_disc.df(t_disc_for);

        // Convert to effective zero rates consistent with t_vol
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
            vol_surface.value_clamped(t_vol, inst.strike)
        };

        Ok((spot, r_d, r_f, sigma, t_vol))
    }

    fn collect_inputs_expired(
        &self,
        inst: &FxDigitalOption,
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

/// Greeks for an FX digital option.
#[derive(Clone, Copy, Debug, Default)]
pub struct FxDigitalOptionGreeks {
    /// Delta: sensitivity to spot FX rate
    pub delta: f64,
    /// Gamma: rate of change of delta with respect to spot
    pub gamma: f64,
    /// Vega: sensitivity to 1% change in volatility
    pub vega: f64,
    /// Theta: time decay per day
    pub theta: f64,
    /// Rho domestic: sensitivity to 1% change in domestic interest rate
    pub rho_domestic: f64,
}

/// Digital option price (closed-form).
///
/// Cash-or-nothing call: `e^{-r_d T} × N(d2) × payout`
/// Cash-or-nothing put:  `e^{-r_d T} × N(-d2) × payout`
/// Asset-or-nothing call: `S × e^{-r_f T} × N(d1) × notional`
/// Asset-or-nothing put:  `S × e^{-r_f T} × N(-d1) × notional`
#[allow(clippy::too_many_arguments)]
fn price_digital(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
    payout_type: DigitalPayoutType,
    payout_amount: f64,
    notional: f64,
) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, r_d, sigma, t, r_f);
    let exp_rd_t = (-r_d * t).exp();
    let exp_rf_t = (-r_f * t).exp();

    match payout_type {
        DigitalPayoutType::CashOrNothing => match option_type {
            OptionType::Call => exp_rd_t * finstack_core::math::norm_cdf(d2) * payout_amount,
            OptionType::Put => exp_rd_t * finstack_core::math::norm_cdf(-d2) * payout_amount,
        },
        DigitalPayoutType::AssetOrNothing => match option_type {
            OptionType::Call => spot * exp_rf_t * finstack_core::math::norm_cdf(d1) * notional,
            OptionType::Put => spot * exp_rf_t * finstack_core::math::norm_cdf(-d1) * notional,
        },
    }
}

/// Compute Greeks for digital option (analytical).
#[allow(clippy::too_many_arguments)]
fn greeks_digital(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
    payout_type: DigitalPayoutType,
    payout_amount: f64,
    notional: f64,
    theta_days_per_year: f64,
) -> FxDigitalOptionGreeks {
    let (d1, d2) = d1_d2(spot, strike, r_d, sigma, t, r_f);
    let exp_rd_t = (-r_d * t).exp();
    let exp_rf_t = (-r_f * t).exp();
    let sqrt_t = t.sqrt();
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    let pdf_d2 = finstack_core::math::norm_pdf(d2);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let sigma_sqrt_t = sigma * sqrt_t;

    // Avoid division by zero
    if sigma_sqrt_t <= 0.0 {
        return FxDigitalOptionGreeks::default();
    }

    match payout_type {
        DigitalPayoutType::CashOrNothing => {
            // Delta: ∂PV/∂S
            // For call: e^{-r_d T} × n(d2) × payout / (S × σ√T)
            // For put: -e^{-r_d T} × n(d2) × payout / (S × σ√T)
            let delta_sign = match option_type {
                OptionType::Call => 1.0,
                OptionType::Put => -1.0,
            };
            let delta = delta_sign * exp_rd_t * pdf_d2 * payout_amount / (spot * sigma_sqrt_t);

            // Gamma: ∂²PV/∂S²
            // For call: -e^{-r_d T} × n(d2) × d1 × payout / (S² × σ² × T)
            let gamma = -delta_sign * exp_rd_t * pdf_d2 * d1 * payout_amount
                / (spot * spot * sigma * sigma * t);

            // Vega: ∂PV/∂σ (per 1% vol change)
            // For call: -e^{-r_d T} × n(d2) × d1 / σ × payout (but d1/σ = ∂d2/∂σ component)
            // More precisely: vega = -e^{-r_d T} × n(d2) × (d1/σ) × payout
            // Per 1%: divide by 100
            let vega = -delta_sign * exp_rd_t * pdf_d2 * (d1 / sigma) * payout_amount / 100.0;

            // Theta: -∂PV/∂T per day
            let base_pv = match option_type {
                OptionType::Call => exp_rd_t * cdf_d2 * payout_amount,
                OptionType::Put => exp_rd_t * (1.0 - cdf_d2) * payout_amount,
            };
            // Numerical theta via small time bump
            let dt = 1.0 / theta_days_per_year;
            let t_minus = (t - dt).max(0.0);
            let pv_t_minus = if t_minus > 0.0 {
                price_digital(
                    spot,
                    strike,
                    r_d,
                    r_f,
                    sigma,
                    t_minus,
                    option_type,
                    payout_type,
                    payout_amount,
                    notional,
                )
            } else {
                // At expiry: intrinsic
                let itm = match option_type {
                    OptionType::Call => spot > strike,
                    OptionType::Put => spot < strike,
                };
                if itm {
                    payout_amount
                } else {
                    0.0
                }
            };
            let theta = pv_t_minus - base_pv;

            // Rho domestic: ∂PV/∂r_d per 1%
            let rho_sign = match option_type {
                OptionType::Call => 1.0,
                OptionType::Put => -1.0,
            };
            let rho_domestic = (-t * base_pv
                + rho_sign * exp_rd_t * pdf_d2 * (t / sigma_sqrt_t) * payout_amount)
                / 100.0;

            FxDigitalOptionGreeks {
                delta,
                gamma,
                vega,
                theta,
                rho_domestic,
            }
        }
        DigitalPayoutType::AssetOrNothing => {
            // Delta: ∂PV/∂S
            // For call: e^{-r_f T} × [N(d1) + n(d1) / (σ√T)] × notional
            // For put:  e^{-r_f T} × [-N(-d1) + n(d1) / (σ√T)] × notional (negative)
            let delta = match option_type {
                OptionType::Call => exp_rf_t * (cdf_d1 + pdf_d1 / sigma_sqrt_t) * notional,
                OptionType::Put => exp_rf_t * (-(1.0 - cdf_d1) + pdf_d1 / sigma_sqrt_t) * notional,
            };

            // Gamma: numerical second derivative for robustness
            let bump = spot * 0.001;
            let pv_up = price_digital(
                spot + bump,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let pv_dn = price_digital(
                spot - bump,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let pv_base = price_digital(
                spot,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let gamma = (pv_up - 2.0 * pv_base + pv_dn) / (bump * bump);

            // Vega per 1%
            let vol_bump = 0.01;
            let pv_vol_up = price_digital(
                spot,
                strike,
                r_d,
                r_f,
                sigma + vol_bump,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let vega = (pv_vol_up - pv_base) / (vol_bump * 100.0);

            // Theta
            let dt = 1.0 / theta_days_per_year;
            let t_minus = (t - dt).max(0.0);
            let pv_t_minus = if t_minus > 0.0 {
                price_digital(
                    spot,
                    strike,
                    r_d,
                    r_f,
                    sigma,
                    t_minus,
                    option_type,
                    payout_type,
                    payout_amount,
                    notional,
                )
            } else {
                let itm = match option_type {
                    OptionType::Call => spot > strike,
                    OptionType::Put => spot < strike,
                };
                if itm {
                    spot * notional
                } else {
                    0.0
                }
            };
            let theta = pv_t_minus - pv_base;

            // Rho domestic: numerical
            let rate_bump = 0.0001; // 1bp
            let pv_rate_up = price_digital(
                spot,
                strike,
                r_d + rate_bump,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let rho_domestic = (pv_rate_up - pv_base) / rate_bump / 100.0;

            FxDigitalOptionGreeks {
                delta,
                gamma,
                vega,
                theta,
                rho_domestic,
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
    use crate::instruments::fx::fx_digital_option::FxDigitalOption;
    use crate::instruments::{common::traits::Attributes, OptionType, PricingOverrides};
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

    fn base_digital_option(
        expiry: Date,
        option_type: OptionType,
        payout_type: DigitalPayoutType,
    ) -> FxDigitalOption {
        FxDigitalOption::builder()
            .id(InstrumentId::new("EURUSD_DIGITAL"))
            .base_currency(BASE)
            .quote_currency(QUOTE)
            .strike(1.15)
            .option_type(option_type)
            .payout_type(payout_type)
            .payout_amount(Money::new(100_000.0, QUOTE))
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, BASE))
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
    fn cash_or_nothing_call_has_positive_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option =
            base_digital_option(expiry, OptionType::Call, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        assert!(pv.amount() > 0.0, "Digital call should have positive value");
        assert_eq!(pv.currency(), QUOTE);
        // PV should be less than discounted payout
        assert!(
            pv.amount() < option.payout_amount.amount(),
            "PV should be less than payout"
        );
    }

    #[test]
    fn cash_or_nothing_put_has_positive_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_digital_option(expiry, OptionType::Put, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        assert!(pv.amount() > 0.0, "Digital put should have positive value");
    }

    #[test]
    fn cash_or_nothing_call_plus_put_equals_discounted_payout() {
        // For cash-or-nothing: C_digital + P_digital = e^{-r_d T} × payout
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let call = base_digital_option(expiry, OptionType::Call, DigitalPayoutType::CashOrNothing);
        let put = base_digital_option(expiry, OptionType::Put, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let pv_call = calc.npv(&call, &ctx, as_of).expect("should succeed");
        let pv_put = calc.npv(&put, &ctx, as_of).expect("should succeed");
        let sum = pv_call.amount() + pv_put.amount();

        let (_, r_d, _, _, t) = calc
            .collect_inputs(&call, &ctx, as_of)
            .expect("should succeed");
        let discounted_payout = (-r_d * t).exp() * call.payout_amount.amount();

        approx_eq(sum, discounted_payout, 1e-3);
    }

    #[test]
    fn asset_or_nothing_call_has_positive_value() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option =
            base_digital_option(expiry, OptionType::Call, DigitalPayoutType::AssetOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        assert!(
            pv.amount() > 0.0,
            "Asset-or-nothing call should have positive value"
        );
    }

    #[test]
    fn expired_itm_digital_call_returns_payout() {
        let expiry = date(2025, 1, 3);
        let option =
            base_digital_option(expiry, OptionType::Call, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(expiry, 1.20, 0.2, 0.02, 0.01); // spot > strike
        let calc = FxDigitalOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, expiry).expect("should succeed");
        approx_eq(pv.amount(), option.payout_amount.amount(), 1e-6);
    }

    #[test]
    fn expired_otm_digital_call_returns_zero() {
        let expiry = date(2025, 1, 3);
        let option =
            base_digital_option(expiry, OptionType::Call, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(expiry, 1.10, 0.2, 0.02, 0.01); // spot < strike
        let calc = FxDigitalOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, expiry).expect("should succeed");
        approx_eq(pv.amount(), 0.0, 1e-6);
    }

    #[test]
    fn greeks_delta_is_positive_for_call() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option =
            base_digital_option(expiry, OptionType::Call, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let greeks = calc
            .compute_greeks(&option, &ctx, as_of)
            .expect("should succeed");
        assert!(greeks.delta > 0.0, "Call delta should be positive");
    }

    #[test]
    fn greeks_delta_is_negative_for_put() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_digital_option(expiry, OptionType::Put, DigitalPayoutType::CashOrNothing);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxDigitalOptionCalculator::default();

        let greeks = calc
            .compute_greeks(&option, &ctx, as_of)
            .expect("should succeed");
        assert!(greeks.delta < 0.0, "Put delta should be negative");
    }

    #[test]
    fn test_curve_dependencies() {
        use crate::instruments::common_impl::traits::CurveDependencies;

        let option = base_digital_option(
            date(2025, 6, 15),
            OptionType::Call,
            DigitalPayoutType::CashOrNothing,
        );
        let deps = option.curve_dependencies();
        assert_eq!(deps.discount_curves.len(), 2);
    }
}
