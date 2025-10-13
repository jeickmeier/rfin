//! FX option calculator implementing Garman–Kohlhagen model.
//!
//! Contains the complex pricing logic separated from the instrument type,
//! following the separation of concerns pattern.

use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::common::models::{d1, d2};
use crate::instruments::common::parameters::OptionType;
use crate::instruments::fx_option::FxOption;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::math::solver::{HybridSolver, Solver};
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
    pub config: FxOptionCalculatorConfig,
}

impl FxOptionCalculator {
    /// Create new calculator with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create calculator with custom configuration.
    pub fn with_config(config: FxOptionCalculatorConfig) -> Self {
        Self { config }
    }

    /// Compute present value using Garman–Kohlhagen.
    pub fn npv(&self, inst: &FxOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

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

        let price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);

        Ok(Money::new(
            price * inst.notional.amount(),
            inst.quote_currency,
        ))
    }

    /// Collect standard inputs (spot, domestic/foreign rates, vol, time to expiry).
    pub fn collect_inputs(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        let t = self.year_fraction(as_of, inst.expiry, inst.day_count)?;

        // Discount curves provide domestic and foreign zero rates
        let domestic_disc = curves.get_discount_ref(inst.domestic_disc_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(inst.foreign_disc_id.as_str())?;
        let r_d = domestic_disc.zero(t);
        let r_f = foreign_disc.zero(t);

        // Spot from FX matrix
        let fx_matrix = curves.fx.as_ref().ok_or(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        // Vol either override or surface lookup (clamped)
        let sigma = if let Some(impl_vol) = inst.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface_ref(inst.vol_id)?;
            vol_surface.value_clamped(t, inst.strike)
        };

        Ok((spot, r_d, r_f, sigma, t))
    }

    /// Collect inputs excluding volatility (spot, domestic/foreign rates, time to expiry).
    pub fn collect_inputs_no_vol(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64)> {
        let t = self.year_fraction(as_of, inst.expiry, inst.day_count)?;

        let domestic_disc = curves.get_discount_ref(inst.domestic_disc_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(inst.foreign_disc_id.as_str())?;
        let r_d = domestic_disc.zero(t);
        let r_f = foreign_disc.zero(t);

        let fx_matrix = curves.fx.as_ref().ok_or(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        Ok((spot, r_d, r_f, t))
    }

    /// Utility: compute year fraction using instrument day-count.
    #[inline]
    pub fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<f64> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }

    /// Price using Garman–Kohlhagen with explicit inputs. Convenience for tests.
    pub fn price_gk_with_inputs(
        &self,
        inst: &FxOption,
        spot: f64,
        r_d: f64,
        r_f: f64,
        sigma: f64,
        t: f64,
    ) -> Result<Money> {
        if t <= 0.0 {
            let intrinsic = match inst.option_type {
                OptionType::Call => (spot - inst.strike).max(0.0),
                OptionType::Put => (inst.strike - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * inst.notional.amount(),
                inst.quote_currency,
            ));
        }
        let price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);
        Ok(Money::new(
            price * inst.notional.amount(),
            inst.quote_currency,
        ))
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
        if t <= 0.0 || spot <= 0.0 {
            return Ok(0.0);
        }

        let price_for_sigma = |sigma: f64| -> f64 {
            if sigma <= 0.0 {
                return f64::NAN;
            }
            let unit_price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);
            unit_price * inst.notional.amount()
        };

        let target = target_price;
        let f = |x: f64| -> f64 {
            let sigma = x.exp();
            price_for_sigma(sigma) - target
        };

        // Initial guess: override or surface vol else config default
        let sigma0 = if let Some(v) = inst.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface_ref(inst.vol_id)
                .ok()
                .map(|s| s.value_clamped(t, inst.strike))
                .unwrap_or(self.config.iv_initial_guess)
        };
        let x0 = (initial_guess.unwrap_or(sigma0.max(1e-6))).ln();

        let solver = HybridSolver::new()
            .with_tolerance(1e-10)
            .with_max_iterations(100);
        let root = solver.solve(f, x0)?;
        Ok(root.exp())
    }

    /// Compute greeks with calculator configuration.
    pub fn compute_greeks(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<FxOptionGreeks> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

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

        // Continuous-carried d1/d2
        let d1 = d1(spot, inst.strike, r_d, sigma, t, r_f);
        let d2 = d2(spot, inst.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();
        let exp_rd_t = (-r_d * t).exp();
        let sqrt_t = t.sqrt();
        let pdf_d1 = finstack_core::math::norm_pdf(d1);
        let cdf_d1 = finstack_core::math::norm_cdf(d1);
        let cdf_m_d1 = finstack_core::math::norm_cdf(-d1);
        let cdf_d2 = finstack_core::math::norm_cdf(d2);
        let cdf_m_d2 = finstack_core::math::norm_cdf(-d2);

        // Unit greeks
        let delta_unit = match inst.option_type {
            OptionType::Call => exp_rf_t * cdf_d1,
            OptionType::Put => -exp_rf_t * cdf_m_d1,
        };
        let gamma_unit = if sigma <= 0.0 {
            0.0
        } else {
            exp_rf_t * pdf_d1 / (spot * sigma * sqrt_t)
        };
        let vega_unit = spot * exp_rf_t * pdf_d1 * sqrt_t / DECIMAL_TO_PERCENT; // per 1% vol
        let theta_unit = match inst.option_type {
            OptionType::Call => {
                let term1 = -spot * pdf_d1 * sigma * exp_rf_t / (2.0 * sqrt_t);
                let term2 = r_f * spot * cdf_d1 * exp_rf_t;
                let term3 = -r_d * inst.strike * exp_rd_t * cdf_d2;
                (term1 + term2 + term3) / self.config.theta_days_per_year
            }
            OptionType::Put => {
                let term1 = -spot * pdf_d1 * sigma * exp_rf_t / (2.0 * sqrt_t);
                let term2 = -r_f * spot * cdf_m_d1 * exp_rf_t;
                let term3 = r_d * inst.strike * exp_rd_t * cdf_m_d2;
                (term1 + term2 + term3) / self.config.theta_days_per_year
            }
        };
        let rho_domestic_unit = match inst.option_type {
            OptionType::Call => inst.strike * t * exp_rd_t * cdf_d2 / DECIMAL_TO_PERCENT,
            OptionType::Put => -inst.strike * t * exp_rd_t * cdf_m_d2 / DECIMAL_TO_PERCENT,
        };
        let rho_foreign_unit = match inst.option_type {
            OptionType::Call => -spot * t * exp_rf_t * cdf_d1 / DECIMAL_TO_PERCENT,
            OptionType::Put => spot * t * exp_rf_t * cdf_m_d1 / DECIMAL_TO_PERCENT,
        };

        let scale = inst.notional.amount();
        Ok(FxOptionGreeks {
            delta: delta_unit * scale,
            gamma: gamma_unit * scale,
            vega: vega_unit * scale,
            theta: theta_unit * scale,
            rho_domestic: rho_domestic_unit * scale,
            rho_foreign: rho_foreign_unit * scale,
        })
    }

    #[inline]
    fn validate_currency(&self, inst: &FxOption) -> Result<()> {
        if inst.notional.currency() as i32 != inst.base_currency as i32 {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }
        Ok(())
    }
}

/// Cash greeks for an FX option (scaled by notional amount).
#[derive(Clone, Copy, Debug, Default)]
pub struct FxOptionGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho_domestic: f64,
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
    let d1 = d1(spot, strike, r_d, sigma, t, r_f);
    let d2 = d2(spot, strike, r_d, sigma, t, r_f);
    match option_type {
        OptionType::Call => {
            spot * (-r_f * t).exp() * finstack_core::math::norm_cdf(d1)
                - strike * (-r_d * t).exp() * finstack_core::math::norm_cdf(d2)
        }
        OptionType::Put => {
            strike * (-r_d * t).exp() * finstack_core::math::norm_cdf(-d2)
                - spot * (-r_f * t).exp() * finstack_core::math::norm_cdf(-d1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::{
        common::traits::Attributes, OptionType, PricingOverrides, SettlementType,
    };
    use crate::instruments::{ExerciseStyle, FxOption};
    use finstack_core::{
        currency::Currency,
        dates::{Date, DayCount},
        market_data::{
            context::MarketContext, scalars::MarketScalar, surfaces::vol_surface::VolSurface,
            term_structures::discount_curve::DiscountCurve,
        },
        money::{
            fx::{FxConversionPolicy, FxMatrix, FxProvider},
            Money,
        },
        types::{CurveId, InstrumentId},
    };
    use std::sync::Arc;
    use time::Month;

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

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn flat_discount(id: &str, as_of: Date, rate: f64) -> DiscountCurve {
        DiscountCurve::builder(id)
            .base_date(as_of)
            .knots([(0.0, 1.0), (1.0, (-rate).exp())])
            .build()
            .unwrap()
    }

    fn vol_surface(vol: f64) -> VolSurface {
        VolSurface::builder(VOL_ID)
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[0.8, 0.9, 1.0, 1.1, 1.2])
            .row(&[vol; 5])
            .row(&[vol; 5])
            .row(&[vol; 5])
            .row(&[vol; 5])
            .build()
            .unwrap()
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
        MarketContext::new()
            .insert_discount(flat_discount(DOMESTIC_ID, as_of, r_domestic))
            .insert_discount(flat_discount(FOREIGN_ID, as_of, r_foreign))
            .insert_surface(vol_surface(vol))
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
            .domestic_disc_id(CurveId::new(DOMESTIC_ID))
            .foreign_disc_id(CurveId::new(FOREIGN_ID))
            .vol_id(VOL_ID)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap()
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
        let calc = FxOptionCalculator::new();

        let pv = calc.npv(&option, &ctx, as_of).unwrap();
        let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&option, &ctx, as_of).unwrap();
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
        let calc = FxOptionCalculator::new();

        let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&option, &ctx, as_of).unwrap();
        approx_eq(spot, 1.2, 1e-12);
        approx_eq(r_d, 0.025, 2e-4);
        approx_eq(r_f, 0.015, 2e-4);
        approx_eq(sigma, 0.35, 1e-12);
        assert!(t > 0.4);

        option.pricing_overrides.implied_volatility = Some(0.5);
        let (_, _, _, override_sigma, _) = calc.collect_inputs(&option, &ctx, as_of).unwrap();
        approx_eq(override_sigma, 0.5, 1e-12);

        let (_, _, _, t_only) = calc.collect_inputs_no_vol(&option, &ctx, as_of).unwrap();
        assert!((t_only - t).abs() < 1e-12);
    }

    #[test]
    fn implied_volatility_recovers_target_price() {
        let as_of = date(2025, 2, 10);
        let expiry = date(2025, 8, 10);
        let option = base_option(expiry, OptionType::Call);
        let ctx = market_context(as_of, 1.17, 0.25, 0.02, 0.012);
        let calc = FxOptionCalculator::new();

        let pv = calc.npv(&option, &ctx, as_of).unwrap();
        let sigma = calc
            .implied_vol(&option, &ctx, as_of, pv.amount(), Some(0.15))
            .unwrap();
        approx_eq(sigma, 0.25, 1e-6);
    }

    #[test]
    fn compute_greeks_align_with_finite_differences() {
        let as_of = date(2025, 1, 15);
        let expiry = date(2025, 6, 15);
        let option = base_option(expiry, OptionType::Call);
        let ctx = market_context(as_of, 1.16, 0.18, 0.028, 0.012);
        let calc = FxOptionCalculator::with_config(FxOptionCalculatorConfig {
            theta_days_per_year: 365.0,
            iv_initial_guess: 0.2,
        });

        let greeks = calc.compute_greeks(&option, &ctx, as_of).unwrap();
        let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&option, &ctx, as_of).unwrap();
        let d1 = crate::instruments::common::models::d1(spot, option.strike, r_d, sigma, t, r_f);
        let d2 = crate::instruments::common::models::d2(spot, option.strike, r_d, sigma, t, r_f);
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
        let calc = FxOptionCalculator::new();

        let pv = calc.npv(&option, &ctx, as_of).unwrap();
        let intrinsic = (option.strike - 1.05).max(0.0) * option.notional.amount();
        approx_eq(pv.amount(), intrinsic, 1e-6);

        let greeks = calc.compute_greeks(&option, &ctx, as_of).unwrap();
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
        let calc = FxOptionCalculator::new();

        let result = calc.npv(&option, &ctx, date(2025, 1, 1));
        assert!(result.is_err());
    }
}
