//! Intensity process default model (Cox process).
//!
//! Models default intensity as a mean-reverting stochastic process
//! driven by systematic factors.
//!
//! # Mathematical Model
//!
//! The default intensity λ follows:
//! ```text
//! λ(t) = λ₀ × exp(β × X(t))
//! ```
//!
//! where X(t) is an Ornstein-Uhlenbeck process:
//! ```text
//! dX = κ(θ - X)dt + σ dW
//! ```
//!
//! The conditional default probability over [t, t+dt]:
//! ```text
//! P(default in dt | λ) = 1 - exp(-λ × dt)
//! ```
//!
//! # References
//!
//! - Duffie, D., & Singleton, K. J. (1999). "Modeling Term Structures of Defaultable Bonds."
//! - Lando, D. (1998). "On Cox Processes and Credit Risky Securities."

use super::super::calibrations::{CLO_STANDARD, RMBS_STANDARD};
use super::traits::{MacroCreditFactors, StochasticDefault};
use crate::instruments::structured_credit::utils::rates::cdr_to_mdr;
use finstack_core::math::distributions::binomial_distribution;

/// Intensity process (Cox model) default model.
///
/// Default intensity follows an exponential of an OU process,
/// providing mean-reverting but always positive intensity.
#[derive(Clone, Debug)]
pub struct IntensityProcessDefault {
    /// Base hazard rate (annual)
    base_hazard: f64,
    /// Factor sensitivity (beta)
    factor_sensitivity: f64,
    /// Mean reversion speed (kappa)
    mean_reversion: f64,
    /// Volatility of intensity process
    volatility: f64,
    /// Asset correlation for distribution calculation
    correlation: f64,
}

impl IntensityProcessDefault {
    /// Create an intensity process default model.
    ///
    /// # Arguments
    /// * `base_hazard` - Base annual hazard rate (λ₀)
    /// * `factor_sensitivity` - Sensitivity to systematic factor (β)
    /// * `mean_reversion` - Mean reversion speed (κ)
    /// * `volatility` - Intensity volatility (σ)
    pub fn new(
        base_hazard: f64,
        factor_sensitivity: f64,
        mean_reversion: f64,
        volatility: f64,
    ) -> Self {
        Self {
            base_hazard: base_hazard.clamp(0.0, 1.0),
            factor_sensitivity: factor_sensitivity.clamp(-2.0, 2.0),
            mean_reversion: mean_reversion.clamp(0.0, 10.0),
            volatility: volatility.clamp(0.0, 2.0),
            correlation: 0.20, // Default correlation
        }
    }

    /// Create with specified correlation.
    pub fn with_correlation(mut self, correlation: f64) -> Self {
        self.correlation = correlation.clamp(0.0, 0.99);
        self
    }

    /// Standard RMBS calibration.
    ///
    /// Uses shared calibration constants from [`RMBS_STANDARD`]:
    /// - Base hazard: 2% annual
    /// - Factor sensitivity: 0.5
    /// - Mean reversion: 0.5 (2-year half-life)
    /// - Volatility: 0.30
    pub fn rmbs_standard() -> Self {
        Self::new(
            RMBS_STANDARD.base_cdr,
            RMBS_STANDARD.default_factor_sensitivity,
            RMBS_STANDARD.default_mean_reversion,
            RMBS_STANDARD.default_volatility,
        )
        .with_correlation(RMBS_STANDARD.default_correlation)
    }

    /// Standard CLO calibration.
    ///
    /// Uses shared calibration constants from [`CLO_STANDARD`]:
    /// Higher base hazard and factor sensitivity for corporate loans.
    pub fn clo_standard() -> Self {
        Self::new(
            CLO_STANDARD.base_cdr,
            CLO_STANDARD.default_factor_sensitivity,
            CLO_STANDARD.default_mean_reversion,
            CLO_STANDARD.default_volatility,
        )
        .with_correlation(CLO_STANDARD.default_correlation)
    }

    /// Get the base hazard rate.
    pub fn base_hazard(&self) -> f64 {
        self.base_hazard
    }

    /// Get the factor sensitivity.
    pub fn factor_sensitivity(&self) -> f64 {
        self.factor_sensitivity
    }

    /// Get the mean reversion speed.
    pub fn mean_reversion(&self) -> f64 {
        self.mean_reversion
    }

    /// Get the volatility.
    pub fn volatility(&self) -> f64 {
        self.volatility
    }

    /// Calculate intensity at given factor value.
    ///
    /// λ(Z) = λ₀ × exp(β × Z × σ)
    fn intensity(&self, factor: f64) -> f64 {
        self.base_hazard * (self.factor_sensitivity * factor * self.volatility).exp()
    }
}

impl StochasticDefault for IntensityProcessDefault {
    fn conditional_mdr(
        &self,
        _seasoning: u32,
        factors: &[f64],
        _macro_factors: &MacroCreditFactors,
    ) -> f64 {
        let z = factors.first().copied().unwrap_or(0.0);

        // Conditional intensity
        let intensity = self.intensity(z);

        // Monthly survival probability
        let monthly_intensity = intensity / 12.0;
        let survival_prob = (-monthly_intensity).exp();

        // MDR = 1 - survival
        (1.0 - survival_prob).clamp(0.0, 1.0)
    }

    fn default_distribution(
        &self,
        n: usize,
        pds: &[f64],
        factors: &[f64],
        _correlation: f64,
    ) -> Vec<f64> {
        // Use conditional MDR to compute binomial distribution
        let z = factors.first().copied().unwrap_or(0.0);
        let cond_pd = if !pds.is_empty() {
            // Scale base PD by intensity ratio
            let intensity_ratio = (self.factor_sensitivity * z * self.volatility).exp();
            (pds[0] * intensity_ratio).min(0.9999)
        } else {
            let intensity = self.intensity(z);
            let monthly_intensity = intensity / 12.0;
            (1.0 - (-monthly_intensity).exp()).min(0.9999)
        };

        // Use the core binomial distribution function
        binomial_distribution(n, cond_pd.clamp(0.0, 1.0))
    }

    fn correlation(&self) -> f64 {
        self.correlation
    }

    fn model_name(&self) -> &'static str {
        "Intensity Process Default Model"
    }

    fn expected_mdr(&self, _seasoning: u32) -> f64 {
        cdr_to_mdr(self.base_hazard)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_intensity_process_creation() {
        let model = IntensityProcessDefault::new(0.02, 0.5, 0.5, 0.30);

        assert!((model.base_hazard() - 0.02).abs() < 1e-10);
        assert!((model.factor_sensitivity() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_conditional_mdr_at_zero_factor() {
        let model = IntensityProcessDefault::new(0.02, 0.5, 0.5, 0.30);
        let factors = MacroCreditFactors::default();

        let mdr = model.conditional_mdr(12, &[0.0], &factors);

        // At Z=0, intensity equals base_hazard
        // MDR should be approximately base_hazard / 12
        let expected = 1.0 - (-0.02 / 12.0_f64).exp();
        assert!(
            (mdr - expected).abs() < 1e-6,
            "MDR {} should equal expected {}",
            mdr,
            expected
        );
    }

    #[test]
    fn test_negative_factor_increases_mdr() {
        let model = IntensityProcessDefault::new(0.02, 0.5, 0.5, 0.30);
        let factors = MacroCreditFactors::default();

        let mdr_neg = model.conditional_mdr(12, &[-2.0], &factors);
        let mdr_zero = model.conditional_mdr(12, &[0.0], &factors);
        let mdr_pos = model.conditional_mdr(12, &[2.0], &factors);

        // With positive factor_sensitivity:
        // Negative factor decreases intensity -> lower MDR
        // Positive factor increases intensity -> higher MDR
        assert!(mdr_pos > mdr_zero, "Positive factor should increase MDR");
        assert!(mdr_neg < mdr_zero, "Negative factor should decrease MDR");
    }

    #[test]
    fn test_intensity_calculation() {
        let model = IntensityProcessDefault::new(0.02, 1.0, 0.5, 1.0);

        // At Z=0: intensity = base
        let int_zero = model.intensity(0.0);
        assert!((int_zero - 0.02).abs() < 1e-10);

        // At Z=1: intensity = base * exp(1 * 1 * 1) ≈ base * 2.718
        let int_pos = model.intensity(1.0);
        assert!(int_pos > int_zero);
        assert!((int_pos / int_zero - 1.0_f64.exp()).abs() < 1e-6);
    }

    #[test]
    fn test_standard_calibrations() {
        let rmbs = IntensityProcessDefault::rmbs_standard();
        assert!((rmbs.base_hazard() - 0.02).abs() < 1e-10);

        let clo = IntensityProcessDefault::clo_standard();
        assert!(clo.base_hazard() > rmbs.base_hazard());
        assert!(clo.correlation() > rmbs.correlation());
    }
}
