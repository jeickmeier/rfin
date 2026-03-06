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
// Allow dead_code for public API items exposed via Python (finstack-py) and WASM bindings.
// Key items: CDSPricer, CDSPricerConfig, IntegrationMethod, CDSBootstrapper.
#![allow(dead_code)]
use crate::constants::{
    credit, isda, numerical, time as time_constants, BASIS_POINTS_PER_UNIT, ONE_BASIS_POINT,
};
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::credit_derivatives::cds::{CdsDocClause, CreditDefaultSwap, PayReceive};
use finstack_core::currency::Currency;
use finstack_core::dates::DateExt;
use finstack_core::dates::{adjust, next_cds_date, Date, DayCount, HolidayCalendar};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::money::Money;
use finstack_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
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
    as_of: Date,
    disc: &'a DiscountCurve,
    surv: &'a HazardCurve,
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

        // Determine the effective protection end considering the restructuring clause.
        // For clauses with a maturity cap (Mr14, Mm14), the effective protection end
        // for the restructuring component is limited. For Xr14, restructuring provides
        // no additional protection. For Cr14, there is no cap.
        let effective_clause = cds.doc_clause_effective();
        let restructuring_factor = restructuring_adjustment_factor(effective_clause, cds);

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

            // Accrual uses instrument's day-count convention (e.g., Act/360 for ISDA NA)
            let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, end_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            premium_pv += spread * accrual * sp * df;

            if self.config.include_accrual {
                // Accrual-on-default contribution for this period
                premium_pv += self.accrual_on_default_isda_midpoint(AodInputs {
                    cds,
                    spread,
                    start_date: start_date.max(as_of),
                    end_date,
                    as_of,
                    disc,
                    surv,
                })?;
            }
        }

        Ok(premium_pv * cds.notional.amount())
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

        // Discount to the *premium payment date* (end_date) from as_of.
        let df = df_asof_to(inp.disc, inp.as_of, inp.end_date)?;

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
            let schedule = self.generate_schedule(cds, as_of)?;
            let mut ann = 0.0;
            for i in 0..schedule.len() - 1 {
                let start_date = schedule[i];
                let end_date = schedule[i + 1];

                // Skip periods that have already ended before as_of
                if end_date <= as_of {
                    continue;
                }

                // Accrual uses instrument day-count
                let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

                // Discounting uses discount curve's day-count and relative DF from as_of
                let df = df_asof_to(disc, as_of, end_date)?;

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
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut per_bp_pv = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, end_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            per_bp_pv += ONE_BASIS_POINT * accrual * sp * df;

            if self.config.include_accrual {
                per_bp_pv += self.accrual_on_default_isda_midpoint(AodInputs {
                    cds,
                    spread: ONE_BASIS_POINT,
                    start_date: start_date.max(as_of),
                    end_date,
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
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut annuity = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = year_fraction(cds.premium.day_count, start_date, end_date)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, end_date)?;

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
    fn test_cr14_higher_protection_than_xr14() {
        // Full restructuring (Cr14) should give equal-or-higher protection PV
        // because restructuring adds credit event types.
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
            pv_cr14 > pv_xr14,
            "Cr14 protection should exceed Xr14. Cr14={}, Xr14={}",
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

        let pricer = CDSPricer::new();
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
