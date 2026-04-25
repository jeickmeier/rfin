//! Hazard curve adapter for stochastic default modeling.
//!
//! Provides an adapter that wraps a [`HazardCurve`] from the core library
//! to implement the [`StochasticDefault`] trait. This allows using market-calibrated
//! hazard curves (e.g., from CDS spreads) as the basis for stochastic default models.
//!
//! # Mathematical Model
//!
//! The adapter applies factor-based shocks to the hazard curve's survival probability:
//! ```text
//! λ_shocked(t) = λ_base(t) × exp(β × Z × σ)
//! ```
//!
//! where:
//! - λ_base(t) is the hazard rate from the HazardCurve
//! - β is the factor sensitivity parameter
//! - Z is the systematic factor realization
//! - σ is the volatility of the intensity shock
//!
//! # Use Cases
//!
//! - Use CDS-calibrated hazard curves for CLO/CDO pricing
//! - Apply stochastic shocks to market-implied default rates
//! - Bridge between credit curves and structured credit models
//!
//! # References
//!
//! - Duffie, D., & Singleton, K. J. (1999). "Modeling Term Structures of Defaultable Bonds."
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*.

#![allow(dead_code)]

use super::traits::{MacroCreditFactors, StochasticDefault};
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::math::distributions::binomial_distribution;

/// Adapter that wraps a HazardCurve to provide a [`StochasticDefault`] interface.
///
/// Uses the hazard curve's survival probability with factor-based shocks
/// to generate conditional default rates for structured credit modeling.
///
/// # Example
///
/// ```text
/// use finstack_valuations::instruments::fixed_income::structured_credit::pricing::stochastic::default::HazardCurveDefault;
/// use finstack_core::market_data::term_structures::HazardCurve;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of = Date::from_calendar_date(2025, Month::January, 1)?;
///
/// // Build hazard curve from CDS spreads
/// let hazard_curve = HazardCurve::builder("CORP-CREDIT")
///     .base_date(as_of)
///     .knots([(1.0, 0.02), (3.0, 0.025), (5.0, 0.03)])
///     .build()?;
///
/// // Wrap in stochastic adapter with factor sensitivity
/// let stochastic_default = HazardCurveDefault::new(hazard_curve, 0.5);
/// # let _ = stochastic_default;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub(crate) struct HazardCurveDefault {
    /// The underlying hazard curve
    hazard_curve: HazardCurve,
    /// Factor sensitivity (β) for systematic risk
    factor_sensitivity: f64,
    /// Volatility of intensity shocks (σ)
    volatility: f64,
    /// Asset correlation for default distribution calculation
    correlation: f64,
}

impl HazardCurveDefault {
    /// Create a new hazard curve default adapter.
    ///
    /// # Arguments
    ///
    /// * `hazard_curve` - The underlying calibrated hazard curve
    /// * `factor_sensitivity` - Sensitivity to systematic factor shocks (typical: 0.3-0.8)
    pub(crate) fn new(hazard_curve: HazardCurve, factor_sensitivity: f64) -> Self {
        Self {
            hazard_curve,
            factor_sensitivity: factor_sensitivity.clamp(-2.0, 2.0),
            volatility: 0.30,  // Default volatility
            correlation: 0.20, // Default correlation
        }
    }

    /// Create with specified volatility.
    pub(crate) fn with_volatility(mut self, volatility: f64) -> Self {
        self.volatility = volatility.clamp(0.0, 2.0);
        self
    }

    /// Create with specified correlation.
    pub(crate) fn with_correlation(mut self, correlation: f64) -> Self {
        self.correlation = correlation.clamp(0.0, 0.99);
        self
    }

    /// Standard RMBS calibration using a hazard curve.
    ///
    /// - Factor sensitivity: 0.5
    /// - Volatility: 0.30
    /// - Correlation: 5% (low for diversified pools)
    pub(crate) fn rmbs_standard(hazard_curve: HazardCurve) -> Self {
        Self::new(hazard_curve, 0.5)
            .with_volatility(0.30)
            .with_correlation(0.05)
    }

    /// Standard CLO calibration using a hazard curve.
    ///
    /// - Factor sensitivity: 0.8
    /// - Volatility: 0.40
    /// - Correlation: 25% (higher for corporate loans)
    pub(crate) fn clo_standard(hazard_curve: HazardCurve) -> Self {
        Self::new(hazard_curve, 0.8)
            .with_volatility(0.40)
            .with_correlation(0.25)
    }

    /// Get the underlying hazard curve.
    pub(crate) fn hazard_curve(&self) -> &HazardCurve {
        &self.hazard_curve
    }

    /// Get the factor sensitivity.
    pub(crate) fn factor_sensitivity(&self) -> f64 {
        self.factor_sensitivity
    }

    /// Get the volatility.
    pub(crate) fn volatility(&self) -> f64 {
        self.volatility
    }

    /// Calculate the shocked hazard rate at a given time and factor realization.
    ///
    /// The shock is multiplicative: λ_shocked = λ_base × exp(β × Z × σ)
    fn shocked_hazard_multiplier(&self, factor: f64) -> f64 {
        (self.factor_sensitivity * factor * self.volatility).exp()
    }

    /// Convert hazard rate to monthly default rate.
    ///
    /// For constant hazard λ, MDR = 1 - exp(-λ/12)
    fn hazard_to_mdr(hazard: f64) -> f64 {
        let monthly_hazard = hazard / 12.0;
        (1.0 - (-monthly_hazard).exp()).clamp(0.0, 1.0)
    }
}

impl StochasticDefault for HazardCurveDefault {
    fn conditional_mdr(
        &self,
        seasoning: u32,
        factors: &[f64],
        _macro_factors: &MacroCreditFactors,
    ) -> f64 {
        // Convert seasoning (months) to years
        let t_years = seasoning as f64 / 12.0;

        // Get base hazard rate from curve
        let base_hazard = self.hazard_curve.hazard_rate(t_years);

        // Apply factor shock
        let z = factors.first().copied().unwrap_or(0.0);
        let shock_multiplier = self.shocked_hazard_multiplier(z);
        let shocked_hazard = base_hazard * shock_multiplier;

        // Convert to MDR
        Self::hazard_to_mdr(shocked_hazard)
    }

    fn default_distribution(
        &self,
        n: usize,
        pds: &[f64],
        factors: &[f64],
        _correlation: f64,
    ) -> Vec<f64> {
        // Get base PD or use curve-implied default probability
        let z = factors.first().copied().unwrap_or(0.0);
        let shock_multiplier = self.shocked_hazard_multiplier(z);

        let cond_pd = if !pds.is_empty() {
            // Scale provided PD by shock
            (pds[0] * shock_multiplier).min(0.9999)
        } else {
            // Use 1-year default probability from curve
            let base_1y_dp = 1.0 - self.hazard_curve.sp(1.0);
            (base_1y_dp * shock_multiplier).min(0.9999)
        };

        // Use the core binomial distribution function
        binomial_distribution(n, cond_pd.clamp(0.0, 1.0))
    }

    fn correlation(&self) -> f64 {
        self.correlation
    }

    fn model_name(&self) -> &'static str {
        "Hazard Curve Default Model"
    }

    fn expected_mdr(&self, seasoning: u32) -> f64 {
        // Unconditional MDR (no factor shock, Z=0)
        let t_years = seasoning as f64 / 12.0;
        let base_hazard = self.hazard_curve.hazard_rate(t_years);
        Self::hazard_to_mdr(base_hazard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use time::Month;

    fn test_hazard_curve() -> HazardCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        HazardCurve::builder("TEST-CREDIT")
            .base_date(base)
            .knots([(0.0, 0.02), (5.0, 0.03), (10.0, 0.04)])
            .build()
            .expect("Valid hazard curve")
    }

    #[test]
    fn test_hazard_curve_default_creation() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5);

        assert!((model.factor_sensitivity() - 0.5).abs() < 1e-10);
        assert!((model.correlation() - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_conditional_mdr_at_zero_factor() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5).with_correlation(0.20);
        let factors = MacroCreditFactors::default();

        // At Z=0, MDR should be based on base hazard only
        let mdr = model.conditional_mdr(12, &[0.0], &factors);

        // MDR should be positive and reasonable
        assert!(mdr > 0.0, "MDR should be positive");
        assert!(mdr < 0.01, "MDR should be less than 1% monthly");
    }

    #[test]
    fn test_factor_shock_direction() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5).with_volatility(0.5);
        let factors = MacroCreditFactors::default();

        let mdr_neg = model.conditional_mdr(12, &[-2.0], &factors);
        let mdr_zero = model.conditional_mdr(12, &[0.0], &factors);
        let mdr_pos = model.conditional_mdr(12, &[2.0], &factors);

        // Positive factor with positive sensitivity should increase MDR
        // (exp(β * Z * σ) > 1 when β, Z, σ > 0)
        assert!(
            mdr_pos > mdr_zero,
            "Positive factor should increase MDR: {} > {}",
            mdr_pos,
            mdr_zero
        );
        assert!(
            mdr_neg < mdr_zero,
            "Negative factor should decrease MDR: {} < {}",
            mdr_neg,
            mdr_zero
        );
    }

    #[test]
    fn test_default_distribution_sums_to_one() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5);

        let dist = model.default_distribution(10, &[0.05], &[0.0], 0.20);

        let sum: f64 = dist.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Distribution should sum to 1");
    }

    #[test]
    fn test_standard_calibrations() {
        let hc = test_hazard_curve();

        let rmbs = HazardCurveDefault::rmbs_standard(hc.clone());
        assert!((rmbs.factor_sensitivity() - 0.5).abs() < 1e-10);
        assert!((rmbs.correlation() - 0.05).abs() < 1e-10);

        let clo = HazardCurveDefault::clo_standard(hc);
        assert!((clo.factor_sensitivity() - 0.8).abs() < 1e-10);
        assert!((clo.correlation() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_model_name() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5);

        assert_eq!(model.model_name(), "Hazard Curve Default Model");
    }

    #[test]
    fn test_expected_mdr() {
        let hc = test_hazard_curve();
        let model = HazardCurveDefault::new(hc, 0.5);

        let expected = model.expected_mdr(12);

        // Expected MDR should match conditional MDR at Z=0
        let factors = MacroCreditFactors::default();
        let conditional = model.conditional_mdr(12, &[0.0], &factors);

        assert!(
            (expected - conditional).abs() < 1e-10,
            "Expected MDR should equal conditional at Z=0"
        );
    }
}
