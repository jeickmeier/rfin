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
#![allow(unused_imports)] // Re-exports for API stability

mod bootstrap;
mod config;
mod engine;
mod helpers;
mod integration;
mod metrics;

#[cfg(test)]
mod tests;

pub(crate) use bootstrap::{BootstrapConvention, CDSBootstrapper};
pub(crate) use config::{max_deliverable_maturity, CDSPricerConfig};
pub(crate) use engine::CDSPricer;

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
pub(crate) enum IntegrationMethod {
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
    pub(crate) fn recommended(tenor_years: f64, is_distressed: bool) -> Self {
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
