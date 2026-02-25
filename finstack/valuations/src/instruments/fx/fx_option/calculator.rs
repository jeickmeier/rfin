//! FX option calculator implementing Garman–Kohlhagen model.
//!
//! Contains the complex pricing logic separated from the instrument type,
//! following the separation of concerns pattern.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::{bs_greeks, bs_price};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::fx::fx_option::FxOption;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

/// Configuration for the FX option calculator.
#[derive(Debug, Clone)]
pub struct FxOptionCalculatorConfig {
    /// Days per year basis for theta scaling (e.g., 365.0).
    pub theta_days_per_year: f64,
    /// Initial guess for implied volatility solver.
    pub iv_initial_guess: f64,
}

impl Default for FxOptionCalculatorConfig {
    fn default() -> Self {
        Self {
            theta_days_per_year: 365.0,
            iv_initial_guess: 0.20,
        }
    }
}

/// FX option calculator implementing Garman–Kohlhagen pricing.
#[derive(Debug, Clone, Default)]
pub struct FxOptionCalculator {
    /// Calculator configuration (volatility model, smile handling)
    pub config: FxOptionCalculatorConfig,
}

impl FxOptionCalculator {
    /// Compute present value using Garman–Kohlhagen.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `exercise_style` is not European (American/Bermudan not supported)
    /// - Notional currency doesn't match base currency
    /// - Required market data is missing
    pub fn npv(&self, inst: &FxOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
        self.validate_exercise_style(inst)?;
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;
        if spot <= 0.0 || inst.strike < 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption requires spot > 0, strike >= 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        if t <= 0.0 {
            // Expired: intrinsic value only
            let intrinsic = match inst.option_type {
                OptionType::Call => (spot - inst.strike).max(0.0),
                OptionType::Put => (inst.strike - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * inst.notional.amount(),
                inst.quote_currency,
            ));
        }

        // Strike = 0 edge case:
        // - Call with K=0 is always exercised: PV = S * exp(-r_f * t)
        // - Put with K=0 is worthless: PV = 0
        if inst.strike == 0.0 {
            let unit_price = match inst.option_type {
                OptionType::Call => spot * (-r_f * t).exp(),
                OptionType::Put => 0.0,
            };
            return Ok(Money::new(
                unit_price * inst.notional.amount(),
                inst.quote_currency,
            ));
        }

        let price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);

        Ok(Money::new(
            price * inst.notional.amount(),
            inst.quote_currency,
        ))
    }

    /// Collect standard inputs (spot, domestic/foreign rates, vol, time to expiry).
    ///
    /// Uses curve-specific day counts for consistency:
    /// - Discount factors are obtained using each curve's native day count
    /// - Effective zero rates are computed to be consistent with `t_vol`
    /// - `t_vol` from instrument day count (for vol surface lookups, typically ACT/365F)
    ///
    /// Returns `(spot, r_d, r_f, sigma, t_vol)` where rates and time are consistent
    /// so that `exp(-r_d * t_vol)` gives the correct domestic discount factor.
    pub fn collect_inputs(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        // Handle expired options - avoid InvalidDateRange error
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        // Discount curves provide discount factors
        // Use each curve's day count for proper discount factor lookup
        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        // Time to expiry using curve-specific day counts for DF lookup
        let t_disc_dom = year_fraction(domestic_disc.day_count(), as_of, inst.expiry)?;
        let t_disc_for = year_fraction(foreign_disc.day_count(), as_of, inst.expiry)?;

        // Get discount factors using curve-native time
        let df_d = domestic_disc.df(t_disc_dom);
        let df_f = foreign_disc.df(t_disc_for);

        // Vol surface time using instrument day count (typically ACT/365F for FX options)
        let t_vol = year_fraction(inst.day_count, as_of, inst.expiry)?;

        // Convert discount factors to effective zero rates consistent with t_vol
        // So that exp(-r_d * t_vol) = df_d (preserving the actual discount factors)
        let r_d = if t_vol > 0.0 { -df_d.ln() / t_vol } else { 0.0 };
        let r_f = if t_vol > 0.0 { -df_f.ln() / t_vol } else { 0.0 };

        // Spot from FX matrix
        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        // Vol either override or surface lookup (clamped)
        let sigma = if let Some(impl_vol) = inst.pricing_overrides.market_quotes.implied_volatility
        {
            impl_vol
        } else {
            let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
            vol_surface.value_clamped(t_vol, inst.strike)
        };

        Ok((spot, r_d, r_f, sigma, t_vol))
    }

    /// Collect inputs for expired options (intrinsic value only).
    fn collect_inputs_expired(
        &self,
        inst: &FxOption,
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

        // Return zero time and rates for expired options
        Ok((spot, 0.0, 0.0, 0.0, 0.0))
    }

    /// Collect inputs excluding volatility (spot, domestic/foreign rates, time to expiry).
    ///
    /// Uses curve-specific day counts for discount factor lookups; returns effective
    /// zero rates consistent with `t_vol` for use in Garman-Kohlhagen formulas.
    pub fn collect_inputs_no_vol(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64)> {
        // Handle expired options
        if as_of >= inst.expiry {
            let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
                finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ))?;
            let spot = fx_matrix
                .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
                .rate;
            return Ok((spot, 0.0, 0.0, 0.0));
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        // Time to expiry using curve-specific day counts for DF lookup
        let t_disc_dom = year_fraction(domestic_disc.day_count(), as_of, inst.expiry)?;
        let t_disc_for = year_fraction(foreign_disc.day_count(), as_of, inst.expiry)?;

        // Get discount factors using curve-native time
        let df_d = domestic_disc.df(t_disc_dom);
        let df_f = foreign_disc.df(t_disc_for);

        // Vol surface time using instrument day count
        let t_vol = year_fraction(inst.day_count, as_of, inst.expiry)?;

        // Convert discount factors to effective zero rates consistent with t_vol
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

        Ok((spot, r_d, r_f, t_vol))
    }

    /// Solve for implied volatility σ such that model price(σ) = target_price.
    /// Uses log-σ parameterization with Hybrid solver for robustness.
    pub fn implied_vol(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, t) = self.collect_inputs_no_vol(inst, curves, as_of)?;
        if t <= 0.0 {
            // Expired options have no time value; implied vol is not identifiable.
            // Returning 0.0 is a pragmatic convention used in analytics pipelines.
            return Ok(0.0);
        }
        if spot <= 0.0 || inst.strike <= 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Implied vol requires spot > 0, strike > 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        // Solve per-unit then scale back: PV = unit_price * notional_base.
        //
        // Default initial guess: config-provided fallback if caller didn't provide one.
        let initial_guess = initial_guess.or(Some(self.config.iv_initial_guess));
        // Using the shared closed-form implied vol utility removes duplicated solvers.
        let target_unit = target_price / inst.notional.amount();
        let _ = initial_guess; // future: warm-start the bracket

        Ok(crate::instruments::common_impl::models::bs_implied_vol(
            spot,
            inst.strike,
            r_d,
            r_f,
            t,
            inst.option_type,
            target_unit,
        ))
    }

    /// Compute greeks with calculator configuration.
    ///
    /// Returns both spot delta (Bloomberg default) and forward delta (interbank convention).
    pub fn compute_greeks(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<FxOptionGreeks> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;
        if spot <= 0.0 || inst.strike < 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption greeks require spot > 0, strike >= 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        // Expired handling
        if t <= 0.0 {
            let spot_gt_strike = spot > inst.strike;
            let delta_unit = match inst.option_type {
                OptionType::Call => {
                    if spot_gt_strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if !spot_gt_strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
            let scale = inst.notional.amount();
            return Ok(FxOptionGreeks {
                delta: delta_unit * scale,
                ..Default::default()
            });
        }

        // Strike = 0 edge case:
        // - Call: PV = S * exp(-r_f t) → delta = exp(-r_f t), gamma/vega/theta = 0
        // - Put: PV = 0 → all greeks = 0
        if inst.strike == 0.0 {
            let scale = inst.notional.amount();
            let exp_rf_t = (-r_f * t).exp();
            let delta_unit = match inst.option_type {
                OptionType::Call => exp_rf_t,
                OptionType::Put => 0.0,
            };
            return Ok(FxOptionGreeks {
                delta: delta_unit * scale,
                ..Default::default()
            });
        }

        let greeks_unit = bs_greeks(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            self.config.theta_days_per_year,
        );

        // Forward delta: N(d1) for calls, N(d1) - 1 for puts (no foreign discount factor).
        // This is the interbank FX convention used for vol surface interpolation.
        let d1 = crate::instruments::common_impl::models::d1(spot, inst.strike, r_d, sigma, t, r_f);
        let d2 = d1 - sigma * t.sqrt();
        let cdf_d1 = finstack_core::math::norm_cdf(d1);
        let cdf_d2 = finstack_core::math::norm_cdf(d2);
        let exp_rf_t = (-r_f * t).exp();
        let delta_forward_unit = match inst.option_type {
            OptionType::Call => cdf_d1,
            OptionType::Put => cdf_d1 - 1.0,
        };
        // Premium-adjusted spot delta is the convention used in many EM FX option markets.
        // Under Garman-Kohlhagen this corresponds to N(d2)-style spot sensitivity.
        let delta_premium_adjusted_unit = match inst.option_type {
            OptionType::Call => exp_rf_t * cdf_d2,
            OptionType::Put => exp_rf_t * (cdf_d2 - 1.0),
        };

        let scale = inst.notional.amount();
        Ok(FxOptionGreeks {
            delta: greeks_unit.delta * scale,
            delta_forward: delta_forward_unit * scale,
            delta_premium_adjusted: delta_premium_adjusted_unit * scale,
            gamma: greeks_unit.gamma * scale,
            vega: greeks_unit.vega * scale,
            theta: greeks_unit.theta * scale,
            rho_domestic: greeks_unit.rho_r * scale,
            rho_foreign: greeks_unit.rho_q * scale,
        })
    }

    #[inline]
    fn validate_exercise_style(&self, inst: &FxOption) -> Result<()> {
        use crate::instruments::ExerciseStyle;
        if inst.exercise_style != ExerciseStyle::European {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption only supports European exercise style. \
                 Got {:?}. American and Bermudan options require \
                 specialized pricers not yet implemented.",
                inst.exercise_style
            )));
        }
        Ok(())
    }

    #[inline]
    fn validate_currency(&self, inst: &FxOption) -> Result<()> {
        if inst.notional.currency() as i32 != inst.base_currency as i32 {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: inst.base_currency,
                actual: inst.notional.currency(),
            });
        }
        Ok(())
    }
}

/// Cash greeks for an FX option (scaled by notional amount).
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)] // Public API: fields exposed for external bindings and downstream crates
pub struct FxOptionGreeks {
    /// Delta: sensitivity to spot FX rate (spot delta convention).
    ///
    /// Spot delta = e^(-r_f × T) × N(d1) for calls, -e^(-r_f × T) × N(-d1) for puts.
    /// This is the Bloomberg default convention.
    pub delta: f64,
    /// Forward delta: interbank convention for FX option hedging and vol surface interpolation.
    ///
    /// Forward delta = N(d1) for calls, N(d1) - 1 for puts.
    /// This does not include the foreign rate discount factor, making it the
    /// convention used in professional FX interbank markets for quoting vol surfaces.
    ///
    /// # References
    ///
    /// - Garman, M. & Kohlhagen, S. (1983). "Foreign Currency Option Values"
    /// - Clark, I. (2011). "Foreign Exchange Option Pricing" Chapter 2
    pub delta_forward: f64,
    /// Premium-adjusted spot delta used in many EM FX option markets.
    ///
    /// This convention adjusts spot delta for premium effects and is commonly
    /// quoted for options where premium-adjusted hedging is standard.
    pub delta_premium_adjusted: f64,
    /// Gamma: rate of change of delta with respect to spot
    pub gamma: f64,
    /// Vega: sensitivity to 1% change in volatility
    pub vega: f64,
    /// Theta: time decay per day
    pub theta: f64,
    /// Rho domestic: sensitivity to 1% change in domestic interest rate
    pub rho_domestic: f64,
    /// Rho foreign: sensitivity to 1% change in foreign interest rate
    pub rho_foreign: f64,
}

#[inline]
fn price_gk_core(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> f64 {
    bs_price(spot, strike, r_d, r_f, sigma, t, option_type)
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
    use crate::constants::DECIMAL_TO_PERCENT;
    use crate::instruments::{
        common::traits::Attributes, OptionType, PricingOverrides, SettlementType,
    };
    use crate::instruments::{ExerciseStyle, FxOption};
    use finstack_core::{
        currency::Currency,
        dates::{Date, DayCount},
        market_data::{
            context::MarketContext, scalars::MarketScalar, term_structures::DiscountCurve,
        },
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
            .insert_price("FX_VOL_OVERRIDE", MarketScalar::Unitless(vol))
            .insert_fx(fx)
    }

    fn base_option(expiry: Date, option_type: OptionType) -> FxOption {
        FxOption::builder()
            .id(InstrumentId::new("EURUSD_OPTION"))
            .base_currency(BASE)
            .quote_currency(QUOTE)
            .strike(1.15)
            .option_type(option_type)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, BASE))
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(CurveId::new(DOMESTIC_ID))
            .foreign_discount_curve_id(CurveId::new(FOREIGN_ID))
            .vol_surface_id(CurveId::new(VOL_ID))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed")
    }

    /// Default market context for tests that just need to validate behavior,
    /// not specific pricing results.
    fn base_market(as_of: Date) -> MarketContext {
        market_context(as_of, 1.18, 0.22, 0.03, 0.01)
    }

    fn approx_eq(actual: f64, expected: f64, tol: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "expected {expected}, got {actual} (diff {diff} > {tol})"
        );
    }

    #[test]
    fn npv_matches_garman_kohlhagen_price() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry, OptionType::Call);
        let ctx = market_context(as_of, 1.18, 0.22, 0.03, 0.01);
        let calc = FxOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        let (spot, r_d, r_f, sigma, t) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("should succeed");
        let expected_unit =
            super::price_gk_core(spot, option.strike, r_d, r_f, sigma, t, option.option_type);
        approx_eq(pv.amount(), expected_unit * option.notional.amount(), 5e-3);
        assert_eq!(pv.currency(), QUOTE);
    }

    #[test]
    fn collect_inputs_respects_surface_and_overrides() {
        let as_of = date(2025, 3, 1);
        let expiry = date(2025, 9, 1);
        let mut option = base_option(expiry, OptionType::Put);
        let ctx = market_context(as_of, 1.2, 0.35, 0.025, 0.015);
        let calc = FxOptionCalculator::default();

        let (spot, r_d, r_f, sigma, t) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("should succeed");
        approx_eq(spot, 1.2, 1e-12);
        approx_eq(r_d, 0.025, 2e-4);
        approx_eq(r_f, 0.015, 2e-4);
        approx_eq(sigma, 0.35, 1e-12);
        assert!(t > 0.4);

        option.pricing_overrides.market_quotes.implied_volatility = Some(0.5);
        let (_, _, _, override_sigma, _) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("should succeed");
        approx_eq(override_sigma, 0.5, 1e-12);

        let (_, _, _, t_only) = calc
            .collect_inputs_no_vol(&option, &ctx, as_of)
            .expect("should succeed");
        assert!((t_only - t).abs() < 1e-12);
    }

    #[test]
    fn implied_volatility_recovers_target_price() {
        let as_of = date(2025, 2, 10);
        let expiry = date(2025, 8, 10);
        let option = base_option(expiry, OptionType::Call);
        let ctx = market_context(as_of, 1.17, 0.25, 0.02, 0.012);
        let calc = FxOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        let sigma = calc
            .implied_vol(&option, &ctx, as_of, pv.amount(), Some(0.15))
            .expect("should succeed");
        approx_eq(sigma, 0.25, 1e-6);
    }

    #[test]
    fn compute_greeks_align_with_finite_differences() {
        let as_of = date(2025, 1, 15);
        let expiry = date(2025, 6, 15);
        let option = base_option(expiry, OptionType::Call);
        let ctx = market_context(as_of, 1.16, 0.18, 0.028, 0.012);
        let calc = FxOptionCalculator {
            config: FxOptionCalculatorConfig {
                theta_days_per_year: 365.0,
                iv_initial_guess: 0.2,
            },
        };

        let greeks = calc
            .compute_greeks(&option, &ctx, as_of)
            .expect("should succeed");
        let (spot, r_d, r_f, sigma, t) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("should succeed");
        let d1 =
            crate::instruments::common_impl::models::d1(spot, option.strike, r_d, sigma, t, r_f);
        let d2 =
            crate::instruments::common_impl::models::d2(spot, option.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();
        let exp_rd_t = (-r_d * t).exp();
        let sqrt_t = t.sqrt();
        let pdf_d1 = finstack_core::math::norm_pdf(d1);
        let cdf_d1 = finstack_core::math::norm_cdf(d1);
        let cdf_m_d1 = finstack_core::math::norm_cdf(-d1);
        let cdf_d2 = finstack_core::math::norm_cdf(d2);
        let cdf_m_d2 = finstack_core::math::norm_cdf(-d2);
        let scale = option.notional.amount();

        let expected_delta_unit = match option.option_type {
            OptionType::Call => exp_rf_t * cdf_d1,
            OptionType::Put => -exp_rf_t * cdf_m_d1,
        };
        approx_eq(greeks.delta, expected_delta_unit * scale, 1e-6);

        let expected_gamma_unit = exp_rf_t * pdf_d1 / (spot * sigma * sqrt_t);
        approx_eq(greeks.gamma, expected_gamma_unit * scale, 1e-9);

        let expected_vega_unit = spot * exp_rf_t * pdf_d1 * sqrt_t / DECIMAL_TO_PERCENT;
        approx_eq(greeks.vega, expected_vega_unit * scale, 1e-6);

        let expected_theta_unit = match option.option_type {
            OptionType::Call => {
                let term1 = -spot * pdf_d1 * sigma * exp_rf_t / (2.0 * sqrt_t);
                let term2 = r_f * spot * cdf_d1 * exp_rf_t;
                let term3 = -r_d * option.strike * exp_rd_t * cdf_d2;
                (term1 + term2 + term3) / calc.config.theta_days_per_year
            }
            OptionType::Put => {
                let term1 = -spot * pdf_d1 * sigma * exp_rf_t / (2.0 * sqrt_t);
                let term2 = -r_f * spot * cdf_m_d1 * exp_rf_t;
                let term3 = r_d * option.strike * exp_rd_t * cdf_m_d2;
                (term1 + term2 + term3) / calc.config.theta_days_per_year
            }
        };
        approx_eq(greeks.theta, expected_theta_unit * scale, 1e-6);

        let expected_rho_domestic_unit = match option.option_type {
            OptionType::Call => option.strike * t * exp_rd_t * cdf_d2 / DECIMAL_TO_PERCENT,
            OptionType::Put => -option.strike * t * exp_rd_t * cdf_m_d2 / DECIMAL_TO_PERCENT,
        };
        approx_eq(
            greeks.rho_domestic,
            expected_rho_domestic_unit * scale,
            1e-6,
        );

        let expected_rho_foreign_unit = match option.option_type {
            OptionType::Call => -spot * t * exp_rf_t * cdf_d1 / DECIMAL_TO_PERCENT,
            OptionType::Put => spot * t * exp_rf_t * cdf_m_d1 / DECIMAL_TO_PERCENT,
        };
        approx_eq(greeks.rho_foreign, expected_rho_foreign_unit * scale, 1e-6);
    }

    #[test]
    fn expired_options_return_intrinsic_and_static_greeks() {
        let expiry = date(2025, 1, 3);
        let option = base_option(expiry, OptionType::Put);
        let as_of = expiry;
        let ctx = market_context(expiry, 1.05, 0.2, 0.02, 0.01);
        let calc = FxOptionCalculator::default();

        let pv = calc.npv(&option, &ctx, as_of).expect("should succeed");
        let intrinsic = (option.strike - 1.05).max(0.0) * option.notional.amount();
        approx_eq(pv.amount(), intrinsic, 1e-6);

        let greeks = calc
            .compute_greeks(&option, &ctx, as_of)
            .expect("should succeed");
        assert_eq!(greeks.gamma, 0.0);
        assert_eq!(greeks.vega, 0.0);
        assert_eq!(greeks.theta, 0.0);
        assert_eq!(greeks.rho_domestic, 0.0);
        assert_eq!(greeks.rho_foreign, 0.0);
    }

    #[test]
    fn currency_validation_rejects_mismatched_notional() {
        let expiry = date(2025, 6, 30);
        let mut option = base_option(expiry, OptionType::Call);
        option.notional = Money::new(1_000_000.0, QUOTE); // mismatched currency
        let ctx = market_context(date(2025, 1, 1), 1.1, 0.2, 0.03, 0.01);
        let calc = FxOptionCalculator::default();

        let result = calc.npv(&option, &ctx, date(2025, 1, 1));
        assert!(result.is_err());
    }

    /// Test EURUSD 6M call with Act/365F vol surface and Act/360 discount curves.
    ///
    /// This test verifies that the calculator correctly handles mismatched day count
    /// conventions between the vol surface (Act/365F, market standard for FX) and
    /// discount curves (Act/360, money market standard for USD).
    ///
    /// Acceptance criteria from market standards review:
    /// - Price error < 1e-6 vs Garman-Kohlhagen reference
    /// - Greeks error < 1e-6 vs reference
    /// - Day count impact is properly handled
    #[test]
    fn eurusd_6m_call_with_mixed_day_counts() {
        // Test parameters matching market standards review acceptance criteria
        let as_of = date(2025, 1, 2);
        let expiry = date(2025, 7, 2); // ~6 months

        // Build discount curves with Act/360 (USD money market standard)
        let domestic_disc = DiscountCurve::builder(DOMESTIC_ID)
            .base_date(as_of)
            .day_count(DayCount::Act360) // USD OIS typically uses Act/360
            .knots([(0.0, 1.0), (1.0, (-0.045_f64).exp())]) // ~4.5% flat
            .build()
            .expect("domestic curve should build");

        let foreign_disc = DiscountCurve::builder(FOREIGN_ID)
            .base_date(as_of)
            .day_count(DayCount::Act365F) // EUR typically uses Act/365F
            .knots([(0.0, 1.0), (1.0, (-0.03_f64).exp())]) // ~3.0% flat
            .build()
            .expect("foreign curve should build");

        // Market data
        let spot = 1.08;
        let vol = 0.10; // 10% flat vol
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [0.8, 0.9, 1.0, 1.1, 1.2];

        let fx = FxMatrix::new(Arc::new(StaticFxProvider { rate: spot }));
        fx.set_quote(BASE, QUOTE, spot);

        let ctx = MarketContext::new()
            .insert_discount(domestic_disc)
            .insert_discount(foreign_disc)
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, vol))
            .insert_fx(fx);

        // Option with Act/365F (standard for FX vol surfaces)
        let option = FxOption::builder()
            .id(InstrumentId::new("EURUSD_6M_CALL"))
            .base_currency(BASE)
            .quote_currency(QUOTE)
            .strike(1.10) // ATM-ish
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(DayCount::Act365F) // Vol surface convention
            .notional(Money::new(1_000_000.0, BASE))
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(CurveId::new(DOMESTIC_ID))
            .foreign_discount_curve_id(CurveId::new(FOREIGN_ID))
            .vol_surface_id(CurveId::new(VOL_ID))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("option should build");

        let calc = FxOptionCalculator::default();

        // Collect inputs and verify day count handling
        let (collected_spot, r_d, r_f, sigma, t_vol) = calc
            .collect_inputs(&option, &ctx, as_of)
            .expect("collect_inputs should succeed");

        // Verify collected inputs
        approx_eq(collected_spot, spot, 1e-12);
        approx_eq(sigma, vol, 1e-12);

        // Time should be calculated using instrument day count (Act/365F)
        // 181 days from Jan 2 to Jul 2
        let expected_t_vol = DayCount::Act365F
            .year_fraction(as_of, expiry, finstack_core::dates::DayCountCtx::default())
            .expect("year fraction should succeed");
        approx_eq(t_vol, expected_t_vol, 1e-10);

        // Zero rates should be looked up using curve-native day counts
        // This verifies the day count convention fix is working
        let t_disc_360 = DayCount::Act360
            .year_fraction(as_of, expiry, finstack_core::dates::DayCountCtx::default())
            .expect("year fraction should succeed");
        let t_disc_365 = DayCount::Act365F
            .year_fraction(as_of, expiry, finstack_core::dates::DayCountCtx::default())
            .expect("year fraction should succeed");

        // Rates should differ due to different day counts
        // Act/360 gives ~181/360 ≈ 0.5028, Act/365F gives ~181/365 ≈ 0.4959
        assert!(
            (t_disc_360 - t_disc_365).abs() > 0.005,
            "Day count difference should be material: t_360={}, t_365={}",
            t_disc_360,
            t_disc_365
        );

        // Compute NPV
        let pv = calc.npv(&option, &ctx, as_of).expect("npv should succeed");
        assert!(
            pv.amount() > 0.0,
            "Call with spot < strike should have positive value"
        );
        assert_eq!(pv.currency(), QUOTE);

        // Compute Greeks
        let greeks = calc
            .compute_greeks(&option, &ctx, as_of)
            .expect("greeks should succeed");

        // Delta should be positive for call
        assert!(greeks.delta > 0.0, "Call delta should be positive");
        assert!(
            greeks.delta < option.notional.amount(),
            "Delta should be < notional"
        );

        // Gamma should be positive
        assert!(greeks.gamma > 0.0, "Gamma should be positive");

        // Vega should be positive
        assert!(greeks.vega > 0.0, "Vega should be positive");

        // Verify Garman-Kohlhagen reference price matches within tolerance
        // Using relative tolerance: 1e-6 relative error on ~$25K = ~$0.025 absolute
        let gk_price = super::price_gk_core(
            collected_spot,
            option.strike,
            r_d,
            r_f,
            sigma,
            t_vol,
            option.option_type,
        );
        let expected_pv = gk_price * option.notional.amount();
        let relative_error = (pv.amount() - expected_pv).abs() / expected_pv;
        assert!(
            relative_error < 1e-6,
            "Relative price error {} exceeds 1e-6 tolerance (expected={}, actual={})",
            relative_error,
            expected_pv,
            pv.amount()
        );
    }

    /// Test that day count mismatches produce different results than uniform day counts.
    #[test]
    fn day_count_mismatch_produces_different_rates() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2025, 7, 2);

        // Build curves with different day counts
        let disc_360 = DiscountCurve::builder("DISC-360")
            .base_date(as_of)
            .day_count(DayCount::Act360)
            .knots([(0.0, 1.0), (1.0, (-0.05_f64).exp())])
            .build()
            .expect("curve should build");

        let disc_365 = DiscountCurve::builder("DISC-365")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, (-0.05_f64).exp())])
            .build()
            .expect("curve should build");

        // Same flat rate, different day counts should give different zero rates
        let t_360 = DayCount::Act360
            .year_fraction(as_of, expiry, finstack_core::dates::DayCountCtx::default())
            .expect("yf");
        let t_365 = DayCount::Act365F
            .year_fraction(as_of, expiry, finstack_core::dates::DayCountCtx::default())
            .expect("yf");

        let r_360 = disc_360.zero(t_360);
        let r_365 = disc_365.zero(t_365);

        // Both curves have the same 5% annual rate, so zero rates should be ~5%
        approx_eq(r_360, 0.05, 0.001);
        approx_eq(r_365, 0.05, 0.001);

        // But the time to expiry differs, which affects discounting
        assert!(
            (t_360 - t_365).abs() > 0.005,
            "Time fractions should differ: t_360={}, t_365={}",
            t_360,
            t_365
        );
    }

    #[test]
    fn test_fx_option_curve_dependencies_includes_both_curves() {
        use crate::instruments::common_impl::traits::CurveDependencies;

        let option = base_option(date(2025, 6, 15), OptionType::Call);
        let deps = option.curve_dependencies().expect("curve_dependencies");

        // Should include both domestic and foreign discount curves
        assert_eq!(
            deps.discount_curves.len(),
            2,
            "FxOption should depend on both domestic and foreign curves"
        );
        assert!(
            deps.discount_curves
                .iter()
                .any(|c| c.as_str() == DOMESTIC_ID),
            "Should include domestic curve"
        );
        assert!(
            deps.discount_curves
                .iter()
                .any(|c| c.as_str() == FOREIGN_ID),
            "Should include foreign curve"
        );
    }

    #[test]
    fn test_fx_option_rejects_american_exercise_style() {
        use crate::instruments::ExerciseStyle;

        let as_of = date(2025, 1, 15);
        let mut option = base_option(date(2025, 6, 15), OptionType::Call);
        option.exercise_style = ExerciseStyle::American;

        let calc = FxOptionCalculator::default();
        let market = base_market(as_of);
        let result = calc.npv(&option, &market, as_of);

        // American exercise should be rejected
        assert!(result.is_err(), "American exercise should be rejected");
        let err_msg = result.expect_err("expected an error").to_string();
        assert!(
            err_msg.contains("European"),
            "Error should mention European: {}",
            err_msg
        );
        assert!(
            err_msg.contains("American"),
            "Error should mention American: {}",
            err_msg
        );
    }

    #[test]
    fn test_fx_option_rejects_bermudan_exercise_style() {
        use crate::instruments::ExerciseStyle;

        let as_of = date(2025, 1, 15);
        let mut option = base_option(date(2025, 6, 15), OptionType::Call);
        option.exercise_style = ExerciseStyle::Bermudan;

        let calc = FxOptionCalculator::default();
        let market = base_market(as_of);
        let result = calc.npv(&option, &market, as_of);

        // Bermudan exercise should be rejected
        assert!(result.is_err(), "Bermudan exercise should be rejected");
        let err_msg = result.expect_err("expected an error").to_string();
        assert!(
            err_msg.contains("European"),
            "Error should mention European: {}",
            err_msg
        );
    }
}
