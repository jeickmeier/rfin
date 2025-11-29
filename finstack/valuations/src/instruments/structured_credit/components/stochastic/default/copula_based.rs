//! Copula-based default model.
//!
//! Implements default correlation using the Li (2000) copula framework,
//! leveraging the shared copula infrastructure.
//!
//! # Mathematical Model
//!
//! For each obligor i:
//! ```text
//! Aᵢ = √ρ · Z + √(1-ρ) · εᵢ
//! Default: Aᵢ ≤ Φ⁻¹(PD)
//! ```
//!
//! The conditional default probability given Z:
//! ```text
//! P(default | Z) = Φ((Φ⁻¹(PD) - √ρ · Z) / √(1-ρ))
//! ```
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."

use super::traits::{MacroCreditFactors, StochasticDefault};
use crate::instruments::common::models::correlation::copula::{Copula, CopulaSpec, GaussianCopula};
use crate::instruments::structured_credit::components::rates::cdr_to_mdr;
use finstack_core::math::standard_normal_inv_cdf;

/// Copula-based stochastic default model.
///
/// Uses the shared copula infrastructure for default correlation modeling.
#[derive(Clone, Debug)]
pub struct CopulaBasedDefault {
    /// Base annual CDR
    base_cdr: f64,
    /// Copula specification
    copula_spec: CopulaSpec,
    /// Asset correlation
    correlation: f64,
    /// Copula instance
    copula: GaussianCopula,
}

impl CopulaBasedDefault {
    /// Create a copula-based default model.
    ///
    /// # Arguments
    /// * `base_cdr` - Base annual CDR (unconditional)
    /// * `copula_spec` - Copula model specification
    /// * `correlation` - Asset correlation
    pub fn new(base_cdr: f64, copula_spec: CopulaSpec, correlation: f64) -> Self {
        let copula = match &copula_spec {
            CopulaSpec::Gaussian => GaussianCopula::new(),
            // For other copulas, use Gaussian as fallback for now
            // Full implementation would dispatch to appropriate copula
            _ => GaussianCopula::new(),
        };

        Self {
            base_cdr: base_cdr.clamp(0.0, 1.0),
            copula_spec,
            correlation: correlation.clamp(0.0, 0.99),
            copula,
        }
    }

    /// Create with Gaussian copula and specified correlation.
    pub fn gaussian(base_cdr: f64, correlation: f64) -> Self {
        Self::new(base_cdr, CopulaSpec::Gaussian, correlation)
    }

    /// Standard RMBS calibration.
    ///
    /// - Base CDR: 2%
    /// - Correlation: 5% (low for diversified pools)
    pub fn rmbs_standard() -> Self {
        Self::gaussian(0.02, 0.05)
    }

    /// Standard CLO calibration.
    ///
    /// - Base CDR: 3%
    /// - Correlation: 20% (higher for corporate loans)
    pub fn clo_standard() -> Self {
        Self::gaussian(0.03, 0.20)
    }

    /// Get the base CDR.
    pub fn base_cdr(&self) -> f64 {
        self.base_cdr
    }

    /// Get the copula specification.
    pub fn copula_spec(&self) -> &CopulaSpec {
        &self.copula_spec
    }
}

impl StochasticDefault for CopulaBasedDefault {
    fn conditional_mdr(
        &self,
        _seasoning: u32,
        factors: &[f64],
        _macro_factors: &MacroCreditFactors,
    ) -> f64 {
        // Get the base monthly default rate
        let base_mdr = cdr_to_mdr(self.base_cdr);

        // Convert to default threshold
        let threshold = standard_normal_inv_cdf(base_mdr.min(0.9999));

        // Get conditional probability using copula
        let cond_prob = self
            .copula
            .conditional_default_prob(threshold, factors, self.correlation);

        cond_prob.clamp(0.0, 1.0)
    }

    fn default_distribution(
        &self,
        n: usize,
        pds: &[f64],
        factors: &[f64],
        correlation: f64,
    ) -> Vec<f64> {
        // For homogeneous pool, use binomial-like distribution
        // with conditional default probability

        let pd = pds.first().copied().unwrap_or(self.base_cdr);
        let threshold = standard_normal_inv_cdf(pd.min(0.9999));

        let cond_pd = self
            .copula
            .conditional_default_prob(threshold, factors, correlation);

        // Simple binomial distribution (could use recursive formula for efficiency)
        let mut dist = vec![0.0; n + 1];

        // P(k) = C(n,k) * p^k * (1-p)^(n-k)
        // Use log to avoid overflow for large n
        let log_p = cond_pd.max(1e-10).ln();
        let log_1mp = (1.0 - cond_pd).max(1e-10).ln();

        let mut log_coeff = 0.0; // log(C(n,k))
        for (k, prob) in dist.iter_mut().enumerate() {
            *prob = (log_coeff + k as f64 * log_p + (n - k) as f64 * log_1mp).exp();

            // Update log(C(n,k)) -> log(C(n,k+1))
            if k < n {
                log_coeff += ((n - k) as f64).ln() - ((k + 1) as f64).ln();
            }
        }

        // Normalize
        let sum: f64 = dist.iter().sum();
        if sum > 0.0 {
            for p in &mut dist {
                *p /= sum;
            }
        }

        dist
    }

    fn correlation(&self) -> f64 {
        self.correlation
    }

    fn model_name(&self) -> &'static str {
        "Copula-Based Default Model"
    }

    fn expected_mdr(&self, _seasoning: u32) -> f64 {
        cdr_to_mdr(self.base_cdr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copula_based_creation() {
        let model = CopulaBasedDefault::gaussian(0.02, 0.20);

        assert!((model.base_cdr() - 0.02).abs() < 1e-10);
        assert!((model.correlation() - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_conditional_mdr_at_zero_factor() {
        let model = CopulaBasedDefault::gaussian(0.02, 0.20);
        let factors = MacroCreditFactors::default();

        let mdr = model.conditional_mdr(12, &[0.0], &factors);
        let expected = model.expected_mdr(12);

        // At Z=0 with correlation, conditional differs from unconditional
        // The relationship depends on how correlation affects the copula formula
        assert!(mdr > 0.0 && mdr < 1.0, "MDR {} should be in (0, 1)", mdr);
        // Both should be small values (around 0.17% monthly for 2% annual)
        assert!(
            expected > 0.0 && expected < 0.01,
            "Expected MDR {} should be small",
            expected
        );
    }

    #[test]
    fn test_negative_factor_increases_mdr() {
        let model = CopulaBasedDefault::gaussian(0.02, 0.30);
        let factors = MacroCreditFactors::default();

        let mdr_neg = model.conditional_mdr(12, &[-2.0], &factors);
        let mdr_zero = model.conditional_mdr(12, &[0.0], &factors);
        let mdr_pos = model.conditional_mdr(12, &[2.0], &factors);

        // Negative factor (stress) should increase defaults
        assert!(mdr_neg > mdr_zero, "Negative factor should increase MDR");
        assert!(mdr_pos < mdr_zero, "Positive factor should decrease MDR");
    }

    #[test]
    fn test_default_distribution_sums_to_one() {
        let model = CopulaBasedDefault::gaussian(0.05, 0.20);

        let dist = model.default_distribution(10, &[0.05], &[0.0], 0.20);

        let sum: f64 = dist.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_standard_calibrations() {
        let rmbs = CopulaBasedDefault::rmbs_standard();
        assert!((rmbs.base_cdr() - 0.02).abs() < 1e-10);
        assert!((rmbs.correlation() - 0.05).abs() < 1e-10);

        let clo = CopulaBasedDefault::clo_standard();
        assert!((clo.base_cdr() - 0.03).abs() < 1e-10);
        assert!(clo.correlation() > rmbs.correlation());
    }
}
