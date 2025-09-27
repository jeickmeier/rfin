//! FX option calculator implementing Garman–Kohlhagen model.
//!
//! Contains the complex pricing logic separated from the instrument type,
//! following the separation of concerns pattern.

use crate::instruments::common::models::{d1, d2};
use crate::instruments::common::parameters::OptionType;
use crate::instruments::fx_option::FxOption;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::math::solver::{HybridSolver, Solver};
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::{Result, F};

/// Configuration for the FX option calculator.
#[derive(Debug, Clone)]
pub struct FxOptionCalculatorConfig {
    /// Days per year basis for theta scaling (e.g., 365.0).
    pub theta_days_per_year: F,
    /// Initial guess for implied volatility solver.
    pub iv_initial_guess: F,
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
    ) -> Result<(F, F, F, F, F)> {
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
    ) -> Result<(F, F, F, F)> {
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
    pub fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<F> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }

    /// Price using Garman–Kohlhagen with explicit inputs. Convenience for tests.
    pub fn price_gk_with_inputs(
        &self,
        inst: &FxOption,
        spot: F,
        r_d: F,
        r_f: F,
        sigma: F,
        t: F,
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
        target_price: F,
        initial_guess: Option<F>,
    ) -> Result<F> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, t) = self.collect_inputs_no_vol(inst, curves, as_of)?;
        if t <= 0.0 || spot <= 0.0 {
            return Ok(0.0);
        }

        let price_for_sigma = |sigma: F| -> F {
            if sigma <= 0.0 {
                return F::NAN;
            }
            let unit_price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);
            unit_price * inst.notional.amount()
        };

        let target = target_price;
        let f = |x: F| -> F {
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
    pub fn compute_greeks(&self, inst: &FxOption, curves: &MarketContext, as_of: Date) -> Result<FxOptionGreeks> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        // Expired handling
        if t <= 0.0 {
            let spot_gt_strike = spot > inst.strike;
            let delta_unit = match inst.option_type {
                OptionType::Call => {
                    if spot_gt_strike { 1.0 } else { 0.0 }
                }
                OptionType::Put => {
                    if !spot_gt_strike { -1.0 } else { 0.0 }
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
        let vega_unit = spot * exp_rf_t * pdf_d1 * sqrt_t / 100.0; // per 1% vol
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
            OptionType::Call => inst.strike * t * exp_rd_t * cdf_d2 / 100.0,
            OptionType::Put => -inst.strike * t * exp_rd_t * cdf_m_d2 / 100.0,
        };
        let rho_foreign_unit = match inst.option_type {
            OptionType::Call => -spot * t * exp_rf_t * cdf_d1 / 100.0,
            OptionType::Put => spot * t * exp_rf_t * cdf_m_d1 / 100.0,
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
    pub delta: F,
    pub gamma: F,
    pub vega: F,
    pub theta: F,
    pub rho_domestic: F,
    pub rho_foreign: F,
}

#[inline]
fn price_gk_core(spot: F, strike: F, r_d: F, r_f: F, sigma: F, t: F, option_type: OptionType) -> F {
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
