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
//! # Day Count Convention Handling
//!
//! The CDS pricer uses **multiple day count conventions** for different purposes,
//! following market standard practice:
//!
//! | Calculation | Day Count Source | Rationale |
//! |-------------|------------------|-----------|
//! | **Accrual fraction** | Instrument premium leg (`premium.dc`) | ACT/360 for NA, ACT/365F for Asia |
//! | **Survival time axis** | Hazard curve (`surv.day_count()`) | Consistent with curve construction |
//! | **Discount time axis** | Discount curve (`disc.day_count()`) | Consistent with yield curve |
//!
//! ## Accrual-on-Default (AoD) Day Count
//!
//! The accrual-on-default calculation uses the **instrument's premium leg day count**
//! for the accrual fraction (the portion of coupon accrued before default), while
//! the default timing within the period uses the **hazard curve's day count** for
//! survival probability interpolation.
//!
//! For most NA CDS (ACT/360 premium on ACT/360 hazard curves), this is identical.
//! For Asian CDS (ACT/365F premium on ACT/360 hazard curves), there can be a small
//! (~1%) difference in AoD contribution. This is the expected behavior as:
//! - The premium accrual represents the contractual payment calculation
//! - The survival probability represents the market's view of default timing
//!
//! ## References
//!
//! - ISDA CDS Standard Model (Markit, 2009)
//! - O'Kane, D. "Modelling Single-name and Multi-name Credit Derivatives" (2008), Chapter 5
//! - Hull, J.C. & White, A. "Valuing Credit Default Swaps I: No Counterparty Default Risk"

#![allow(dead_code)] // WIP: public API not yet wired into main pricing paths
use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::solver::SolverConfig;
use crate::calibration::targets::hazard::HazardCurveTarget;
use crate::calibration::{CalibrationConfig, CalibrationMethod};
use crate::constants::{
    credit, isda, numerical, time as time_constants, BASIS_POINTS_PER_UNIT, ONE_BASIS_POINT,
};
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::credit_derivatives::cds::{CdsDocClause, CreditDefaultSwap, PayReceive};
use crate::market::conventions::ids::CdsConventionKey;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::{Pillar, QuoteId};
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::currency::Currency;
use finstack_core::dates::DateExt;
use finstack_core::dates::{adjust, next_cds_date, Date, DayCount, HolidayCalendar, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve, Seniority};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};
use rust_decimal::Decimal;
use std::cell::RefCell;
use time::Duration;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// aligned with ISDA Standard Model v1.8.2 for the provided curve inputs.
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
    /// ```text
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
#[derive(Debug, Clone)]
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
    /// If true, apply the current restructuring-clause approximation to the protection leg.
    ///
    /// Default is `false` because the approximation is not clause-consistent enough for
    /// production pricing. When enabled, protection PV ordering follows
    /// `Xr14 <= Mr14 <= Mm14 <= Cr14` heuristically.
    pub enable_restructuring_approximation: bool,
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
            tolerance: numerical::ZERO_TOLERANCE,
            integration_method: IntegrationMethod::IsdaStandardModel,
            use_isda_coupon_dates: true,
            par_spread_uses_full_premium: false,
            enable_restructuring_approximation: false,
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
            enable_restructuring_approximation: false,
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

    /// Validate configuration parameters.
    ///
    /// Returns an error if any parameter is out of valid range. This method provides
    /// fail-fast validation for catching configuration errors early.
    ///
    /// # Errors
    ///
    /// Returns a validation error if:
    /// - `tolerance` is not positive
    /// - `steps_per_year` is zero
    /// - `min_steps_per_year` is zero
    /// - `bootstrap_max_iterations` is zero
    /// - `bootstrap_tolerance` is not positive
    /// - `business_days_per_year` is not positive
    /// - `adaptive_max_depth` is zero
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::credit_derivatives::cds::CDSPricerConfig;
    ///
    /// let config = CDSPricerConfig::isda_standard();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        if self.tolerance <= 0.0 {
            return Err(Error::Validation(
                "CDSPricerConfig: tolerance must be positive".into(),
            ));
        }
        if self.steps_per_year == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: steps_per_year must be at least 1".into(),
            ));
        }
        if self.min_steps_per_year == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: min_steps_per_year must be at least 1".into(),
            ));
        }
        if self.bootstrap_max_iterations == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: bootstrap_max_iterations must be at least 1".into(),
            ));
        }
        if self.bootstrap_tolerance <= 0.0 {
            return Err(Error::Validation(
                "CDSPricerConfig: bootstrap_tolerance must be positive".into(),
            ));
        }
        if self.business_days_per_year <= 0.0 {
            return Err(Error::Validation(
                "CDSPricerConfig: business_days_per_year must be positive".into(),
            ));
        }
        if self.adaptive_max_depth == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: adaptive_max_depth must be at least 1".into(),
            ));
        }
        Ok(())
    }
}

/// Maximum deliverable obligation maturity cap (in months) for a given
/// documentation clause.
///
/// This controls how restructuring credit events affect the protection leg:
///
/// - **`Cr14`** (Full Restructuring): No maturity cap on deliverable obligations.
///   All bonds of the reference entity are deliverable, making restructuring a
///   broad credit event. Returns `None` (uncapped).
///
/// - **`Mr14`** (Modified Restructuring): Deliverable obligations are capped at
///   30 months from the restructuring event. This limits the cheapest-to-deliver
///   option and reduces the value of restructuring protection.
///
/// - **`Mm14`** (Modified-Modified Restructuring): 60-month cap on deliverable
///   obligation maturity. A compromise between CR and MR, common in European CDS.
///
/// - **`Xr14`** (No Restructuring): Restructuring is not a credit event.
///   Returns `Some(0)` indicating no restructuring benefit.
///
/// - **Meta-clauses** (`IsdaNa`, `IsdaEu`, `IsdaAs`, `IsdaAu`, `IsdaNz`):
///   Delegate to the effective concrete clause per regional convention.
///
/// - **`Custom`**: Treated as no restructuring (`Some(0)`) by default.
///
/// # Returns
///
/// - `None`: No maturity cap (full restructuring benefit).
/// - `Some(0)`: No restructuring benefit (Xr14 or Custom).
/// - `Some(n)`: Maturity cap of `n` months from the restructuring event.
#[must_use]
pub fn max_deliverable_maturity(clause: CdsDocClause) -> Option<u32> {
    match clause {
        CdsDocClause::Cr14 => None,      // Full restructuring, uncapped
        CdsDocClause::Mr14 => Some(30),  // Modified Restructuring: 30 months
        CdsDocClause::Mm14 => Some(60),  // Modified-Modified Restructuring: 60 months
        CdsDocClause::Xr14 => Some(0),   // No Restructuring: no benefit
        CdsDocClause::Custom => Some(0), // Conservative default: no benefit
        // Meta-clauses delegate to their effective concrete clause
        CdsDocClause::IsdaNa => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaEu => max_deliverable_maturity(CdsDocClause::Mm14),
        CdsDocClause::IsdaAs => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaAu => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaNz => max_deliverable_maturity(CdsDocClause::Xr14),
    }
}

/// CDS pricing engine. Stateless wrapper carrying configuration.
#[derive(Debug)]
pub struct CDSPricer {
    config: CDSPricerConfig,
}

#[derive(Clone, Copy)]
struct AodInputs<'a> {
    cds: &'a CreditDefaultSwap,
    spread: f64,
    start_date: Date,
    end_date: Date,
    settlement_delay: u16,
    calendar: Option<&'a dyn HolidayCalendar>,
    as_of: Date,
    disc: &'a DiscountCurve,
    surv: &'a HazardCurve,
}

#[derive(Clone, Copy)]
struct CouponPeriod {
    accrual_start: Date,
    accrual_end: Date,
    payment_date: Date,
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
    ///
    /// Note: This method does not validate the configuration. For fail-fast
    /// validation, use [`try_with_config`](Self::try_with_config) instead.
    #[must_use]
    pub fn with_config(config: CDSPricerConfig) -> Self {
        Self { config }
    }

    /// Create pricer with custom config, validating parameters.
    ///
    /// Returns an error if the configuration contains invalid parameters.
    /// Prefer this over [`with_config`](Self::with_config) when configuration
    /// comes from external sources (user input, config files, etc.).
    ///
    /// # Errors
    ///
    /// Returns a validation error if the configuration is invalid.
    /// See [`CDSPricerConfig::validate`] for details.
    pub fn try_with_config(config: CDSPricerConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_protection_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of protection leg (raw f64)
    ///
    /// Uses proper time-axis conventions:
    /// - Times are computed using the hazard curve's day-count convention
    /// - Survival probabilities are conditional on no default before `as_of`
    /// - Discounting uses the discount curve (times mapped from hazard curve axis)
    ///
    /// # Panics
    ///
    /// This method assumes the CDS has been validated at construction time.
    /// Recovery rate is expected to be in [0, 1]. Invalid recovery rates will
    /// produce incorrect results without error.
    pub fn pv_protection_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        // Note: Recovery rate validation is performed at CDS construction time.
        // All public constructors (builder, new_isda) call validate().

        // Protection leg covers the period from protection start to premium end.
        // For forward-starting CDS, protection begins at protection_effective_date
        // (which may be later than the premium leg start).
        // We only value protection from as_of onwards (can't protect against past defaults).
        let protection_start = as_of.max(cds.protection_start());
        let protection_end = cds.premium.end;

        // Forward-start at or past maturity: no protection interval by construction.
        if cds.protection_start() >= protection_end {
            return Ok(0.0);
        }

        // Determine the effective restructuring adjustment policy.
        // By default the pricer disables restructuring uplift because the current
        // implementation is only a heuristic approximation. It can be re-enabled
        // explicitly via `CDSPricerConfig::enable_restructuring_approximation`.
        let effective_clause = cds.doc_clause_effective();
        let restructuring_factor = if self.config.enable_restructuring_approximation {
            restructuring_adjustment_factor(effective_clause, cds)
        } else {
            1.0
        };

        // Use hazard curve's day-count for time axis (survival is the dominant factor)
        let t_asof = haz_t(surv, as_of)?;
        let t_start = haz_t(surv, protection_start)?;
        let t_end = haz_t(surv, protection_end)?;

        let recovery = cds.protection.recovery_rate;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        // Compute survival at as_of for conditioning
        let sp_asof = surv.sp(t_asof);

        let protection_pv = match self.config.integration_method {
            IntegrationMethod::Midpoint => self.protection_leg_midpoint_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
            IntegrationMethod::GaussianQuadrature => {
                match self.protection_leg_gaussian_quadrature_cond(
                    t_start,
                    t_end,
                    recovery,
                    cds.protection.settlement_delay,
                    calendar,
                    sp_asof,
                    as_of,
                    disc,
                    surv,
                ) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "GaussianQuadrature",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(
                            t_start,
                            t_end,
                            recovery,
                            cds.protection.settlement_delay,
                            calendar,
                            sp_asof,
                            as_of,
                            disc,
                            surv,
                        )?
                    }
                }
            }
            IntegrationMethod::AdaptiveSimpson => {
                match self.protection_leg_adaptive_simpson_cond(
                    t_start,
                    t_end,
                    recovery,
                    cds.protection.settlement_delay,
                    calendar,
                    sp_asof,
                    as_of,
                    disc,
                    surv,
                ) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "AdaptiveSimpson",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(
                            t_start,
                            t_end,
                            recovery,
                            cds.protection.settlement_delay,
                            calendar,
                            sp_asof,
                            as_of,
                            disc,
                            surv,
                        )?
                    }
                }
            }
            IntegrationMethod::IsdaExact => self.protection_leg_isda_exact_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
            IntegrationMethod::IsdaStandardModel => self.protection_leg_isda_standard_model_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
        };

        // Apply the restructuring adjustment. Contracts with restructuring as a
        // credit event (Cr14, Mr14, Mm14) have protection worth more than Xr14
        // because they cover an additional class of credit events.
        Ok(protection_pv * restructuring_factor * cds.notional.amount())
    }

    /// Calculate PV of premium leg with optional accrual-on-default
    /// Calculate PV of premium leg (Money)
    pub fn pv_premium_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_premium_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of premium leg (raw f64)
    ///
    /// Uses proper time-axis conventions:
    /// - Discounting: relative DF from `as_of` using discount curve's day-count
    /// - Survival: conditional survival given no default before `as_of` using hazard curve's day-count
    /// - Accrual: instrument's premium leg day-count convention (Act/360 for NA, etc.)
    pub fn pv_premium_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);
        let schedule = crate::cashflow::traits::CashflowProvider::build_full_schedule(
            cds,
            &MarketContext::new(),
            as_of,
        )?;

        let mut premium_pv = 0.0;
        let mut start_date = cds.premium.start;

        for flow in schedule.flows.iter().filter(|cf| {
            cf.kind == finstack_core::cashflow::CFKind::Fixed
                || cf.kind == finstack_core::cashflow::CFKind::Stub
        }) {
            let end_date = flow.date;
            let payment_date = flow.date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                start_date = end_date;
                continue;
            }

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            premium_pv += flow.amount.amount() * sp * df;

            if self.config.include_accrual {
                // The scheduled coupon cashflows already include notional in `flow.amount`.
                // Keep AoD on the same dollar basis when adding it into premium PV.
                premium_pv += cds.notional.amount()
                    * self.accrual_on_default_isda_midpoint(AodInputs {
                        cds,
                        spread: flow.rate.unwrap_or(0.0),
                        start_date: start_date.max(as_of),
                        end_date,
                        settlement_delay: cds.protection.settlement_delay,
                        calendar,
                        as_of,
                        disc,
                        surv,
                    })?;
            }

            start_date = end_date;
        }

        Ok(premium_pv)
    }

    /// Calculate accrual-on-default for a period using dates with proper time-axis handling.
    ///
    /// This method properly handles:
    /// - Discounting using discount curve's day-count relative to as_of
    /// - Survival using hazard curve's day-count with conditional probability from as_of
    /// - Accrual fraction within the period
    fn accrual_on_default_isda_midpoint(&self, inp: AodInputs<'_>) -> Result<f64> {
        // ISDA midpoint approximation for accrual-on-default, using **dates**:
        //
        // AoD ≈ spread * (0.5 * τ_remaining) * DF(as_of→pay) * P(default in (start, end] | survived to as_of)
        //
        // Important: `start_date` is already `max(period_start, as_of)` in all call sites,
        // so this implements a "clean" AoD (does not include already-accrued premium before `as_of`).
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }

        // Remaining accrual fraction uses the instrument premium day count convention.
        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;

        // Conditional default probability between start and end (conditioned on survival to as_of).
        let sp_start = sp_cond_to(inp.surv, inp.as_of, inp.start_date)?;
        let sp_end = sp_cond_to(inp.surv, inp.as_of, inp.end_date)?;
        let default_prob = (sp_start - sp_end).max(0.0);

        let default_date = midpoint_default_date(inp.surv, inp.start_date, inp.end_date)?;
        let settle_date = settlement_date(
            default_date,
            inp.settlement_delay,
            inp.calendar,
            self.config.business_days_per_year,
        )?;
        let df = df_asof_to(inp.disc, inp.as_of, settle_date)?;

        Ok(inp.spread * 0.5 * tau_remaining * default_prob * df)
    }

    /// Midpoint method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_midpoint_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        let num_steps = (period_length * self.config.steps_per_year as f64).ceil() as usize;
        let num_steps = num_steps.max(1);
        let dt_haz = period_length / num_steps as f64;
        // Assume disc and haz time axes are similar for step sizing (not exact but reasonable)
        let dt_disc = (t_start_disc - t_start_haz + period_length) / num_steps as f64;
        let _ = dt_disc; // We'll interpolate disc time from haz time ratio

        let mut accrual_pv = 0.0;
        for i in 0..num_steps {
            let t1_haz = t_start_haz + i as f64 * dt_haz;
            let t2_haz = t_start_haz + (i + 1) as f64 * dt_haz;

            // Conditional default probability for this sub-period
            let sp1 = surv.sp(t1_haz) / sp_asof;
            let sp2 = surv.sp(t2_haz) / sp_asof;
            let default_prob = sp1 - sp2;

            // Default assumed at midpoint
            let t_mid_haz = (t1_haz + t2_haz) * 0.5;
            // Map haz time to disc time (linear interpolation approximation)
            let ratio = (t_mid_haz - t_start_haz) / period_length;
            let t_mid_disc = t_start_disc + ratio * (t_start_disc - t_start_haz + period_length);

            // Relative DF from as_of
            let df = disc.df(t_mid_disc) / df_asof;

            // Accrued time within the period (from start to default)
            let accrued_fraction = t_mid_haz - t_start_haz;
            let accrual = spread * accrued_fraction;

            accrual_pv += accrual * default_prob * df;
        }
        Ok(accrual_pv)
    }

    /// Adaptive method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_adaptive_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        if period_length <= 0.0 || spread < 0.0 {
            return Ok(0.0);
        }

        let h = period_length * numerical::INTEGRATION_STEP_FACTOR;
        let integrand = |t_haz: f64| {
            // Density of default at t_haz (conditioned on survival to as_of)
            let density = approx_default_density(surv, t_haz, h, t_start_haz, t_end_haz) / sp_asof;

            // Map haz time to disc time
            let ratio = (t_haz - t_start_haz) / period_length;
            let t_disc = t_start_disc + ratio * period_length;

            // Relative DF from as_of
            let df = disc.df(t_disc) / df_asof;

            // Accrued time within period
            let accrued_time = (t_haz - t_start_haz).max(0.0);

            spread * accrued_time * density * df
        };

        adaptive_simpson(
            integrand,
            t_start_haz,
            t_end_haz,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    /// ISDA-style method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_isda_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        if period_length <= 0.0 {
            return Ok(0.0);
        }

        // Use ISDA piecewise-constant approximation
        let steps_per_period = self.config.effective_steps(period_length);
        let dt = period_length / steps_per_period as f64;
        let mut accrual_pv = 0.0;

        for i in 0..steps_per_period {
            let t1_haz = t_start_haz + i as f64 * dt;
            let t2_haz = t1_haz + dt;

            // Conditional survival probabilities
            let sp1 = surv.sp(t1_haz) / sp_asof;
            let sp2 = surv.sp(t2_haz) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                // Note: Hazard rate computed for documentation but not needed in simplified formula
                // let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Map to disc time axis
                let ratio1 = (t1_haz - t_start_haz) / period_length;
                let t1_disc = t_start_disc + ratio1 * period_length;

                // Relative DF at interval start
                let df1 = disc.df(t1_disc) / df_asof;

                // Accrued time from period start to interval start
                let accrued_time = t1_haz - t_start_haz;

                // ISDA: accrual at default is approximately at interval midpoint
                // Simplified: use accrued_time + dt/2 as average
                let avg_accrued = accrued_time + dt * 0.5;

                // Contribution: spread * avg_accrued * (probability of default in interval) * df
                // Default prob = sp1 - sp2
                accrual_pv += spread * avg_accrued * (sp1 - sp2) * df1;
            }
        }

        Ok(accrual_pv)
    }

    /// Calculate accrual-on-default for a period using configured method (legacy time-based)
    ///
    /// Note: This method assumes times are computed using consistent day-count conventions.
    /// Prefer using `calculate_accrual_on_default_dates` for new code.
    fn calculate_accrual_on_default(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end || spread < 0.0 {
            return Err(Error::internal(
                "accrued-on-default integral requires t_start < t_end and non-negative spread",
            ));
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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

                // Calculate piecewise constant interest rate (allow negative rates)
                // Negative rates are valid when df2 > df1 (discount factors rising)
                let interest_rate = if df1 > 0.0 && df2 > 0.0 {
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
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

                // Calculate piecewise constant interest rate (allow negative rates)
                // Negative rates are valid when df2 > df1 (discount factors rising)
                let interest_rate = if df1 > 0.0 && df2 > 0.0 {
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

    // ----- Conditioned protection leg methods (proper time-axis handling) -----

    /// Midpoint method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    fn protection_leg_midpoint_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tenor_years = t_end - t_start;
        let steps_per_year = self.config.effective_steps(tenor_years);
        let num_steps = ((tenor_years) * steps_per_year as f64).ceil() as usize;
        let num_steps = num_steps.max(1);
        let dt = tenor_years / num_steps as f64;
        let lgd = 1.0 - recovery;
        let mut protection_pv = 0.0;

        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let t_mid = (t1 + t2) * 0.5;

            // Conditional survival probabilities
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;
            let default_prob = sp1 - sp2;

            // Discount on actual dates (supports discount/hazard curves with different day-counts).
            let default_date = date_from_hazard_time(surv, t_mid);
            let settle_date = self::settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            )?;
            let df = df_asof_to(disc, as_of, settle_date)?;

            protection_pv += lgd * default_prob * df;
        }
        Ok(protection_pv)
    }

    /// Gaussian quadrature method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    fn protection_leg_gaussian_quadrature_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let df_error: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if df_error.borrow().is_some() {
                return 0.0;
            }
            // Conditional default density
            let density = approx_default_density(surv, t, h, t_start, t_end) / sp_asof;
            // Discount on actual dates
            let default_date = date_from_hazard_time(surv, t);
            let settle_date = match self::settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            ) {
                Ok(date) => date,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(disc, as_of, settle_date) {
                Ok(df) => df,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    0.0
                }
            };
            lgd * density * df
        };
        let result =
            gauss_legendre_integrate(integrand, t_start, t_end, self.config.validated_gl_order())?;
        if let Some(err) = df_error.into_inner() {
            return Err(err);
        }
        Ok(result)
    }

    /// Adaptive Simpson method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    fn protection_leg_adaptive_simpson_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let df_error: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if df_error.borrow().is_some() {
                return 0.0;
            }
            // Conditional default density
            let density = approx_default_density(surv, t, h, t_start, t_end) / sp_asof;
            // Discount on actual dates
            let default_date = date_from_hazard_time(surv, t);
            let settle_date = match self::settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            ) {
                Ok(date) => date,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(disc, as_of, settle_date) {
                Ok(df) => df,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    0.0
                }
            };
            lgd * density * df
        };
        let result = adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )?;
        if let Some(err) = df_error.into_inner() {
            return Err(err);
        }
        Ok(result)
    }

    /// ISDA exact method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    fn protection_leg_isda_exact_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let lgd = 1.0 - recovery;
        let tenor_years = t_end - t_start;
        let steps_per_period = self.config.effective_steps(tenor_years);
        let dt = tenor_years / steps_per_period as f64;
        let mut integral = 0.0;

        for i in 0..steps_per_period {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;

            // Conditional survival probabilities
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                let hazard_rate = -(sp2 / sp1).ln() / dt;
                let avg_t = (t1 + t2) * 0.5;
                let default_date = date_from_hazard_time(surv, avg_t);
                let settle_date = self::settlement_date(
                    default_date,
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let df_mid = df_asof_to(disc, as_of, settle_date)?;

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

    /// ISDA Standard Model with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    fn protection_leg_isda_standard_model_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let lgd = 1.0 - recovery;
        let boundaries = isda_standard_model_boundaries(t_start, t_end, surv, disc);
        let mut integral = 0.0;

        for window in boundaries.windows(2) {
            let t1 = window[0];
            let t2 = window[1];
            let dt = t2 - t1;
            if dt <= numerical::ZERO_TOLERANCE {
                continue;
            }

            // Conditional survival probabilities
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                // Piecewise constant hazard rate for this interval
                let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Relative discount factors from as_of
                let d1 = self::settlement_date(
                    date_from_hazard_time(surv, t1),
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let d2 = self::settlement_date(
                    date_from_hazard_time(surv, t2),
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let df1 = df_asof_to(disc, as_of, d1)?;
                let df2 = df_asof_to(disc, as_of, d2)?;

                // Piecewise constant interest rate (allow negative rates)
                let interest_rate = if df1 > 0.0 && df2 > 0.0 {
                    -(df2 / df1).ln() / dt
                } else {
                    0.0
                };

                // ISDA Standard Model analytical integration
                let lambda_plus_r = hazard_rate + interest_rate;

                if lambda_plus_r.abs() > numerical::ZERO_TOLERANCE {
                    let exp_term = (-lambda_plus_r * dt).exp();
                    integral += df1 * sp1 * (hazard_rate / lambda_plus_r) * (1.0 - exp_term);
                } else {
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
                cds.premium.frequency,
                cds.premium.stub,
                cds.premium.bdc,
                false,
                0,
                cds.premium
                    .calendar_id
                    .as_deref()
                    .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
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

    fn coupon_periods(&self, cds: &CreditDefaultSwap, as_of: Date) -> Result<Vec<CouponPeriod>> {
        if self.config.use_isda_coupon_dates {
            self.generate_isda_coupon_periods(cds, as_of)
        } else {
            let schedule = self.generate_schedule(cds, as_of)?;
            Ok(schedule
                .windows(2)
                .map(|w| CouponPeriod {
                    accrual_start: w[0],
                    accrual_end: w[1],
                    payment_date: w[1],
                })
                .collect())
        }
    }

    fn generate_isda_coupon_periods(
        &self,
        cds: &CreditDefaultSwap,
        _as_of: Date,
    ) -> Result<Vec<CouponPeriod>> {
        let mut accrual_dates = vec![cds.premium.start];
        let mut current = cds.premium.start;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        while current < cds.premium.end {
            current = next_cds_date(current);
            if current <= cds.premium.end {
                accrual_dates.push(current);
            }
        }
        if accrual_dates.last() != Some(&cds.premium.end) {
            accrual_dates.push(cds.premium.end);
        }

        let mut periods = Vec::with_capacity(accrual_dates.len().saturating_sub(1));
        for window in accrual_dates.windows(2) {
            let payment_date = if let Some(cal) = calendar {
                adjust(window[1], cds.premium.bdc, cal).unwrap_or(window[1])
            } else {
                window[1]
            };
            periods.push(CouponPeriod {
                accrual_start: window[0],
                accrual_end: window[1],
                payment_date,
            });
        }
        Ok(periods)
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
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;

        // Default behavior (par_spread_uses_full_premium = false) uses Risky Annuity only.
        // This excludes accrual-on-default from the denominator per ISDA convention.
        let denom = if self.config.par_spread_uses_full_premium {
            // Opt-in: Compute full premium PV per 1bp including AoD
            let periods = self.coupon_periods(cds, as_of)?;
            let calendar = cds
                .premium
                .calendar_id
                .as_deref()
                .and_then(finstack_core::dates::calendar::calendar_by_id);
            let mut ann = 0.0;
            for period in periods {
                let start_date = period.accrual_start;
                let end_date = period.accrual_end;
                let payment_date = period.payment_date;

                // Skip periods that have already ended before as_of
                if end_date <= as_of {
                    continue;
                }

                // Accrual uses instrument day-count
                let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

                // Discounting uses discount curve's day-count and relative DF from as_of
                let df = df_asof_to(disc, as_of, payment_date)?;

                // Survival uses hazard curve's day-count and conditional probability
                let sp = sp_cond_to(surv, as_of, end_date)?;

                let unit_spread = 1.0;
                // coupon part per unit spread
                ann += unit_spread * accrual * sp * df;

                // AoD part per unit spread in this period
                ann += self.accrual_on_default_isda_midpoint(AodInputs {
                    cds,
                    spread: unit_spread,
                    start_date: start_date.max(as_of),
                    end_date,
                    settlement_delay: cds.protection.settlement_delay,
                    calendar,
                    as_of,
                    disc,
                    surv,
                })?;
            }
            ann
        } else {
            // ISDA Standard: Risky Annuity (sum of DF * SP * YearFrac)
            self.risky_annuity(cds, disc, surv, as_of)?
        };

        if denom.abs() < numerical::RATE_COMPARISON_TOLERANCE {
            return Err(Error::Validation(
                "Par spread denominator is too small (risky annuity ≈ 0). \
                 This may indicate zero survival probability or expired CDS."
                    .to_string(),
            ));
        }

        // Result in Basis Points
        Ok(protection_pv.amount() / (denom * cds.notional.amount()) * BASIS_POINTS_PER_UNIT)
    }

    /// Premium leg PV per 1 bp of spread, including accrual-on-default if configured.
    ///
    /// Uses proper time-axis conventions for discounting and survival.
    #[must_use = "premium leg calculation is pure computation"]
    pub fn premium_leg_pv_per_bp(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let periods = self.coupon_periods(cds, as_of)?;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);
        let mut per_bp_pv = 0.0;
        for period in periods {
            let start_date = period.accrual_start;
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            per_bp_pv += ONE_BASIS_POINT * accrual * sp * df;

            if self.config.include_accrual {
                per_bp_pv += self.accrual_on_default_isda_midpoint(AodInputs {
                    cds,
                    spread: ONE_BASIS_POINT,
                    start_date: start_date.max(as_of),
                    end_date,
                    settlement_delay: cds.protection.settlement_delay,
                    calendar,
                    as_of,
                    disc,
                    surv,
                })?;
            }
        }
        Ok(per_bp_pv)
    }

    /// Risky annuity: survival-weighted duration of premium payments.
    ///
    /// This is the sum of `DF(t) × SP(t) × YearFrac` across all coupon periods.
    /// Used as the denominator in ISDA standard par spread calculation.
    ///
    /// Uses proper time-axis conventions for discounting and survival.
    #[must_use = "risky annuity calculation is pure computation"]
    pub fn risky_annuity(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let periods = self.coupon_periods(cds, as_of)?;
        let mut annuity = 0.0;
        for period in periods {
            let start_date = period.accrual_start;
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

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
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let risky_annuity = self.risky_annuity(cds, disc, surv, as_of)?;
        Ok(risky_annuity * cds.notional.amount() / BASIS_POINTS_PER_UNIT)
    }

    /// Instrument NPV from the perspective of the `PayReceive` side.
    ///
    /// - **Protection buyer** (PayFixed): NPV = Protection PV − Premium PV
    /// - **Protection seller** (ReceiveFixed): NPV = Premium PV − Protection PV
    pub fn npv(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;
        let premium_pv = self.pv_premium_leg(cds, disc, surv, as_of)?;
        match cds.side {
            PayReceive::PayFixed => protection_pv.checked_sub(premium_pv),
            PayReceive::ReceiveFixed => premium_pv.checked_sub(protection_pv),
        }
    }

    /// Instrument NPV including both types of upfront payments.
    ///
    /// This method applies two types of upfront payments (if present):
    ///
    /// 1. **Dated cashflow** (`cds.upfront: Option<(Date, Money)>`):
    ///    A specific payment on a specific date, discounted from `as_of`.
    ///    - Positive amount = payment by buyer, negative = receipt by buyer
    ///    - Applied with sign convention based on trade side
    ///
    /// 2. **PV adjustment** (`cds.pricing_overrides.market_quotes.upfront_payment: Option<Money>`):
    ///    An already-discounted adjustment to the PV at `as_of`.
    ///    - Added directly without further discounting
    ///    - Positive = increases NPV, negative = decreases NPV (for both sides)
    ///
    /// Both can be set simultaneously without double-counting.
    pub fn npv_with_upfront(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let mut pv = self.npv(cds, disc, surv, as_of)?;

        // 1. Handle dated cashflow upfront (discounted and signed)
        if let Some((dt, amount)) = cds.upfront {
            if dt >= as_of {
                let df = df_asof_to(disc, as_of, dt)?;
                let upfront_pv = Money::new(amount.amount() * df, cds.notional.currency());
                // Sign convention: positive upfront is paid by buyer
                pv = match cds.side {
                    PayReceive::PayFixed => pv.checked_sub(upfront_pv)?,
                    PayReceive::ReceiveFixed => pv.checked_add(upfront_pv)?,
                };
            }
        }

        // 2. Handle PV adjustment upfront (added directly without discounting)
        if let Some(upfront) = cds.pricing_overrides.market_quotes.upfront_payment {
            pv = pv.checked_add(upfront)?;
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
}

// ----- Time-axis helpers -----
//
// These helpers ensure we use the correct day-count conventions:
// - For discounting: use the discount curve's day-count convention
// - For survival: use the hazard curve's day-count convention
// - For accrual: use the instrument's premium leg day-count convention

/// Compute time from discount curve's base date using its day-count convention.
#[inline]
fn disc_t(disc: &DiscountCurve, date: Date) -> Result<f64> {
    disc.day_count().year_fraction(
        disc.base_date(),
        date,
        finstack_core::dates::DayCountCtx::default(),
    )
}

/// Compute time from hazard curve's base date using its day-count convention.
#[inline]
fn haz_t(surv: &HazardCurve, date: Date) -> Result<f64> {
    surv.day_count().year_fraction(
        surv.base_date(),
        date,
        finstack_core::dates::DayCountCtx::default(),
    )
}

/// Approximate inverse mapping from hazard-curve time (years) to a calendar date.
///
/// This is exact for ACT/365F and ACT/360 hazard curve day-counts (since the forward
/// mapping uses actual day counts), and a reasonable approximation for other
/// conventions. The resulting date is used only for discounting on actual dates.
#[inline]
fn date_from_hazard_time(surv: &HazardCurve, t: f64) -> Date {
    let t = t.max(0.0);
    let days_per_year = match surv.day_count() {
        DayCount::Act360 => 360.0,
        DayCount::Act365F => 365.0,
        DayCount::Act365L | DayCount::ActAct | DayCount::ActActIsma => 365.25,
        DayCount::Thirty360 | DayCount::ThirtyE360 => 360.0,
        DayCount::Bus252 => 252.0,
        // Fallback for less common conventions; used only for discount-date mapping.
        _ => 365.25,
    };
    let days = (t * days_per_year).round() as i64;
    surv.base_date() + Duration::days(days)
}

/// Resolve settlement date for a default occurring on `default_date`.
#[inline]
fn settlement_date(
    default_date: Date,
    settlement_delay: u16,
    calendar: Option<&dyn HolidayCalendar>,
    business_days_per_year: f64,
) -> Result<Date> {
    if settlement_delay == 0 {
        return Ok(default_date);
    }

    if let Some(cal) = calendar {
        return default_date.add_business_days(settlement_delay as i32, cal);
    }

    // Fallback: approximate business days into calendar days.
    let delay_days = ((settlement_delay as f64) * credit::CALENDAR_DAYS_PER_YEAR
        / business_days_per_year)
        .round() as i64;
    Ok(default_date + Duration::days(delay_days))
}

#[inline]
fn midpoint_default_date(surv: &HazardCurve, start_date: Date, end_date: Date) -> Result<Date> {
    let t_start = haz_t(surv, start_date)?;
    let t_end = haz_t(surv, end_date)?;
    Ok(date_from_hazard_time(surv, 0.5 * (t_start + t_end)))
}

fn isda_standard_model_boundaries(
    t_start: f64,
    t_end: f64,
    surv: &HazardCurve,
    disc: &DiscountCurve,
) -> Vec<f64> {
    let mut boundaries = Vec::with_capacity(surv.len() + disc.knots().len() + 2);
    boundaries.push(t_start);
    boundaries.push(t_end);
    boundaries.extend(
        surv.knot_points()
            .map(|(t, _)| t)
            .filter(|&t| t > t_start && t < t_end),
    );
    boundaries.extend(
        disc.knots()
            .iter()
            .copied()
            .filter(|&t| t > t_start && t < t_end),
    );
    boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    boundaries.dedup_by(|a, b| (*a - *b).abs() <= numerical::ZERO_TOLERANCE);
    boundaries
}

/// Compute discount factor from as_of to date using curve's time axis.
/// This returns df(date) / df(as_of) = exp(-r*(t_date - t_asof))
#[inline]
fn df_asof_to(disc: &DiscountCurve, as_of: Date, date: Date) -> Result<f64> {
    disc.df_between_dates(as_of, date)
}

/// Compute conditional survival probability: S(date | survived to as_of).
/// Returns S(t_date) / S(t_asof) where times are computed using hazard curve's day-count.
///
/// Uses `credit::SURVIVAL_PROBABILITY_FLOOR` to prevent division by near-zero
/// values that could produce inf/NaN results.
#[inline]
fn sp_cond_to(surv: &HazardCurve, as_of: Date, date: Date) -> Result<f64> {
    let t_asof = haz_t(surv, as_of)?;
    let t_date = haz_t(surv, date)?;
    let sp_asof = surv.sp(t_asof);
    let sp_date = surv.sp(t_date);
    // Conditional survival: S(date) / S(as_of)
    // Use floor constant to prevent division by near-zero producing inf/NaN
    if sp_asof > credit::SURVIVAL_PROBABILITY_FLOOR {
        Ok(sp_date / sp_asof)
    } else {
        Ok(0.0) // Already defaulted (or effectively defaulted) by as_of
    }
}

// ----- Local helpers -----
#[inline]
fn approx_default_density(surv: &HazardCurve, t: f64, h: f64, t_start: f64, t_end: f64) -> f64 {
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

/// Compute a multiplicative adjustment factor for the protection leg PV
/// based on the effective documentation clause.
///
/// Restructuring credit events increase the probability of a payout (more
/// event types can trigger protection). The factor represents how much
/// additional protection value the restructuring clause provides relative
/// to the base default-only protection.
///
/// The factor is calibrated to approximate market practice:
///
/// | Clause | Factor | Rationale |
/// |--------|--------|-----------|
/// | `Xr14` | 1.00 | Baseline: default events only |
/// | `Mr14` | 1.02 | Small uplift: limited deliverables (30 months) |
/// | `Mm14` | 1.03 | Moderate uplift: longer deliverable window (60 months) |
/// | `Cr14` | 1.05 | Full uplift: unrestricted deliverables |
/// | `Custom`| 1.00 | Conservative: no restructuring benefit assumed |
///
/// These factors are first-order approximations. In production, a full
/// restructuring model would separate the restructuring hazard rate from
/// the default hazard rate.
fn restructuring_adjustment_factor(clause: CdsDocClause, cds: &CreditDefaultSwap) -> f64 {
    let cap = max_deliverable_maturity(clause);
    match cap {
        Some(0) => {
            // No restructuring benefit (Xr14 or Custom)
            1.0
        }
        Some(months) => {
            // Limited restructuring: scale based on how much of the CDS tenor
            // the restructuring cap covers. If the cap exceeds the remaining
            // tenor, the full restructuring benefit applies.
            let tenor_months = {
                let start = cds.premium.start;
                let end = cds.premium.end;
                // Approximate tenor in months
                let days = (end - start).whole_days();
                days as f64 / 30.44 // average days per month
            };
            // Coverage ratio: what fraction of the CDS tenor is covered by the cap
            let coverage = (months as f64 / tenor_months).min(1.0);
            // Base restructuring premium scaled by coverage
            // MR14 (30 months) has ~2% base uplift, MM14 (60 months) has ~3%
            let base_uplift = if months <= 30 { 0.02 } else { 0.03 };
            1.0 + base_uplift * coverage
        }
        None => {
            // Full restructuring (Cr14): uncapped deliverable maturity
            1.05
        }
    }
}

/// Configuration for CDS bootstrapping.
///
/// Controls how synthetic CDS instruments are constructed during hazard curve
/// bootstrapping to match market quote conventions.
#[derive(Debug, Clone)]
pub struct BootstrapConvention {
    /// CDS convention (determines day count, frequency, etc.)
    pub convention: crate::instruments::credit_derivatives::cds::CDSConvention,
    /// Whether to use IMM dates for maturity (20th of Mar/Jun/Sep/Dec)
    pub use_imm_dates: bool,
}

impl Default for BootstrapConvention {
    fn default() -> Self {
        Self {
            convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa,
            use_imm_dates: true, // Standard market practice
        }
    }
}

impl BootstrapConvention {
    fn representative_convention_key(&self) -> CdsConventionKey {
        match self.convention {
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa => {
                CdsConventionKey {
                    currency: Currency::USD,
                    doc_clause: CdsDocClause::IsdaNa,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaEu => {
                CdsConventionKey {
                    currency: Currency::EUR,
                    doc_clause: CdsDocClause::IsdaEu,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaAs => {
                CdsConventionKey {
                    currency: Currency::JPY,
                    doc_clause: CdsDocClause::IsdaAs,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::Custom => {
                CdsConventionKey {
                    currency: Currency::USD,
                    doc_clause: CdsDocClause::Custom,
                }
            }
        }
    }
}

/// Bootstrap hazard rates from CDS spreads to a simple hazard curve
pub struct CDSBootstrapper {
    config: CDSPricerConfig,
    convention: BootstrapConvention,
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
            convention: BootstrapConvention::default(),
        }
    }

    /// Create bootstrapper with custom convention
    pub fn with_convention(convention: BootstrapConvention) -> Self {
        Self {
            config: CDSPricerConfig::default(),
            convention,
        }
    }

    /// Create bootstrapper with custom pricer config and convention
    pub fn with_config(config: CDSPricerConfig, convention: BootstrapConvention) -> Self {
        Self { config, convention }
    }

    /// Bootstrap hazard curve from CDS spreads (tenor years, spread bps)
    ///
    /// This method constructs synthetic CDS instruments for each input tenor/spread
    /// pair and solves for the hazard rate that reproduces the quoted spread.
    ///
    /// # Arguments
    ///
    /// * `cds_spreads` - Slice of (tenor_years, spread_bps) pairs
    /// * `recovery_rate` - Assumed recovery rate for the reference entity
    /// * `disc` - Discount curve for present value calculations
    /// * `base_date` - Valuation date and curve base date
    ///
    /// # IMM Date Handling
    ///
    /// When `use_imm_dates` is true (default), maturities are aligned to the
    /// standard CDS IMM dates (20th of Mar/Jun/Sep/Dec). For example:
    /// - A 5Y CDS quoted on 2024-01-15 would have maturity 2029-03-20
    /// - Premium start is the most recent IMM date (2023-12-20)
    pub fn bootstrap_hazard_curve(
        &self,
        cds_spreads: &[(f64, f64)],
        recovery_rate: f64,
        disc: &DiscountCurve,
        base_date: Date,
    ) -> Result<HazardCurve> {
        let mut sorted_spreads = cds_spreads.to_vec();
        sorted_spreads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if sorted_spreads.is_empty() {
            return Err(Error::Input(finstack_core::InputError::TooFewPoints));
        }

        let convention_key = self.convention.representative_convention_key();
        let entity = "BOOTSTRAPPED".to_string();
        let quotes: Vec<MarketQuote> = sorted_spreads
            .iter()
            .map(|(tenor_years, spread_bp)| {
                MarketQuote::Cds(CdsQuote::CdsParSpread {
                    id: QuoteId::new(format!("BOOTSTRAPPED-{tenor_years:.6}")),
                    entity: entity.clone(),
                    convention: convention_key.clone(),
                    pillar: Pillar::Tenor(Tenor::from_years(
                        *tenor_years,
                        self.convention.convention.day_count(),
                    )),
                    spread_bp: *spread_bp,
                    recovery_rate,
                })
            })
            .collect();

        let mut config = CalibrationConfig {
            calibration_method: CalibrationMethod::Bootstrap,
            solver: SolverConfig::brent_default()
                .with_tolerance(self.config.bootstrap_tolerance)
                .with_max_iterations(self.config.bootstrap_max_iterations),
            ..Default::default()
        };
        if sorted_spreads
            .iter()
            .any(|(_, spread_bp)| *spread_bp >= 1_000.0)
        {
            config.hazard_curve.hazard_hard_max = config.hazard_curve.hazard_hard_max.max(100.0);
            config.hazard_curve.validation_tolerance =
                config.hazard_curve.validation_tolerance.max(1e-6);
            config.validation.max_hazard_rate = config.validation.max_hazard_rate.max(2.0);
        }

        let params = HazardCurveParams {
            curve_id: CurveId::from("BOOTSTRAPPED"),
            entity,
            seniority: Seniority::Senior,
            currency: convention_key.currency,
            base_date,
            discount_curve_id: disc.id().clone(),
            recovery_rate,
            notional: 1_000_000.0,
            method: CalibrationMethod::Bootstrap,
            interpolation: Default::default(),
            par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
            doc_clause: Some(format!("{:?}", convention_key.doc_clause)),
        };

        let base_context = MarketContext::new().insert(disc.clone());
        let (context, _) = HazardCurveTarget::solve(&params, &quotes, &base_context, &config)?;
        Ok(context
            .get_hazard(params.curve_id.as_str())?
            .as_ref()
            .clone())
    }

    fn create_synthetic_cds(
        &self,
        base_date: Date,
        tenor_years: f64,
        spread_bps: f64,
        recovery_rate: f64,
    ) -> Result<CreditDefaultSwap> {
        let spread_bp_decimal = Decimal::try_from(spread_bps).map_err(|e| {
            Error::Validation(format!(
                "spread_bps {} cannot be represented as Decimal: {}",
                spread_bps, e
            ))
        })?;

        // Determine premium start and end dates
        let (start_date, end_date) = if self.convention.use_imm_dates {
            // IMM-aligned dates: maturities on 20th of Mar/Jun/Sep/Dec
            // Premium start is the most recent IMM date on or before base_date
            let prev_imm = self.previous_imm_date(base_date);
            let months = (tenor_years * 12.0).round() as i32;
            // End date is the IMM date approximately `months` months after base_date
            let approx_end = base_date.add_months(months);
            let end_imm = next_cds_date(approx_end);
            (prev_imm, end_imm)
        } else {
            // Non-IMM: simple date arithmetic
            let months = (tenor_years * 12.0).round() as i32;
            let end_date = base_date.add_months(months);
            (base_date, end_date)
        };

        CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(format!("SYNTHETIC_{:.1}Y", tenor_years)),
            Money::new(1_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            self.convention.convention,
            spread_bp_decimal,
            start_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("DISC"),
            finstack_core::types::CurveId::new("CREDIT"),
        )
    }

    /// Find the most recent IMM date on or before the given date.
    ///
    /// IMM dates are the 20th of Mar, Jun, Sep, Dec.
    fn previous_imm_date(&self, date: Date) -> Date {
        use time::Month;

        let year = date.year();
        let month = date.month();
        let day = date.day();

        // IMM months are Mar(3), Jun(6), Sep(9), Dec(12)
        let month_num: u8 = month.into();

        // Find the current or previous IMM month.
        // For dates within an IMM month but before the 20th, we must fall back
        // to the previous IMM month (e.g., Dec 5 → Sep 20, not Dec 20).
        let (imm_year, imm_month) = if month_num == 12 && day >= 20 {
            // Dec 20 or later -> Dec 20 of this year
            (year, Month::December)
        } else if month_num > 9 || (month_num == 9 && day >= 20) {
            // Sep 20 or later (through Dec 19) -> Sep 20 of this year
            (year, Month::September)
        } else if month_num > 6 || (month_num == 6 && day >= 20) {
            // Jun 20 or later (through Sep 19) -> Jun 20 of this year
            (year, Month::June)
        } else if month_num > 3 || (month_num == 3 && day >= 20) {
            // Mar 20 or later (through Jun 19) -> Mar 20 of this year
            (year, Month::March)
        } else {
            // Before Mar 20 -> Dec 20 of previous year
            (year - 1, Month::December)
        };

        // Return the IMM date (20th of the month)
        Date::from_calendar_date(imm_year, imm_month, 20).unwrap_or(date)
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_for_hazard_rate(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
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
            match pricer.par_spread(cds, disc, &surv, base_date) {
                Ok(spread) => spread - target_spread_bps,
                Err(_) => f64::NAN, // Signal invalid region to solver
            }
        };

        // Initial guess using credit triangle approximation: h ~ S / (1-R)
        // Or use the last bootstrapped hazard rate if available
        let lgd = (1.0 - cds.protection.recovery_rate).max(numerical::DIVISION_EPSILON);
        let implied_hazard = target_spread_bps / BASIS_POINTS_PER_UNIT / lgd;

        let initial_guess = if let Some(&(_, last_h)) = existing_knots.last() {
            last_h
        } else {
            implied_hazard
        };

        // Adaptive bracket: for distressed credits (high spreads), expand upper bound
        let bracket_min = credit::MIN_HAZARD_RATE;
        let bracket_max = (implied_hazard * credit::HAZARD_RATE_BRACKET_MULTIPLIER)
            .max(credit::DEFAULT_MAX_HAZARD_RATE);

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
    use crate::calibration::api::engine;
    use crate::calibration::api::schema::{
        CalibrationEnvelope, CalibrationPlan, CalibrationStep, HazardCurveParams, StepParams,
    };
    use crate::calibration::CalibrationMethod;
    use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    use crate::market::quotes::cds::CdsQuote;
    use crate::market::quotes::ids::{Pillar, QuoteId};
    use crate::market::quotes::market_quote::MarketQuote;
    use finstack_core::dates::{DateExt, Tenor};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::{DiscountCurve, Seniority};
    use finstack_core::types::CurveId;
    use finstack_core::HashMap;
    use time::macros::date;

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
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa,
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
    fn premium_leg_scales_linearly_with_notional_when_accrual_on_default_enabled() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::February, 15).expect("valid date");
        let pricer = CDSPricer::new();

        let mut cds_unit = create_test_cds(
            "TEST-CDS-UNIT",
            date!(2024 - 12 - 20),
            date!(2028 - 03 - 20),
            100.0,
            0.40,
        );
        cds_unit.notional = Money::new(1.0, Currency::USD);

        let mut cds_large = cds_unit.clone();
        cds_large.id = finstack_core::types::InstrumentId::new("TEST-CDS-LARGE");
        cds_large.notional = Money::new(1_000_000.0, Currency::USD);

        let pv_unit = pricer
            .pv_premium_leg_raw(&cds_unit, &disc, &credit, as_of)
            .expect("unit notional premium leg");
        let pv_large = pricer
            .pv_premium_leg_raw(&cds_large, &disc, &credit, as_of)
            .expect("large notional premium leg");

        let scaled_unit = pv_unit * cds_large.notional.amount();
        assert!(
            (pv_large - scaled_unit).abs() < 1e-8,
            "premium leg PV should scale with notional, unit={pv_unit}, large={pv_large}, scaled_unit={scaled_unit}"
        );
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
        // A CDS at par spread should have near-zero NPV. Tolerance of $5000
        // (~5bp on $10M) accounts for the accrual-on-default midpoint approximation
        // and discrete quarterly premium schedule vs. continuous protection leg.
        assert!(
            npv.amount().abs() < 5000.0,
            "CDS at par spread should have near-zero NPV, got {}",
            npv.amount()
        );
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
    fn test_isda_standard_model_ignores_step_tuning() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("CDS-STEP", as_of, as_of.add_months(120), 100.0, 0.40);

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.995),
                (0.5, 0.985),
                (1.0, 0.955),
                (3.0, 0.860),
                (7.0, 0.705),
                (10.0, 0.575),
            ])
            .build()
            .expect("should succeed");

        let credit = HazardCurve::builder("TEST-CREDIT")
            .base_date(as_of)
            .recovery_rate(0.40)
            .knots(vec![
                (0.25, 0.01),
                (0.5, 0.08),
                (1.0, 0.12),
                (3.0, 0.18),
                (7.0, 0.22),
                (10.0, 0.25),
            ])
            .build()
            .expect("should succeed");

        let coarse = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::IsdaStandardModel,
            steps_per_year: 1,
            min_steps_per_year: 1,
            adaptive_steps: false,
            ..Default::default()
        });
        let fine = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::IsdaStandardModel,
            steps_per_year: 3650,
            min_steps_per_year: 3650,
            adaptive_steps: false,
            ..Default::default()
        });

        let pv_coarse = coarse
            .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
            .expect("coarse pricer should succeed");
        let pv_fine = fine
            .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
            .expect("fine pricer should succeed");

        assert!(
            (pv_coarse - pv_fine).abs() < 1e-8,
            "ISDA standard model protection PV should not depend on step tuning; coarse={pv_coarse}, fine={pv_fine}",
        );
    }

    #[test]
    fn test_legacy_bootstrapper_matches_canonical_hazard_target_conventions() {
        let base = Date::from_calendar_date(2025, time::Month::March, 20).expect("valid date");
        let currency = Currency::USD;
        let recovery_rate = 0.40;
        let cds_spreads = vec![(1.0, 100.0), (3.0, 150.0)];

        let disc = DiscountCurve::builder("TEST-DISC")
            .base_date(base)
            .knots(vec![
                (0.0, 1.0),
                (1.0, 0.98),
                (3.0, 0.94),
                (5.0, 0.88),
                (10.0, 0.75),
            ])
            .build()
            .expect("discount curve");

        let legacy_curve = CDSBootstrapper::new()
            .bootstrap_hazard_curve(&cds_spreads, recovery_rate, &disc, base)
            .expect("legacy bootstrap should succeed");

        let initial_market = MarketContext::new().insert(disc);
        let quotes = cds_spreads
            .iter()
            .map(|(tenor_years, spread_bp)| {
                MarketQuote::Cds(CdsQuote::CdsParSpread {
                    id: QuoteId::new(format!("CDS-{tenor_years:.1}Y")),
                    entity: "BOOTSTRAP-CONSISTENCY".to_string(),
                    pillar: Pillar::Tenor(Tenor::from_years(
                        *tenor_years,
                        legacy_curve.day_count(),
                    )),
                    spread_bp: *spread_bp,
                    recovery_rate,
                    convention: CdsConventionKey {
                        currency,
                        doc_clause: CdsDocClause::IsdaNa,
                    },
                })
            })
            .collect();

        let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
        quote_sets.insert("credit".to_string(), quotes);

        let hazard_id: CurveId = "BOOTSTRAP-CONSISTENCY-SENIOR".into();
        let plan = CalibrationPlan {
            id: "plan".to_string(),
            description: None,
            quote_sets,
            settings: Default::default(),
            steps: vec![CalibrationStep {
                id: "haz".to_string(),
                quote_set: "credit".to_string(),
                params: StepParams::Hazard(HazardCurveParams {
                    curve_id: hazard_id.clone(),
                    entity: "BOOTSTRAP-CONSISTENCY".to_string(),
                    seniority: Seniority::Senior,
                    currency,
                    base_date: base,
                    discount_curve_id: "TEST-DISC".into(),
                    recovery_rate,
                    notional: 1.0,
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                    par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
                    doc_clause: None,
                }),
            }],
        };

        let envelope = CalibrationEnvelope {
            schema: "finstack.calibration/2".to_string(),
            plan,
            initial_market: Some((&initial_market).into()),
        };

        let result = engine::execute(&envelope).expect("canonical execute");
        let canonical_ctx =
            MarketContext::try_from(result.result.final_market).expect("restore context");
        let canonical_curve = canonical_ctx
            .get_hazard(hazard_id.as_str())
            .expect("canonical hazard curve");

        assert_eq!(
            legacy_curve.day_count(),
            canonical_curve.day_count(),
            "legacy bootstrap should use the same day count as canonical hazard calibration"
        );

        let legacy_knots: Vec<f64> = legacy_curve.knot_points().map(|(t, _)| t).collect();
        let canonical_knots: Vec<f64> = canonical_curve.knot_points().map(|(t, _)| t).collect();
        assert_eq!(
            legacy_knots.len(),
            canonical_knots.len(),
            "legacy and canonical curves should have the same number of knots"
        );
        for (legacy_t, canonical_t) in legacy_knots.iter().zip(canonical_knots.iter()) {
            assert!(
                (legacy_t - canonical_t).abs() <= 1e-12,
                "legacy bootstrap should use canonical pillar times; legacy={legacy_t}, canonical={canonical_t}"
            );
        }
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

    // ─── Restructuring clause / doc_clause tests ───────────────────────

    #[test]
    fn test_xr14_regression_matches_baseline() {
        // Xr14 (no restructuring) should produce the same output as a CDS without
        // any explicit doc_clause, since the default convention (IsdaNa) maps to Xr14.
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        let cds_baseline =
            create_test_cds("CDS-BASELINE", as_of, as_of.add_months(60), 100.0, 0.40);

        let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

        let pricer = CDSPricer::new();

        let pv_baseline = pricer
            .pv_protection_leg_raw(&cds_baseline, &disc, &credit, as_of)
            .expect("should succeed");
        let pv_xr14 = pricer
            .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
            .expect("should succeed");

        // Both should be identical since IsdaNa convention defaults to Xr14
        assert!(
            (pv_baseline - pv_xr14).abs() < 1e-10,
            "Xr14 should match baseline (IsdaNa default). Baseline={}, Xr14={}",
            pv_baseline,
            pv_xr14,
        );
    }

    #[test]
    fn test_default_pricer_disables_restructuring_uplift() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

        let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_cr14.doc_clause = Some(CdsDocClause::Cr14);

        let pricer = CDSPricer::new();

        let pv_xr14 = pricer
            .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
            .expect("should succeed");
        let pv_cr14 = pricer
            .pv_protection_leg_raw(&cds_cr14, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            (pv_cr14 - pv_xr14).abs() < 1e-10,
            "Default pricer should not apply restructuring uplift. Cr14={}, Xr14={}",
            pv_cr14,
            pv_xr14,
        );
    }

    #[test]
    fn test_cr14_higher_protection_than_xr14_when_approximation_enabled() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

        let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_cr14.doc_clause = Some(CdsDocClause::Cr14);

        let pricer = CDSPricer::with_config(CDSPricerConfig {
            enable_restructuring_approximation: true,
            ..Default::default()
        });

        let pv_xr14 = pricer
            .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
            .expect("should succeed");
        let pv_cr14 = pricer
            .pv_protection_leg_raw(&cds_cr14, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            pv_cr14 > pv_xr14,
            "Cr14 protection should exceed Xr14 when approximation is enabled. Cr14={}, Xr14={}",
            pv_cr14,
            pv_xr14,
        );
    }

    #[test]
    fn test_restructuring_ordering_xr14_mr14_mm14_cr14() {
        // Protection PV should increase with broader restructuring coverage:
        // Xr14 <= Mr14 <= Mm14 <= Cr14
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        let clauses = [
            CdsDocClause::Xr14,
            CdsDocClause::Mr14,
            CdsDocClause::Mm14,
            CdsDocClause::Cr14,
        ];

        let pricer = CDSPricer::with_config(CDSPricerConfig {
            enable_restructuring_approximation: true,
            ..Default::default()
        });
        let mut pvs = Vec::new();

        for clause in &clauses {
            let mut cds = create_test_cds("CDS-TEST", as_of, as_of.add_months(60), 100.0, 0.40);
            cds.doc_clause = Some(*clause);
            let pv = pricer
                .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
                .expect("should succeed");
            pvs.push(pv);
        }

        for i in 0..pvs.len() - 1 {
            assert!(
                pvs[i] <= pvs[i + 1],
                "Protection PV should increase with broader restructuring: {:?}={} should be <= {:?}={}",
                clauses[i], pvs[i], clauses[i + 1], pvs[i + 1],
            );
        }
    }

    #[test]
    fn test_doc_clause_effective_defaults() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        // No explicit doc_clause with IsdaNa convention -> Xr14
        let cds_na = create_test_cds("CDS-NA", as_of, as_of.add_months(60), 100.0, 0.40);
        assert_eq!(cds_na.doc_clause_effective(), CdsDocClause::Xr14);

        // Explicit Cr14 should override convention default
        let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_cr14.doc_clause = Some(CdsDocClause::Cr14);
        assert_eq!(cds_cr14.doc_clause_effective(), CdsDocClause::Cr14);

        // Meta-clause IsdaEu should resolve to Mm14
        let mut cds_eu = create_test_cds("CDS-EU", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_eu.doc_clause = Some(CdsDocClause::IsdaEu);
        assert_eq!(cds_eu.doc_clause_effective(), CdsDocClause::Mm14);
    }

    #[test]
    fn test_max_deliverable_maturity_mapping() {
        assert_eq!(max_deliverable_maturity(CdsDocClause::Cr14), None);
        assert_eq!(max_deliverable_maturity(CdsDocClause::Mr14), Some(30));
        assert_eq!(max_deliverable_maturity(CdsDocClause::Mm14), Some(60));
        assert_eq!(max_deliverable_maturity(CdsDocClause::Xr14), Some(0));
        // Meta-clauses delegate
        assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaNa), Some(0));
        assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaEu), Some(60));
    }

    #[test]
    fn test_doc_clause_serde_roundtrip() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

        // With doc_clause set
        let mut cds_with = create_test_cds("CDS-SERDE", as_of, as_of.add_months(60), 100.0, 0.40);
        cds_with.doc_clause = Some(CdsDocClause::Cr14);
        let json = serde_json::to_string(&cds_with).expect("serialize should succeed");
        assert!(
            json.contains("doc_clause"),
            "JSON should contain doc_clause field"
        );
        let deser: CreditDefaultSwap =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deser.doc_clause, Some(CdsDocClause::Cr14));

        // Without doc_clause (None) - should not appear in JSON (skip_serializing_if)
        let cds_without =
            create_test_cds("CDS-SERDE-NONE", as_of, as_of.add_months(60), 100.0, 0.40);
        let json_without = serde_json::to_string(&cds_without).expect("serialize should succeed");
        assert!(
            !json_without.contains("doc_clause"),
            "JSON should NOT contain doc_clause when None"
        );
        let deser_without: CreditDefaultSwap =
            serde_json::from_str(&json_without).expect("deserialize should succeed");
        assert_eq!(deser_without.doc_clause, None);
    }

    #[test]
    fn test_doc_clause_backward_compatible_construction() {
        // Existing construction without doc_clause should still work
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("CDS-COMPAT", as_of, as_of.add_months(60), 100.0, 0.40);
        assert_eq!(cds.doc_clause, None);

        // Builder pattern should also work without doc_clause
        let cds_built = CreditDefaultSwap::builder()
            .id(finstack_core::types::InstrumentId::new("CDS-BUILDER"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .convention(crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa)
            .premium(
                crate::instruments::common_impl::parameters::legs::PremiumLegSpec {
                    start: as_of,
                    end: as_of.add_months(60),
                    frequency: finstack_core::dates::Tenor::quarterly(),
                    stub: finstack_core::dates::StubKind::ShortFront,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: Some("nyse".to_string()),
                    day_count: finstack_core::dates::DayCount::Act360,
                    spread_bp: Decimal::try_from(100.0).expect("valid"),
                    discount_curve_id: finstack_core::types::CurveId::new("USD-OIS"),
                },
            )
            .protection(
                crate::instruments::common_impl::parameters::legs::ProtectionLegSpec {
                    credit_curve_id: finstack_core::types::CurveId::new("TEST-CREDIT"),
                    recovery_rate: 0.40,
                    settlement_delay: 3,
                },
            )
            .build()
            .expect("builder should succeed without doc_clause");
        assert_eq!(cds_built.doc_clause, None);
        assert_eq!(cds_built.doc_clause_effective(), CdsDocClause::Xr14);
    }

    #[test]
    fn test_doc_clause_serde_backward_compat_deserialization() {
        // Simulate old serialized data by serializing a CDS, stripping the
        // doc_clause field from JSON, and verifying it still deserializes.
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("CDS-OLD", as_of, as_of.add_months(60), 100.0, 0.40);
        let json = serde_json::to_string(&cds).expect("serialize should succeed");

        // The JSON should not contain "doc_clause" since it is None
        assert!(
            !json.contains("doc_clause"),
            "Baseline CDS JSON should not contain doc_clause"
        );

        // Deserialize and verify backward compat
        let deser: CreditDefaultSwap = serde_json::from_str(&json)
            .expect("Should deserialize old JSON without doc_clause field");
        assert_eq!(deser.doc_clause, None);
        assert_eq!(deser.doc_clause_effective(), CdsDocClause::Xr14);
    }

    #[test]
    fn test_integration_method_recommendations_cover_boundaries() {
        assert_eq!(
            IntegrationMethod::recommended(0.5, false),
            IntegrationMethod::Midpoint
        );
        assert_eq!(
            IntegrationMethod::recommended(1.99, false),
            IntegrationMethod::Midpoint
        );
        assert_eq!(
            IntegrationMethod::recommended(2.0, false),
            IntegrationMethod::IsdaStandardModel
        );
        assert_eq!(
            IntegrationMethod::recommended(10.0, false),
            IntegrationMethod::IsdaStandardModel
        );
        assert_eq!(
            IntegrationMethod::recommended(10.01, false),
            IntegrationMethod::AdaptiveSimpson
        );
        assert_eq!(
            IntegrationMethod::recommended(0.25, true),
            IntegrationMethod::GaussianQuadrature
        );
        assert_eq!(
            IntegrationMethod::recommended(30.0, true),
            IntegrationMethod::GaussianQuadrature
        );
    }

    #[test]
    fn test_pricer_config_factories_helpers_and_validation_paths() {
        let standard = CDSPricerConfig::isda_standard();
        assert_eq!(
            standard.integration_method,
            IntegrationMethod::IsdaStandardModel
        );
        assert!(standard.use_isda_coupon_dates);
        assert!(standard.adaptive_steps);
        assert_eq!(
            standard.business_days_per_year,
            time_constants::BUSINESS_DAYS_PER_YEAR_US
        );

        let europe = CDSPricerConfig::isda_europe();
        assert_eq!(
            europe.business_days_per_year,
            time_constants::BUSINESS_DAYS_PER_YEAR_UK
        );
        assert_eq!(europe.integration_method, standard.integration_method);

        let asia = CDSPricerConfig::isda_asia();
        assert_eq!(
            asia.business_days_per_year,
            time_constants::BUSINESS_DAYS_PER_YEAR_JP
        );
        assert_eq!(asia.integration_method, standard.integration_method);

        let simplified = CDSPricerConfig::simplified();
        assert_eq!(simplified.integration_method, IntegrationMethod::Midpoint);
        assert!(!simplified.use_isda_coupon_dates);
        assert!(!simplified.adaptive_steps);
        assert_eq!(simplified.validated_gl_order(), 4);

        let mut invalid_gl = standard.clone();
        invalid_gl.gl_order = 3;
        assert_eq!(invalid_gl.validated_gl_order(), 8);

        let mut adaptive = simplified.clone();
        adaptive.adaptive_steps = true;
        adaptive.min_steps_per_year = 20;
        assert_eq!(adaptive.effective_steps(1.1), 20);
        assert_eq!(adaptive.effective_steps(3.0), 36);
        assert_eq!(simplified.effective_steps(30.0), simplified.steps_per_year);

        let pricer = CDSPricer::try_with_config(standard.clone()).expect("valid config");
        assert_eq!(
            pricer.config().business_days_per_year,
            standard.business_days_per_year
        );

        let invalid_cases = {
            let mut cases = Vec::new();

            let mut cfg = standard.clone();
            cfg.tolerance = 0.0;
            cases.push((cfg, "tolerance"));

            let mut cfg = standard.clone();
            cfg.steps_per_year = 0;
            cases.push((cfg, "steps_per_year"));

            let mut cfg = standard.clone();
            cfg.min_steps_per_year = 0;
            cases.push((cfg, "min_steps_per_year"));

            let mut cfg = standard.clone();
            cfg.bootstrap_max_iterations = 0;
            cases.push((cfg, "bootstrap_max_iterations"));

            let mut cfg = standard.clone();
            cfg.bootstrap_tolerance = 0.0;
            cases.push((cfg, "bootstrap_tolerance"));

            let mut cfg = standard.clone();
            cfg.business_days_per_year = 0.0;
            cases.push((cfg, "business_days_per_year"));

            let mut cfg = standard.clone();
            cfg.adaptive_max_depth = 0;
            cases.push((cfg, "adaptive_max_depth"));

            cases
        };

        for (cfg, needle) in invalid_cases {
            let err = cfg.validate().expect_err("config should be rejected");
            assert!(
                err.to_string().contains(needle),
                "expected validation error mentioning {needle}, got {err}"
            );
        }

        let mut bad_for_pricer = standard.clone();
        bad_for_pricer.steps_per_year = 0;
        assert!(
            CDSPricer::try_with_config(bad_for_pricer).is_err(),
            "try_with_config should reject invalid settings"
        );
    }

    #[test]
    fn test_max_deliverable_maturity_covers_remaining_meta_clauses_and_custom() {
        assert_eq!(max_deliverable_maturity(CdsDocClause::Custom), Some(0));
        assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaAs), Some(0));
        assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaAu), Some(0));
        assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaNz), Some(0));
    }

    #[test]
    fn test_bootstrap_hazard_curve_rejects_empty_quotes_and_handles_distressed_spreads() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.97), (3.0, 0.90), (5.0, 0.82)])
            .build()
            .expect("discount curve");

        let bootstrapper = CDSBootstrapper::new();
        let err = bootstrapper
            .bootstrap_hazard_curve(&[], 0.40, &disc, as_of)
            .expect_err("empty spread set should be rejected");
        assert!(matches!(
            err,
            Error::Input(finstack_core::InputError::TooFewPoints)
        ));

        let distressed = bootstrapper
            .bootstrap_hazard_curve(
                &[(1.0, 1_200.0), (3.0, 1_600.0), (5.0, 2_000.0)],
                0.25,
                &disc,
                as_of,
            )
            .expect("distressed spreads should still bootstrap");

        assert_eq!(distressed.base_date(), as_of);
        assert_eq!(distressed.recovery_rate(), 0.25);
        assert!(distressed.knot_points().count() >= 3);
    }

    #[test]
    fn test_schedule_generation_respects_isda_flag_and_calendar_availability() {
        let start = Date::from_calendar_date(2025, time::Month::July, 1).expect("valid date");
        let end = Date::from_calendar_date(2026, time::Month::July, 1).expect("valid date");
        let cds = create_test_cds("CDS-SCHED", start, end, 100.0, 0.40);

        let simplified = CDSPricer::with_config(CDSPricerConfig::simplified());
        let regular_schedule = simplified
            .generate_schedule(&cds, start)
            .expect("regular schedule");
        let expected_regular = crate::cashflow::builder::build_dates(
            cds.premium.start,
            cds.premium.end,
            cds.premium.frequency,
            cds.premium.stub,
            cds.premium.bdc,
            false,
            0,
            cds.premium
                .calendar_id
                .as_deref()
                .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
        )
        .expect("expected regular schedule")
        .dates;
        assert_eq!(regular_schedule, expected_regular);

        let isda = CDSPricer::new();
        let adjusted_schedule = isda
            .generate_isda_schedule(&cds)
            .expect("adjusted ISDA schedule");
        assert_ne!(
            regular_schedule, adjusted_schedule,
            "non-ISDA schedule should differ from IMM-style ISDA schedule"
        );

        let mut cds_no_calendar = cds.clone();
        cds_no_calendar.premium.calendar_id = None;
        let unadjusted_schedule = isda
            .generate_isda_schedule(&cds_no_calendar)
            .expect("unadjusted ISDA schedule");

        let sep_20 =
            Date::from_calendar_date(2025, time::Month::September, 20).expect("valid date");
        let sep_22 =
            Date::from_calendar_date(2025, time::Month::September, 22).expect("valid date");
        assert!(
            unadjusted_schedule.contains(&sep_20),
            "calendar-less ISDA schedule should keep weekend IMM dates"
        );
        assert!(
            adjusted_schedule.contains(&sep_22),
            "calendar-aware ISDA schedule should adjust weekend IMM dates"
        );
        assert!(
            !adjusted_schedule.contains(&sep_20),
            "calendar-aware ISDA schedule should not keep the unadjusted weekend date"
        );
    }

    #[test]
    fn test_premium_leg_per_bp_matches_risky_annuity_without_accrual_on_default() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let cds = create_test_cds("CDS-PER-BP", as_of, as_of.add_months(60), 100.0, 0.40);

        let without_aod = CDSPricer::with_config(CDSPricerConfig {
            include_accrual: false,
            ..Default::default()
        });
        let risky_annuity = without_aod
            .risky_annuity(&cds, &disc, &credit, as_of)
            .expect("risky annuity");
        let per_bp_without_aod = without_aod
            .premium_leg_pv_per_bp(&cds, &disc, &credit, as_of)
            .expect("premium leg per bp");
        assert!(
            (per_bp_without_aod - risky_annuity * ONE_BASIS_POINT).abs() < 1e-14,
            "premium leg per bp without AoD should equal risky annuity × 1bp"
        );

        let with_aod = CDSPricer::new();
        let per_bp_with_aod = with_aod
            .premium_leg_pv_per_bp(&cds, &disc, &credit, as_of)
            .expect("premium leg per bp with AoD");
        assert!(
            per_bp_with_aod > per_bp_without_aod,
            "including AoD should increase premium leg PV per bp"
        );
    }

    #[test]
    fn test_full_premium_par_spread_is_below_risky_annuity_par_spread() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.975), (5.0, 0.86), (10.0, 0.72)])
            .build()
            .expect("discount curve");
        let credit = HazardCurve::builder("TEST-CREDIT")
            .base_date(as_of)
            .recovery_rate(0.40)
            .knots(vec![(0.25, 0.08), (1.0, 0.12), (3.0, 0.16), (5.0, 0.20)])
            .build()
            .expect("hazard curve");
        let cds = create_test_cds("CDS-PAR-FULL", as_of, as_of.add_months(60), 100.0, 0.40);

        let isda = CDSPricer::new();
        let full_premium = CDSPricer::with_config(CDSPricerConfig {
            par_spread_uses_full_premium: true,
            ..Default::default()
        });

        let isda_spread = isda
            .par_spread(&cds, &disc, &credit, as_of)
            .expect("ISDA par spread");
        let full_spread = full_premium
            .par_spread(&cds, &disc, &credit, as_of)
            .expect("full-premium par spread");

        assert!(isda_spread.is_finite() && full_spread.is_finite());
        assert!(
            full_spread < isda_spread,
            "including AoD in the denominator should reduce the par spread"
        );
    }

    #[test]
    fn test_npv_with_upfront_combines_dated_and_market_quote_adjustments() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let mut cds = create_test_cds("CDS-UPFRONT", as_of, as_of.add_months(60), 100.0, 0.40);
        let pricer = CDSPricer::new();

        let base_npv = pricer
            .npv(&cds, &disc, &credit, as_of)
            .expect("base npv")
            .amount();

        let dated_upfront_date = as_of.add_months(6);
        let dated_upfront_amount = 150_000.0;
        let quote_adjustment = Money::new(25_000.0, Currency::USD);
        cds.upfront = Some((
            dated_upfront_date,
            Money::new(dated_upfront_amount, Currency::USD),
        ));
        cds.pricing_overrides.market_quotes.upfront_payment = Some(quote_adjustment);

        let dated_df = disc
            .df_between_dates(as_of, dated_upfront_date)
            .expect("discount factor");
        let expected = base_npv - dated_upfront_amount * dated_df + quote_adjustment.amount();
        let npv_with_upfront = pricer
            .npv_with_upfront(&cds, &disc, &credit, as_of)
            .expect("npv with upfront")
            .amount();
        assert!(
            (npv_with_upfront - expected).abs() < 1e-8,
            "dated upfront and direct PV adjustment should combine additively"
        );

        let market = MarketContext::new()
            .insert(disc.clone())
            .insert(credit.clone());
        let npv_market = pricer
            .npv_market(&cds, &market, as_of)
            .expect("market npv")
            .amount();
        assert!(
            (npv_market - npv_with_upfront).abs() < 1e-12,
            "npv_market should match direct-curve npv_with_upfront"
        );
    }

    #[test]
    fn test_time_and_settlement_helpers_match_curve_and_calendar_conventions() {
        let (disc, credit) = create_test_curves();
        let base_date = disc.base_date();
        let one_year = base_date.add_months(12);

        let expected_disc_t = disc
            .day_count()
            .year_fraction(
                base_date,
                one_year,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("discount year fraction");
        assert!(
            (disc_t(&disc, one_year).expect("disc_t") - expected_disc_t).abs() < 1e-12,
            "disc_t should respect the discount curve day-count"
        );

        let expected_haz_t = credit
            .day_count()
            .year_fraction(
                credit.base_date(),
                one_year,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("hazard year fraction");
        assert!(
            (haz_t(&credit, one_year).expect("haz_t") - expected_haz_t).abs() < 1e-12,
            "haz_t should respect the hazard curve day-count"
        );

        assert_eq!(
            date_from_hazard_time(&credit, -1.0),
            credit.base_date(),
            "negative hazard times should clamp to the curve base date"
        );
        let days_per_year: f64 = match credit.day_count() {
            DayCount::Act360 => 360.0,
            DayCount::Act365F => 365.0,
            DayCount::Act365L | DayCount::ActAct | DayCount::ActActIsma => 365.25,
            DayCount::Thirty360 | DayCount::ThirtyE360 => 360.0,
            DayCount::Bus252 => 252.0,
            _ => 365.25,
        };
        let hazard_time = 1.25_f64;
        let expected_date =
            credit.base_date() + Duration::days((hazard_time * days_per_year).round() as i64);
        assert_eq!(date_from_hazard_time(&credit, hazard_time), expected_date);

        let fallback_settlement = settlement_date(base_date, 3, None, 252.0).expect("fallback");
        assert_eq!(
            fallback_settlement,
            base_date + Duration::days(4),
            "3 business days at 252 bdays/year should round to 4 calendar days"
        );

        let nyse = finstack_core::dates::fx::resolve_calendar(Some("nyse")).expect("nyse calendar");
        let friday =
            Date::from_calendar_date(2025, time::Month::January, 3).expect("valid Friday date");
        let monday =
            Date::from_calendar_date(2025, time::Month::January, 6).expect("valid Monday date");
        assert_eq!(
            settlement_date(friday, 1, Some(nyse.as_holiday_calendar()), 252.0)
                .expect("calendar settlement"),
            monday,
            "calendar-aware settlement should advance by business days"
        );

        let midpoint = midpoint_default_date(&credit, base_date, one_year).expect("midpoint");
        let midpoint_time = 0.5
            * (haz_t(&credit, base_date).expect("haz_t start")
                + haz_t(&credit, one_year).expect("haz_t end"));
        assert_eq!(midpoint, date_from_hazard_time(&credit, midpoint_time));
    }

    #[test]
    fn test_discount_survival_and_default_density_helpers_cover_boundary_cases() {
        let (disc, credit) = create_test_curves();
        let as_of = disc.base_date();
        let one_year = as_of.add_months(12);

        assert_eq!(
            df_asof_to(&disc, as_of, one_year).expect("df"),
            disc.df_between_dates(as_of, one_year)
                .expect("df between dates")
        );

        let t_asof = haz_t(&credit, as_of).expect("haz_t as_of");
        let t_one_year = haz_t(&credit, one_year).expect("haz_t future");
        let expected_conditional_survival = credit.sp(t_one_year) / credit.sp(t_asof);
        assert!(
            (sp_cond_to(&credit, as_of, one_year).expect("conditional survival")
                - expected_conditional_survival)
                .abs()
                < 1e-12
        );

        let mut late_as_of = as_of;
        while credit.sp(haz_t(&credit, late_as_of).expect("haz_t late"))
            > credit::SURVIVAL_PROBABILITY_FLOOR
        {
            late_as_of = late_as_of.add_months(600);
        }
        assert_eq!(
            sp_cond_to(&credit, late_as_of, late_as_of.add_months(12))
                .expect("conditional survival after effective default"),
            0.0,
            "conditional survival should floor to zero after effective default"
        );

        let t_start = 0.5;
        let t_end = 1.5;
        let h = 0.1;
        let center_density = approx_default_density(&credit, 1.0, h, t_start, t_end);
        let expected_center_density = -((credit.sp(1.0 + h) - credit.sp(1.0 - h)) / (2.0 * h));
        assert!(
            (center_density - expected_center_density.max(0.0)).abs() < 1e-12,
            "interior default density should use central differences"
        );
        assert!(approx_default_density(&credit, t_start, 0.0, t_start, t_end) >= 0.0);
        assert!(approx_default_density(&credit, t_end, 0.0, t_start, t_end) >= 0.0);
    }

    #[test]
    fn test_restructuring_adjustment_factor_scales_with_clause_and_remaining_tenor() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let short_cds = create_test_cds("CDS-1Y", as_of, as_of.add_months(12), 100.0, 0.40);
        let long_cds = create_test_cds("CDS-10Y", as_of, as_of.add_months(120), 100.0, 0.40);

        assert_eq!(
            restructuring_adjustment_factor(CdsDocClause::Xr14, &short_cds),
            1.0
        );
        assert_eq!(
            restructuring_adjustment_factor(CdsDocClause::Custom, &short_cds),
            1.0
        );
        assert_eq!(
            restructuring_adjustment_factor(CdsDocClause::Mr14, &short_cds),
            1.02
        );
        assert_eq!(
            restructuring_adjustment_factor(CdsDocClause::Mm14, &short_cds),
            1.03
        );
        assert_eq!(
            restructuring_adjustment_factor(CdsDocClause::Cr14, &short_cds),
            1.05
        );

        let mr14_long = restructuring_adjustment_factor(CdsDocClause::Mr14, &long_cds);
        let mm14_long = restructuring_adjustment_factor(CdsDocClause::Mm14, &long_cds);
        let cr14_long = restructuring_adjustment_factor(CdsDocClause::Cr14, &long_cds);
        assert!(
            mr14_long > 1.0 && mr14_long < 1.02,
            "modified restructuring should be partially scaled for long tenors"
        );
        assert!(
            mm14_long > mr14_long && mm14_long < 1.03,
            "modified-modified restructuring should sit between MR14 and its full uplift"
        );
        assert_eq!(cr14_long, 1.05);
    }

    #[test]
    fn test_bootstrap_convention_defaults_and_representative_keys_match_regions() {
        let default_convention = BootstrapConvention::default();
        assert!(default_convention.use_imm_dates);
        assert_eq!(
            default_convention.representative_convention_key(),
            CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::IsdaNa,
            }
        );

        let eu = BootstrapConvention {
            convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaEu,
            use_imm_dates: true,
        };
        assert_eq!(
            eu.representative_convention_key(),
            CdsConventionKey {
                currency: Currency::EUR,
                doc_clause: CdsDocClause::IsdaEu,
            }
        );

        let asia = BootstrapConvention {
            convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaAs,
            use_imm_dates: true,
        };
        assert_eq!(
            asia.representative_convention_key(),
            CdsConventionKey {
                currency: Currency::JPY,
                doc_clause: CdsDocClause::IsdaAs,
            }
        );

        let custom = BootstrapConvention {
            convention: crate::instruments::credit_derivatives::cds::CDSConvention::Custom,
            use_imm_dates: false,
        };
        assert_eq!(
            custom.representative_convention_key(),
            CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::Custom,
            }
        );

        let bootstrapper = CDSBootstrapper::default();
        assert_eq!(
            bootstrapper.config.integration_method,
            CDSPricerConfig::default().integration_method
        );
        assert_eq!(
            bootstrapper.convention.representative_convention_key(),
            default_convention.representative_convention_key()
        );
    }

    // ── Forward-starting CDS tests ──────────────────────────────────────

    /// Helper: create a forward-starting CDS with a specified protection effective date.
    fn create_forward_start_cds(
        id: impl Into<String>,
        start_date: Date,
        end_date: Date,
        spread_bp: f64,
        recovery_rate: f64,
        protection_effective_date: Option<Date>,
    ) -> CreditDefaultSwap {
        let mut cds = create_test_cds(id, start_date, end_date, spread_bp, recovery_rate);
        cds.protection_effective_date = protection_effective_date;
        cds.validate().expect("forward-start CDS should validate");
        cds
    }

    #[test]
    fn test_forward_start_none_matches_spot_cds() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);

        let spot_cds = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
        let fwd_none = create_forward_start_cds("CDS-FWD-NONE", as_of, end, 100.0, 0.40, None);

        let pricer = CDSPricer::new();

        let spot_prot = pricer
            .pv_protection_leg_raw(&spot_cds, &disc, &credit, as_of)
            .expect("should succeed");
        let fwd_prot = pricer
            .pv_protection_leg_raw(&fwd_none, &disc, &credit, as_of)
            .expect("should succeed");

        let spot_prem = pricer
            .pv_premium_leg_raw(&spot_cds, &disc, &credit, as_of)
            .expect("should succeed");
        let fwd_prem = pricer
            .pv_premium_leg_raw(&fwd_none, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            (spot_prot - fwd_prot).abs() < 1e-10,
            "None protection_effective_date should match spot: spot={spot_prot}, fwd={fwd_prot}",
        );
        assert!(
            (spot_prem - fwd_prem).abs() < 1e-10,
            "None protection_effective_date should match spot premium: spot={spot_prem}, fwd={fwd_prem}",
        );
    }

    #[test]
    fn test_forward_start_lower_protection_pv_same_premium_pv() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);
        let fwd_date = as_of.add_months(24);

        let spot_cds = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
        let fwd_cds = create_forward_start_cds("CDS-FWD", as_of, end, 100.0, 0.40, Some(fwd_date));

        let pricer = CDSPricer::new();

        let spot_prot = pricer
            .pv_protection_leg_raw(&spot_cds, &disc, &credit, as_of)
            .expect("should succeed");
        let fwd_prot = pricer
            .pv_protection_leg_raw(&fwd_cds, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            fwd_prot < spot_prot,
            "Forward-start protection PV ({fwd_prot}) should be less than spot ({spot_prot})",
        );
        assert!(
            fwd_prot > 0.0,
            "Forward-start protection PV should still be positive"
        );

        let spot_prem = pricer
            .pv_premium_leg_raw(&spot_cds, &disc, &credit, as_of)
            .expect("should succeed");
        let fwd_prem = pricer
            .pv_premium_leg_raw(&fwd_cds, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            (spot_prem - fwd_prem).abs() < 1e-10,
            "Premium leg should be identical: spot={spot_prem}, fwd={fwd_prem}",
        );
    }

    #[test]
    fn test_forward_start_protection_at_end_near_zero() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);

        let fwd_cds = create_forward_start_cds("CDS-FWD-END", as_of, end, 100.0, 0.40, Some(end));

        let pricer = CDSPricer::new();
        let prot_pv = pricer
            .pv_protection_leg_raw(&fwd_cds, &disc, &credit, as_of)
            .expect("should succeed");

        assert!(
            prot_pv.abs() < 1.0,
            "Protection PV should be ~0 when effective_date = end, got {prot_pv}",
        );
    }

    #[test]
    fn test_forward_start_invalid_before_premium_start() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);
        let before_start =
            Date::from_calendar_date(2024, time::Month::June, 1).expect("valid date");

        let mut cds = create_test_cds("CDS-BAD", as_of, end, 100.0, 0.40);
        cds.protection_effective_date = Some(before_start);
        let result = cds.validate();
        assert!(
            result.is_err(),
            "protection_effective_date before premium start should fail"
        );
    }

    #[test]
    fn test_forward_start_invalid_after_premium_end() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);
        let after_end = end.add_months(12);

        let mut cds = create_test_cds("CDS-BAD", as_of, end, 100.0, 0.40);
        cds.protection_effective_date = Some(after_end);
        let result = cds.validate();
        assert!(
            result.is_err(),
            "protection_effective_date after premium end should fail"
        );
    }

    #[test]
    fn test_protection_start_helper() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let end = as_of.add_months(60);
        let fwd_date = as_of.add_months(24);

        let spot = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
        assert_eq!(spot.protection_start(), as_of);

        let mut fwd = spot.clone();
        fwd.protection_effective_date = Some(fwd_date);
        assert_eq!(fwd.protection_start(), fwd_date);
    }
}
