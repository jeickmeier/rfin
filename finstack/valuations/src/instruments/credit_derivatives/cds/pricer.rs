//! Core CDS pricing engine and helpers.
//!
//! Provides deterministic valuation for Credit Default Swaps (CDS) with
//! support for ISDA-style premium schedules, accrual-on-default, and
//! multiple numerical integration methods for the protection leg.
//!
//! The engine exposes present value calculations for the protection and
//! premium legs, par spread, risky annuity, PV01/CS01, and a simple
//! bootstrapping helper for hazard curves. Heavy numerics are kept here to
//! isolate pricing policy from instrument data shapes.
//!
//! # Par Spread Calculation
//!
//! The par spread is the spread at which the CDS has zero initial value (i.e.,
//! protection leg PV equals premium leg PV). It is calculated as:
//!
//! ```text
//! Par Spread = Protection_PV / RPV01
//! ```
//!
//! where RPV01 (Risky PV01 or Risky Duration) is defined as:
//!
//! ```text
//! RPV01 = Σᵢ DF(tᵢ) × SP(tᵢ) × Δt(tᵢ₋₁, tᵢ)
//! ```
//!
//! - **DF(t)**: Discount factor from valuation date to time t
//! - **SP(t)**: Survival probability to time t (from hazard curve)
//! - **Δt**: Day count fraction for the accrual period
//!
//! This is the **Risky Annuity** excluding accrual-on-default, which matches
//! the ISDA CDS Standard Model convention.
//!
//! ## ISDA vs Bloomberg CDSW Methodology
//!
//! | Methodology | Denominator | Use Case |
//! |-------------|-------------|----------|
//! | ISDA Standard Model | Risky Annuity only | Default, curve building |
//! | Bloomberg CDSW | Includes accrual-on-default | Trading desk analytics |
//!
//! The difference is typically:
//! - **< 1bp** for investment grade credits (hazard rate < 1%)
//! - **2-5 bps** for high yield/distressed credits (hazard rate > 3%)
//!
//! To use Bloomberg CDSW-style calculations, set `par_spread_uses_full_premium = true`
//! in the [`CDSPricerConfig`].
//!
//! ## References
//!
//! - ISDA CDS Standard Model (Markit, 2009)
//! - O'Kane, D. "Modelling Single-name and Multi-name Credit Derivatives" (2008), Chapter 5
//! - Hull, J.C. & White, A. "Valuing Credit Default Swaps I: No Counterparty Default Risk"
#![allow(dead_code)] // Public API items may be used by external bindings or tests
use crate::constants::{
    isda, numerical, time as time_constants, NUMERICAL_TOLERANCE, ONE_BASIS_POINT,
};
use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use finstack_core::currency::Currency;
use finstack_core::dates::DateExt;
use finstack_core::dates::{adjust, next_cds_date, Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::money::Money;
use finstack_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Numerical integration method for protection leg.
///
/// Different integration methods trade off accuracy against speed. Use
/// [`IntegrationMethod::recommended`] for guidance based on instrument characteristics.
///
/// # Method Comparison
///
/// | Method | Speed | Accuracy | Best For |
/// |--------|-------|----------|----------|
/// | `Midpoint` | ★★★★★ | ★★☆☆☆ | Screening, batch processing |
/// | `GaussianQuadrature` | ★★★☆☆ | ★★★★☆ | Distressed credits, stability |
/// | `AdaptiveSimpson` | ★★☆☆☆ | ★★★★★ | Long tenors, complex curves |
/// | `IsdaExact` | ★★★★☆ | ★★★★☆ | Standard market quotes |
/// | `IsdaStandardModel` | ★★★★★ | ★★★★★ | ISDA compliance, production |
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// Simple midpoint rule with fixed steps (non-ISDA).
    ///
    /// Fast but lower accuracy. Suitable for approximate valuations,
    /// high-volume batch processing, or when exact ISDA compliance is not required.
    Midpoint,
    /// Gaussian quadrature for higher accuracy.
    ///
    /// Provides better stability for distressed credits where hazard rates
    /// are high (>5%) and the integrand varies rapidly. Uses configurable
    /// Gauss-Legendre order (2, 4, 8, or 16 points).
    GaussianQuadrature,
    /// Adaptive Simpson's rule.
    ///
    /// Automatically adjusts integration density based on curve shape.
    /// Best for long tenors (>10Y) or complex hazard curves with steep
    /// term structure. Slower but handles curve irregularities well.
    AdaptiveSimpson,
    /// ISDA standard integration with exact points.
    ///
    /// Uses ISDA-specified integration points for regulatory compliance.
    /// Good balance of accuracy and speed for standard market instruments.
    IsdaExact,
    /// ISDA Standard Model (analytical integration over piecewise constant rates).
    ///
    /// The recommended method for production CDS pricing. Uses analytical
    /// formulas assuming piecewise-constant hazard rates between curve knots,
    /// matching ISDA Standard Model v1.8.2 exactly.
    IsdaStandardModel,
}

impl IntegrationMethod {
    /// Recommended integration method based on instrument characteristics.
    ///
    /// This helper provides guidance for selecting an appropriate integration
    /// method based on the CDS tenor and credit quality.
    ///
    /// # Selection Logic
    ///
    /// - **Short tenors (< 2Y)**: `Midpoint` - Fast, sufficient accuracy for
    ///   short-dated instruments where integration error is small.
    ///
    /// - **Standard tenors (2-10Y), investment grade**: `IsdaStandardModel` -
    ///   ISDA-compliant, analytical, and fast. The default for production.
    ///
    /// - **Long tenors (> 10Y)**: `AdaptiveSimpson` - Better handles the
    ///   complexity of long-dated protection legs with changing curve shapes.
    ///
    /// - **Distressed credits (any tenor)**: `GaussianQuadrature` - Provides
    ///   numerical stability when hazard rates are high and survival probability
    ///   decays rapidly.
    ///
    /// # Arguments
    ///
    /// * `tenor_years` - CDS tenor in years (e.g., 5.0 for a 5Y CDS)
    /// * `is_distressed` - Whether the credit is distressed (hazard rate > 5%,
    ///   or spread > 500bps typically)
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_valuations::instruments::credit_derivatives::cds::pricer::IntegrationMethod;
    ///
    /// // Standard 5Y investment grade CDS
    /// let method = IntegrationMethod::recommended(5.0, false);
    /// assert_eq!(method, IntegrationMethod::IsdaStandardModel);
    ///
    /// // Distressed 3Y CDS
    /// let method = IntegrationMethod::recommended(3.0, true);
    /// assert_eq!(method, IntegrationMethod::GaussianQuadrature);
    ///
    /// // Long-dated 15Y CDS
    /// let method = IntegrationMethod::recommended(15.0, false);
    /// assert_eq!(method, IntegrationMethod::AdaptiveSimpson);
    /// ```
    #[must_use]
    pub fn recommended(tenor_years: f64, is_distressed: bool) -> Self {
        if is_distressed {
            // Distressed credits need stable integration regardless of tenor
            Self::GaussianQuadrature
        } else if tenor_years < 2.0 {
            // Short tenors: speed matters, error is small
            Self::Midpoint
        } else if tenor_years > 10.0 {
            // Long tenors: curve shape matters more
            Self::AdaptiveSimpson
        } else {
            // Standard tenors: ISDA compliance and speed
            Self::IsdaStandardModel
        }
    }
}

/// Configuration for CDS pricing.
///
/// Controls numerical integration, day count conventions, and par spread calculation
/// methodology. Use factory methods like [`isda_standard()`](Self::isda_standard) for
/// pre-configured setups.
#[derive(Clone, Debug)]
pub struct CDSPricerConfig {
    /// Number of integration steps per year for protection leg (used with Midpoint method).
    /// For adaptive integration, use `min_steps_per_year` and `adaptive_steps` instead.
    pub steps_per_year: usize,
    /// Minimum integration steps per year (floor for adaptive step calculation).
    pub min_steps_per_year: usize,
    /// If true, adapt integration steps based on tenor: `max(min_steps_per_year, tenor * 12)`.
    /// Provides higher accuracy for longer tenors and distressed credits.
    pub adaptive_steps: bool,
    /// Include accrual on default in premium leg calculation
    pub include_accrual: bool,
    /// Tolerance for iterative calculations
    pub tolerance: f64,
    /// Integration method for protection leg calculation
    pub integration_method: IntegrationMethod,
    /// Use ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec)
    pub use_isda_coupon_dates: bool,
    /// Par spread denominator methodology:
    /// - `false` (default): Use Risky Annuity only (ISDA Standard Model)
    /// - `true`: Include accrual-on-default in denominator (Bloomberg CDSW style)
    ///
    /// The difference is typically < 1bp for investment grade but can reach 2-5 bps
    /// for distressed credits (hazard rate > 3%).
    pub par_spread_uses_full_premium: bool,
    /// Gauss–Legendre order for GaussianQuadrature method.
    /// Supported values: 2, 4, 8, 16. Invalid values default to 8.
    pub gl_order: usize,
    /// Maximum recursion depth for AdaptiveSimpson integration
    pub adaptive_max_depth: usize,
    /// Business days per year for settlement delay calculations (region-specific).
    /// Default: 252 (US), alternatives: 250 (UK), 255 (Japan)
    pub business_days_per_year: f64,
    /// Max iterations for bootstrapping solver
    pub bootstrap_max_iterations: usize,
    /// Tolerance for bootstrapping solver
    pub bootstrap_tolerance: f64,
}

/// Supported Gauss-Legendre orders for numerical integration.
const SUPPORTED_GL_ORDERS: [usize; 4] = [2, 4, 8, 16];

impl Default for CDSPricerConfig {
    fn default() -> Self {
        Self::isda_standard()
    }
}

impl CDSPricerConfig {
    /// Create an ISDA 2014 standard compliant configuration (North America/US market).
    ///
    /// Features:
    /// - ISDA Standard Model integration (analytical piecewise-constant)
    /// - Adaptive step sizing based on tenor
    /// - ISDA coupon dates (20th of Mar/Jun/Sep/Dec)
    /// - Accrual-on-default included
    /// - Risky annuity for par spread denominator
    #[must_use]
    pub fn isda_standard() -> Self {
        Self {
            steps_per_year: isda::STANDARD_INTEGRATION_POINTS,
            min_steps_per_year: isda::STANDARD_INTEGRATION_POINTS,
            adaptive_steps: true,
            include_accrual: true,
            tolerance: NUMERICAL_TOLERANCE,
            integration_method: IntegrationMethod::IsdaStandardModel,
            use_isda_coupon_dates: true,
            par_spread_uses_full_premium: false,
            gl_order: 8,
            adaptive_max_depth: 12,
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_US,
            bootstrap_max_iterations: 100,
            bootstrap_tolerance: numerical::SOLVER_TOLERANCE,
        }
    }

    /// Create an ISDA configuration for European markets (UK conventions).
    #[must_use]
    pub fn isda_europe() -> Self {
        Self {
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_UK,
            ..Self::isda_standard()
        }
    }

    /// Create an ISDA configuration for Asian markets (Japan conventions).
    #[must_use]
    pub fn isda_asia() -> Self {
        Self {
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_JP,
            ..Self::isda_standard()
        }
    }

    /// Create a simplified configuration for faster but less accurate pricing.
    ///
    /// Uses midpoint integration without adaptive steps. Suitable for
    /// approximate valuations or high-volume batch processing.
    #[must_use]
    pub fn simplified() -> Self {
        Self {
            steps_per_year: 365,
            min_steps_per_year: 52,
            adaptive_steps: false,
            include_accrual: true,
            tolerance: 1e-7,
            integration_method: IntegrationMethod::Midpoint,
            use_isda_coupon_dates: false,
            par_spread_uses_full_premium: false,
            gl_order: 4,
            adaptive_max_depth: 10,
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_US,
            bootstrap_max_iterations: 100,
            bootstrap_tolerance: numerical::SOLVER_TOLERANCE,
        }
    }

    /// Get validated Gauss-Legendre order (2, 4, 8, or 16).
    ///
    /// Returns the configured `gl_order` if supported, otherwise defaults to 8.
    #[must_use]
    pub fn validated_gl_order(&self) -> usize {
        if SUPPORTED_GL_ORDERS.contains(&self.gl_order) {
            self.gl_order
        } else {
            8 // Default to 8-point quadrature
        }
    }

    /// Calculate effective integration steps based on tenor.
    ///
    /// When `adaptive_steps` is enabled, returns `max(min_steps_per_year, tenor_years * 12)`.
    /// This ensures higher accuracy for longer tenors and distressed credits.
    #[must_use]
    pub fn effective_steps(&self, tenor_years: f64) -> usize {
        if self.adaptive_steps {
            let adaptive = (tenor_years * 12.0).ceil() as usize;
            self.min_steps_per_year.max(adaptive)
        } else {
            self.steps_per_year
        }
    }
}

/// CDS pricing engine. Stateless wrapper carrying configuration.
#[derive(Debug)]
pub struct CDSPricer {
    config: CDSPricerConfig,
}

impl Default for CDSPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSPricer {
    /// Create new pricer with default ISDA-compliant config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
        }
    }

    /// Create pricer with custom config.
    #[must_use]
    pub fn with_config(config: CDSPricerConfig) -> Self {
        Self { config }
    }

    /// Get the configuration for this pricer.
    #[must_use]
    pub fn config(&self) -> &CDSPricerConfig {
        &self.config
    }

    /// Calculate PV of protection leg with numerical integration.
    ///
    /// The protection leg represents the contingent payment made by the
    /// protection seller upon a credit event. Its present value is:
    ///
    /// ```text
    /// PV_prot = (1 - R) × ∫ DF(t + delay) × (-dS(t)) dt
    /// ```
    ///
    /// where R is the recovery rate, DF is the discount factor, S is the
    /// survival probability, and delay is the settlement delay in years.
    /// Calculate PV of protection leg (Money)
    pub fn pv_protection_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_protection_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of protection leg (raw f64)
    ///
    /// # Panics
    ///
    /// This method assumes the CDS has been validated at construction time.
    /// Recovery rate is expected to be in [0, 1]. Invalid recovery rates will
    /// produce incorrect results without error.
    pub fn pv_protection_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        // Note: Recovery rate validation is performed at CDS construction time.
        // All public constructors (buy_protection, sell_protection, new_isda) call validate().

        // Protection leg covers the period from premium start to premium end
        // But we only value protection from as_of onwards (can't protect against past defaults)
        let protection_start = as_of.max(cds.premium.start);
        let t_start = self.year_fraction(as_of, protection_start, cds.premium.dc)?;
        let t_end = self.year_fraction(as_of, cds.premium.end, cds.premium.dc)?;
        let recovery = cds.protection.recovery_rate;
        let delay_years =
            (cds.protection.settlement_delay as f64) / self.config.business_days_per_year;

        let protection_pv = match self.config.integration_method {
            IntegrationMethod::Midpoint => {
                self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
            }
            IntegrationMethod::GaussianQuadrature => match self.protection_leg_gaussian_quadrature(
                t_start,
                t_end,
                recovery,
                delay_years,
                disc,
                surv,
            ) {
                Ok(pv) => pv,
                Err(_) => {
                    self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
                }
            },
            IntegrationMethod::AdaptiveSimpson => match self.protection_leg_adaptive_simpson(
                t_start,
                t_end,
                recovery,
                delay_years,
                disc,
                surv,
            ) {
                Ok(pv) => pv,
                Err(_) => {
                    self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
                }
            },
            IntegrationMethod::IsdaExact => {
                self.protection_leg_isda_exact(t_start, t_end, recovery, delay_years, disc, surv)?
            }
            IntegrationMethod::IsdaStandardModel => self.protection_leg_isda_standard_model(
                t_start,
                t_end,
                recovery,
                delay_years,
                disc,
                surv,
            )?,
        };

        Ok(protection_pv * cds.notional.amount())
    }

    /// Calculate PV of premium leg with optional accrual-on-default
    /// Calculate PV of premium leg (Money)
    pub fn pv_premium_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_premium_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of premium leg (raw f64)
    pub fn pv_premium_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;

        let mut premium_pv = 0.0;
        let spread = cds.premium.spread_bp.to_f64().ok_or_else(|| {
            Error::Validation(format!(
                "spread_bp {} cannot be converted to f64",
                cds.premium.spread_bp
            ))
        })? * ONE_BASIS_POINT;

        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;

            let sp = surv.sp(t_end);
            let df = disc.df(t_end);

            premium_pv += spread * accrual * sp * df;

            if self.config.include_accrual {
                premium_pv += self.calculate_accrual_on_default(
                    spread,
                    self.year_fraction(base_date, start_date, cds.premium.dc)?,
                    t_end,
                    disc,
                    surv,
                )?;
            }
        }

        Ok(premium_pv * cds.notional.amount())
    }

    /// Calculate accrual-on-default for a period using configured method
    fn calculate_accrual_on_default(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let period_length = t_end - t_start;
        match self.config.integration_method {
            IntegrationMethod::Midpoint => {
                self.accrual_on_default_midpoint(spread, t_start, t_end, period_length, disc, surv)
            }
            IntegrationMethod::GaussianQuadrature | IntegrationMethod::AdaptiveSimpson => {
                self.accrual_on_default_adaptive(spread, t_start, t_end, period_length, disc, surv)
            }
            IntegrationMethod::IsdaExact => self.accrual_on_default_isda_exact(
                spread,
                t_start,
                t_end,
                period_length,
                disc,
                surv,
            ),
            IntegrationMethod::IsdaStandardModel => self.accrual_on_default_isda_standard_model(
                spread,
                t_start,
                t_end,
                period_length,
                disc,
                surv,
            ),
        }
    }

    fn accrual_on_default_midpoint(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let num_steps = (period_length * self.config.steps_per_year as f64).ceil() as usize;
        let dt = period_length / num_steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let default_prob = surv.sp(t1) - surv.sp(t2);
            let t_default = (t1 + t2) * 0.5;
            let accrued_time = t_default - t_start;
            let df = disc.df(t_default);
            let accrual = spread * accrued_time;
            accrual_pv += accrual * default_prob * df;
        }
        Ok(accrual_pv)
    }

    fn accrual_on_default_adaptive(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        _period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end || spread < 0.0 {
            return Err(Error::Internal);
        }
        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let accrued_time = (t - t_start).max(0.0);
            let df = disc.df(t);
            spread * accrued_time * density * df
        };
        adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    fn accrual_on_default_isda_exact(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let steps = isda::STANDARD_INTEGRATION_POINTS;
        let dt = period_length / steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..steps {
            let t = t_start + (i as f64 + 0.5) * dt;
            let accrual_fraction = (t - t_start) / period_length;
            let t1 = t - dt * 0.5;
            let t2 = t + dt * 0.5;
            let sp1 = if t1 >= 0.0 { surv.sp(t1) } else { 1.0 };
            let sp2 = surv.sp(t2);
            let default_prob = if sp1 > 0.0 && sp2 < sp1 {
                (sp1 - sp2) / dt
            } else {
                0.0
            };
            let df = disc.df(t);
            accrual_pv += spread * accrual_fraction * default_prob * df * dt;
        }
        Ok(accrual_pv)
    }

    fn accrual_on_default_isda_standard_model(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let steps = isda::STANDARD_INTEGRATION_POINTS;
        let dt = period_length / steps as f64;
        let mut accrual_pv = 0.0;

        for i in 0..steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = if t1 >= 0.0 { surv.sp(t1) } else { 1.0 };
            let sp2 = surv.sp(t2);

            if sp1 > 0.0 && sp2 < sp1 {
                // Calculate piecewise constant hazard rate
                let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Get discount factors at both ends
                let df1 = disc.df(t1);
                let df2 = disc.df(t2);

                // Calculate piecewise constant interest rate
                let interest_rate = if df1 > 0.0 && df2 > 0.0 && df2 < df1 {
                    -(df2 / df1).ln() / dt
                } else {
                    0.0
                };

                // ISDA Standard Model analytical integration for accrual on default:
                // We need ∫[t1,t2] (t - t_start) * D(t) * λ * S(t) dt
                // where D(t) = D(t1) * exp(-r*(t-t1)) and S(t) = S(t1) * exp(-λ*(t-t1))
                //
                // Let τ = t - t1, then the integral becomes:
                // D(t1) * S(t1) * ∫[0,dt] (t_start - t1 + τ) * exp(-(r+λ)*τ) * λ dτ
                // = D(t1) * S(t1) * λ * [(t1 - t_start) * I0 + I1]
                // where I0 = ∫exp(-(r+λ)*τ)dτ and I1 = ∫τ*exp(-(r+λ)*τ)dτ

                let lambda_plus_r = hazard_rate + interest_rate;

                if lambda_plus_r.abs() > numerical::ZERO_TOLERANCE {
                    let exp_term = (-lambda_plus_r * dt).exp();
                    // I0 = [1 - exp(-k*dt)] / k
                    let i0 = (1.0 - exp_term) / lambda_plus_r;
                    // I1 = [1 - exp(-k*dt)*(1 + k*dt)] / k^2
                    let i1 = (1.0 - exp_term * (1.0 + lambda_plus_r * dt))
                        / (lambda_plus_r * lambda_plus_r);

                    accrual_pv += spread * df1 * sp1 * hazard_rate * ((t1 - t_start) * i0 + i1);
                } else {
                    // Fallback: midpoint approximation for very small rates
                    let t_mid = (t1 + t2) * 0.5;
                    let accrued_time = t_mid - t_start;
                    accrual_pv += spread * accrued_time * (sp1 - sp2) * df1;
                }
            }
        }

        Ok(accrual_pv)
    }

    fn protection_leg_midpoint(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let tenor_years = t_end - t_start;
        let steps_per_year = self.config.effective_steps(tenor_years);
        let num_steps = ((tenor_years) * steps_per_year as f64).ceil() as usize;
        let num_steps = num_steps.max(1); // Ensure at least one step
        let dt = tenor_years / num_steps as f64;
        let mut protection_pv = 0.0;
        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let t_mid = (t1 + t2) * 0.5;
            let default_prob = surv.sp(t1) - surv.sp(t2);
            let df = disc.df(t_mid + delay_years);
            protection_pv += (1.0 - recovery) * default_prob * df;
        }
        Ok(protection_pv)
    }

    fn protection_leg_gaussian_quadrature(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        // Recovery validation done at entry point (pv_protection_leg)
        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let df = disc.df(t + delay_years);
            lgd * density * df
        };
        gauss_legendre_integrate(integrand, t_start, t_end, self.config.validated_gl_order())
    }

    fn protection_leg_adaptive_simpson(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        // Recovery validation done at entry point (pv_protection_leg)
        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let df = disc.df(t + delay_years);
            lgd * density * df
        };
        adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    fn protection_leg_isda_exact(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        // Recovery validation done at entry point (pv_protection_leg)
        let lgd = 1.0 - recovery;
        let tenor_years = t_end - t_start;
        let steps_per_period = self.config.effective_steps(tenor_years);
        let dt = tenor_years / steps_per_period as f64;
        let mut integral = 0.0;
        for i in 0..steps_per_period {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = surv.sp(t1);
            let sp2 = surv.sp(t2);
            if sp1 > sp2 && sp1 > 0.0 {
                let hazard_rate = -(sp2 / sp1).ln() / dt;
                let avg_t = (t1 + t2) * 0.5;
                let df_mid = disc.df(avg_t + delay_years);
                if hazard_rate.abs() > numerical::ZERO_TOLERANCE {
                    integral += (sp1 - sp2) * df_mid;
                } else {
                    let sp_mid = (sp1 + sp2) * 0.5;
                    integral += sp_mid * df_mid * hazard_rate * dt;
                }
            }
        }
        Ok(integral * lgd)
    }

    fn protection_leg_isda_standard_model(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        // Recovery validation done at entry point (pv_protection_leg)
        let lgd = 1.0 - recovery;
        let tenor_years = t_end - t_start;
        let steps_per_period = self.config.effective_steps(tenor_years);
        let dt = tenor_years / steps_per_period as f64;
        let mut integral = 0.0;

        for i in 0..steps_per_period {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = surv.sp(t1);
            let sp2 = surv.sp(t2);

            if sp1 > sp2 && sp1 > 0.0 {
                // Calculate piecewise constant hazard rate for this interval
                let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Get discount factors at both ends
                let df1 = disc.df(t1 + delay_years);
                let df2 = disc.df(t2 + delay_years);

                // Calculate piecewise constant interest rate
                let interest_rate = if df1 > 0.0 && df2 > 0.0 && df2 < df1 {
                    -(df2 / df1).ln() / dt
                } else {
                    0.0
                };

                // ISDA Standard Model analytical integration:
                // For piecewise constant hazard (λ) and interest (r) rates:
                // ∫[t1,t2] D(t) * (-dS(t)) dt = D(t1) * S(t1) * [λ/(λ+r)] * [1 - exp(-(λ+r)*Δt)]
                let lambda_plus_r = hazard_rate + interest_rate;

                if lambda_plus_r.abs() > numerical::ZERO_TOLERANCE {
                    let exp_term = (-lambda_plus_r * dt).exp();
                    integral += df1 * sp1 * (hazard_rate / lambda_plus_r) * (1.0 - exp_term);
                } else {
                    // Fallback to simple approximation when rates are very small
                    integral += df1 * sp1 * hazard_rate * dt;
                }
            }
        }

        Ok(integral * lgd)
    }

    /// Generate payment schedule for CDS with ISDA standard dates support.
    ///
    /// When `use_isda_coupon_dates` is enabled, generates IMM dates (20th of
    /// Mar/Jun/Sep/Dec) with business day adjustment per the CDS calendar.
    #[must_use = "schedule generation is pure computation"]
    pub fn generate_schedule(&self, cds: &CreditDefaultSwap, _as_of: Date) -> Result<Vec<Date>> {
        if self.config.use_isda_coupon_dates {
            self.generate_isda_schedule(cds)
        } else {
            let sched = crate::cashflow::builder::build_dates(
                cds.premium.start,
                cds.premium.end,
                cds.premium.freq,
                cds.premium.stub,
                cds.premium.bdc,
                cds.premium.calendar_id.as_deref(),
            )?;
            Ok(sched.dates)
        }
    }

    /// Generate ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec).
    ///
    /// Payment dates are adjusted using the CDS calendar and business day
    /// convention (Modified Following per ISDA 2014 standard). If no calendar
    /// is specified, dates are returned unadjusted.
    pub fn generate_isda_schedule(&self, cds: &CreditDefaultSwap) -> Result<Vec<Date>> {
        let mut schedule = vec![cds.premium.start];
        let mut current = cds.premium.start;

        // Resolve calendar for business day adjustment
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        while current < cds.premium.end {
            current = next_cds_date(current);
            if current <= cds.premium.end {
                // Apply business day adjustment if calendar is available
                let adjusted = if let Some(cal) = calendar {
                    adjust(current, cds.premium.bdc, cal).unwrap_or(current)
                } else {
                    current
                };
                schedule.push(adjusted);
            }
        }

        // Handle maturity date - ensure it's in the schedule
        let maturity_adjusted = if let Some(cal) = calendar {
            adjust(cds.premium.end, cds.premium.bdc, cal).unwrap_or(cds.premium.end)
        } else {
            cds.premium.end
        };

        if schedule.last() != Some(&maturity_adjusted) {
            schedule.push(maturity_adjusted);
        }

        Ok(schedule)
    }

    /// Calculate par spread (bps) that sets NPV to zero.
    ///
    /// # ISDA Standard Par Spread Definition
    ///
    /// By default (when `par_spread_uses_full_premium = false`), this implements
    /// the **ISDA standard definition**:
    ///
    /// ```text
    /// Par Spread = Protection_PV / Risky_Annuity
    /// ```
    ///
    /// where `Risky_Annuity` is the sum of `DF(t) * SP(t) * YearFrac` across
    /// coupon periods, **excluding** accrual-on-default from the denominator.
    ///
    /// # Par Spread with Full Premium Leg
    ///
    /// When `par_spread_uses_full_premium = true`, the denominator includes the
    /// full premium leg PV (with accrual-on-default) per basis point. This matches
    /// Bloomberg CDSW-style calculations and is appropriate for distressed credits
    /// where accrual-on-default has material impact (typically 2-5 bps for hazard > 3%).
    ///
    /// # Returns
    ///
    /// Par spread in basis points (bps).
    #[must_use = "par spread calculation is pure computation"]
    pub fn par_spread(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;

        // Default behavior (par_spread_uses_full_premium = false) uses Risky Annuity only.
        // This excludes accrual-on-default from the denominator per ISDA convention.
        let denom = if self.config.par_spread_uses_full_premium {
            // Opt-in: Compute full premium PV per 1bp including AoD
            let base_date = disc.base_date();
            let schedule = self.generate_schedule(cds, as_of)?;
            let mut ann = 0.0;
            for i in 0..schedule.len() - 1 {
                let start_date = schedule[i];
                let end_date = schedule[i + 1];
                let t_start = self.year_fraction(base_date, start_date, cds.premium.dc)?;
                let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
                let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
                let sp = surv.sp(t_end);
                let df = disc.df(t_end);
                let unit_spread = 1.0;
                // coupon part per unit spread
                ann += unit_spread * accrual * sp * df;
                // AoD part per unit spread in this period
                ann +=
                    self.calculate_accrual_on_default(unit_spread, t_start, t_end, disc, surv)?;
            }
            ann
        } else {
            // ISDA Standard: Risky Annuity (sum of DF * SP * YearFrac)
            self.risky_annuity(cds, disc, surv, as_of)?
        };

        if denom.abs() < 1e-12 {
            return Err(Error::Validation(
                "Par spread denominator is too small (risky annuity ≈ 0). \
                 This may indicate zero survival probability or expired CDS."
                    .to_string(),
            ));
        }

        // Result in Basis Points
        Ok(protection_pv.amount() / (denom * cds.notional.amount()) * 10000.0)
    }

    /// Premium leg PV per 1 bp of spread, including accrual-on-default if configured.
    #[must_use = "premium leg calculation is pure computation"]
    pub fn premium_leg_pv_per_bp(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut per_bp_pv = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];
            let t_start = self.year_fraction(base_date, start_date, cds.premium.dc)?;
            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
            let sp = surv.sp(t_end);
            let df = disc.df(t_end);
            per_bp_pv += ONE_BASIS_POINT * accrual * sp * df;
            if self.config.include_accrual {
                per_bp_pv +=
                    self.calculate_accrual_on_default(ONE_BASIS_POINT, t_start, t_end, disc, surv)?;
            }
        }
        Ok(per_bp_pv)
    }

    /// Risky annuity: survival-weighted duration of premium payments.
    ///
    /// This is the sum of `DF(t) × SP(t) × YearFrac` across all coupon periods.
    /// Used as the denominator in ISDA standard par spread calculation.
    #[must_use = "risky annuity calculation is pure computation"]
    pub fn risky_annuity(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut annuity = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];
            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
            let sp = surv.sp(t_end);
            let df = disc.df(t_end);
            annuity += accrual * sp * df;
        }
        Ok(annuity)
    }

    /// Risky PV01: change in NPV for a 1bp increase in the CDS spread.
    ///
    /// Computed as `Risky Annuity × Notional / 10000`.
    #[must_use = "risky PV01 calculation is pure computation"]
    pub fn risky_pv01(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let risky_annuity = self.risky_annuity(cds, disc, surv, as_of)?;
        Ok(risky_annuity * cds.notional.amount() / 10000.0)
    }

    /// Instrument NPV from the perspective of the `PayReceive` side.
    ///
    /// - **Protection buyer** (PayFixed): NPV = Protection PV − Premium PV
    /// - **Protection seller** (ReceiveFixed): NPV = Premium PV − Protection PV
    pub fn npv(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;
        let premium_pv = self.pv_premium_leg(cds, disc, surv, as_of)?;
        match cds.side {
            PayReceive::PayFixed => protection_pv.checked_sub(premium_pv),
            PayReceive::ReceiveFixed => premium_pv.checked_sub(protection_pv),
        }
    }

    /// Instrument NPV including upfront override when provided.
    pub fn npv_with_upfront(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let mut pv = self.npv(cds, disc, surv, as_of)?;
        if let Some(upfront) = cds.pricing_overrides.upfront_payment {
            pv = (pv + upfront)?;
        }
        Ok(pv)
    }

    /// Resolve curves from MarketContext and compute NPV with upfront.
    pub fn npv_market(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
        let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
        self.npv_with_upfront(cds, disc.as_ref(), surv.as_ref(), as_of)
    }

    /// Year fraction helper using the provided day count convention.
    fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<f64> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }
}

// ----- Local helpers -----
#[inline]
fn approx_default_density(surv: &dyn Survival, t: f64, h: f64, t_start: f64, t_end: f64) -> f64 {
    // Finite-difference approximation of -dS/dt, clipped to [t_start, t_end]
    let hh = if h <= 0.0 {
        (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR
    } else {
        h
    };
    let (s_left, s_right, denom) = if t <= t_start + hh {
        (surv.sp(t), surv.sp((t + hh).min(t_end)), hh)
    } else if t >= t_end - hh {
        (surv.sp((t - hh).max(t_start)), surv.sp(t), hh)
    } else {
        (surv.sp(t - hh), surv.sp(t + hh), 2.0 * hh)
    };
    let deriv = (s_right - s_left) / denom; // ≈ dS/dt
    (-deriv).max(0.0)
}

/// Bootstrap hazard rates from CDS spreads to a simple hazard curve
pub struct CDSBootstrapper {
    config: CDSPricerConfig,
}

impl Default for CDSBootstrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSBootstrapper {
    /// Create new bootstrapper with default config
    pub fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
        }
    }

    /// Bootstrap hazard curve from CDS spreads (tenor years, spread bps)
    pub fn bootstrap_hazard_curve(
        &self,
        cds_spreads: &[(f64, f64)],
        recovery_rate: f64,
        disc: &dyn Discounting,
        base_date: Date,
    ) -> Result<HazardCurve> {
        let mut knots = Vec::new();
        let mut par_spreads = Vec::new();
        let pricer = CDSPricer::with_config(self.config.clone());

        // Sort spreads by tenor to ensure correct bootstrapping order
        let mut sorted_spreads = cds_spreads.to_vec();
        sorted_spreads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        for &(tenor, spread_bps) in &sorted_spreads {
            let cds = self.create_synthetic_cds(base_date, tenor, spread_bps, recovery_rate)?;
            let hazard_rate = self
                .solve_for_hazard_rate(&cds, disc, spread_bps, &pricer, &knots, tenor, base_date)?;
            knots.push((tenor, hazard_rate));
            par_spreads.push((tenor, spread_bps));
        }

        HazardCurve::builder("BOOTSTRAPPED")
            .base_date(base_date)
            .knots(knots)
            .recovery_rate(recovery_rate)
            .par_spreads(par_spreads)
            .build()
    }

    fn create_synthetic_cds(
        &self,
        base_date: Date,
        tenor_years: f64,
        spread_bps: f64,
        recovery_rate: f64,
    ) -> Result<CreditDefaultSwap> {
        let months = (tenor_years * 12.0).round() as i32;
        let end_date = base_date.add_months(months);
        let spread_bp_decimal = Decimal::try_from(spread_bps).map_err(|e| {
            Error::Validation(format!(
                "spread_bps {} cannot be represented as Decimal: {}",
                spread_bps, e
            ))
        })?;
        CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(format!("SYNTHETIC_{:.1}Y", tenor_years)),
            Money::new(1_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            crate::instruments::cds::CDSConvention::IsdaNa,
            spread_bp_decimal,
            base_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("DISC"),
            finstack_core::types::CurveId::new("CREDIT"),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_for_hazard_rate(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        target_spread_bps: f64,
        pricer: &CDSPricer,
        existing_knots: &[(f64, f64)],
        current_tenor: f64,
        base_date: Date,
    ) -> Result<f64> {
        // Objective function: ParSpread(h) - TargetSpread = 0
        let objective = |h: f64| -> f64 {
            // Create temp hazard curve with existing knots + trial point
            let surv = match self.create_temp_hazard_curve(
                existing_knots,
                current_tenor,
                h,
                base_date,
                cds.protection.recovery_rate,
            ) {
                Ok(curve) => curve,
                Err(_) => return f64::NAN, // Signal invalid region to solver
            };
            match pricer.par_spread(cds, disc, &surv, disc.base_date()) {
                Ok(spread) => spread - target_spread_bps,
                Err(_) => f64::NAN, // Signal invalid region to solver
            }
        };

        // Initial guess using credit triangle approximation: h ~ S / (1-R)
        // Or use the last bootstrapped hazard rate if available
        let initial_guess = if let Some(&(_, last_h)) = existing_knots.last() {
            last_h
        } else {
            target_spread_bps / 10000.0 / (1.0 - cds.protection.recovery_rate)
        };

        let bracket_min = 1e-5; // 0.1 bp hazard
        let bracket_max = 1.0; // 100% hazard

        let solver = BrentSolver {
            tolerance: self.config.bootstrap_tolerance,
            max_iterations: self.config.bootstrap_max_iterations,
            ..Default::default()
        };

        solver.solve(objective, initial_guess.clamp(bracket_min, bracket_max))
    }

    fn create_temp_hazard_curve(
        &self,
        existing_knots: &[(f64, f64)],
        current_tenor: f64,
        current_rate: f64,
        base_date: Date,
        recovery_rate: f64,
    ) -> Result<HazardCurve> {
        let mut knots = existing_knots.to_vec();
        knots.push((current_tenor, current_rate));

        HazardCurve::builder("TEMP")
            .base_date(base_date)
            .recovery_rate(recovery_rate)
            .knots(knots)
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::DateExt;
    use finstack_core::market_data::term_structures::DiscountCurve;

    fn create_test_cds(
        id: impl Into<String>,
        start_date: Date,
        end_date: Date,
        spread_bp: f64,
        recovery_rate: f64,
    ) -> CreditDefaultSwap {
        CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(id),
            Money::new(10_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            crate::instruments::cds::CDSConvention::IsdaNa,
            Decimal::try_from(spread_bp).expect("valid spread_bp"),
            start_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("USD-OIS"),
            finstack_core::types::CurveId::new("TEST-CREDIT"),
        )
        .expect("test CDS creation should succeed")
    }

    fn create_test_curves() -> (DiscountCurve, HazardCurve) {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date"))
            .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.65)])
            .build()
            .expect("should succeed");

        let credit = HazardCurve::builder("TEST-CREDIT")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date"))
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.02), (3.0, 0.03), (5.0, 0.04), (10.0, 0.05)])
            .par_spreads(vec![
                (1.0, 100.0),
                (3.0, 150.0),
                (5.0, 200.0),
                (10.0, 250.0),
            ])
            .build()
            .expect("should succeed");

        (disc, credit)
    }

    #[test]
    fn test_enhanced_protection_leg() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 100.0, 0.40);
        let pricer = CDSPricer::new();
        let protection_pv = pricer
            .pv_protection_leg(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        assert!(protection_pv.amount() > 0.0);
    }

    #[test]
    fn test_accrual_on_default() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 100.0, 0.40);
        let pricer_with = CDSPricer::new();
        let pricer_without = CDSPricer::with_config(CDSPricerConfig {
            include_accrual: false,
            integration_method: IntegrationMethod::Midpoint,
            ..Default::default()
        });
        let pv_with = pricer_with
            .pv_premium_leg(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        let pv_without = pricer_without
            .pv_premium_leg(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        assert!(pv_with.amount() > pv_without.amount());
    }

    #[test]
    fn test_par_spread_calculation() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 0.0, 0.40);
        let pricer = CDSPricer::new();
        let par_spread = pricer
            .par_spread(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        assert!(par_spread > 0.0 && par_spread < 2000.0);
        let mut cds_at_par = cds.clone();
        cds_at_par.premium.spread_bp = Decimal::try_from(par_spread).expect("valid par_spread");
        let npv = pricer
            .npv(&cds_at_par, &disc, &credit, as_of)
            .expect("should succeed");
        assert!(npv.amount().abs() < 15000.0);
    }

    #[test]
    fn test_settlement_delay_reduces_protection_pv() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let mut cds0 = create_test_cds("CDS-0D", as_of, as_of.add_months(60), 100.0, 0.40);
        let mut cds20 = cds0.clone();
        cds0.protection.settlement_delay = 0;
        cds20.protection.settlement_delay = 20;
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::GaussianQuadrature,
            ..Default::default()
        });
        let pv0 = pricer
            .pv_protection_leg(&cds0, &disc, &credit, as_of)
            .expect("should succeed")
            .amount();
        let pv20 = pricer
            .pv_protection_leg(&cds20, &disc, &credit, as_of)
            .expect("should succeed")
            .amount();
        assert!(pv20 < pv0);
    }

    #[test]
    fn test_par_spread_full_premium_option_runs() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("CDS-PAR", as_of, as_of.add_months(60), 0.0, 0.40);
        let pricer_ra = CDSPricer::new();
        let pricer_full = CDSPricer::with_config(CDSPricerConfig {
            par_spread_uses_full_premium: true,
            ..Default::default()
        });
        let s1 = pricer_ra
            .par_spread(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        let s2 = pricer_full
            .par_spread(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        assert!(s1.is_finite() && s2.is_finite());
    }
}
