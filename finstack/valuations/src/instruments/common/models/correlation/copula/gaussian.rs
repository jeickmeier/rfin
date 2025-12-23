//! One-factor Gaussian copula for credit portfolio correlation.
//!
//! The standard market model for credit derivative correlation modeling.
//! Assumes a single systematic factor drives all defaults.
//!
//! # Mathematical Model
//!
//! Latent variable for entity i:
//! ```text
//! Aᵢ = √ρ · Z + √(1-ρ) · εᵢ
//! ```
//!
//! where:
//! - ρ is the asset correlation
//! - Z ~ N(0,1) is the common (systematic) factor
//! - εᵢ ~ N(0,1) is the idiosyncratic factor for entity i
//!
//! Default occurs when: Aᵢ ≤ Φ⁻¹(PD)
//!
//! # Conditional Default Probability
//!
//! Given the systematic factor Z:
//! ```text
//! P(default | Z) = Φ((Φ⁻¹(PD) - √ρ · Z) / √(1-ρ))
//! ```
//!
//! # Limitations
//!
//! - Zero tail dependence: joint extreme events are underestimated
//! - Correlation "smile" required via base correlation framework
//! - Static correlation doesn't capture stress dynamics
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//!   *Journal of Fixed Income*, 9(4), 43-54.

use super::{select_quadrature, Copula, DEFAULT_QUADRATURE_ORDER};
use finstack_core::math::{norm_cdf, GaussHermiteQuadrature};

/// Minimum correlation for numerical stability.
const MIN_CORRELATION: f64 = 0.01;
/// Maximum correlation for numerical stability.
const MAX_CORRELATION: f64 = 0.99;
/// CDF argument clipping to prevent overflow.
const CDF_CLIP: f64 = 10.0;

/// One-factor Gaussian copula model.
///
/// The industry-standard model for credit index tranche pricing,
/// used with base correlation to capture the correlation smile.
///
/// # Numerical Stability
///
/// - Correlation is clamped to [0.01, 0.99] to avoid numerical issues
/// - CDF arguments are clipped to [-10, 10] to prevent overflow
/// - Quadrature is cached for performance
pub struct GaussianCopula {
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Cached quadrature for performance
    quadrature: GaussHermiteQuadrature,
}

impl Clone for GaussianCopula {
    fn clone(&self) -> Self {
        Self {
            quadrature_order: self.quadrature_order,
            quadrature: select_quadrature(self.quadrature_order),
        }
    }
}

impl std::fmt::Debug for GaussianCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GaussianCopula")
            .field("quadrature_order", &self.quadrature_order)
            .finish()
    }
}

impl Default for GaussianCopula {
    fn default() -> Self {
        Self::new()
    }
}

impl GaussianCopula {
    /// Create a new Gaussian copula with default parameters.
    ///
    /// Uses 7-point Gauss-Hermite quadrature for integration.
    #[must_use]
    pub fn new() -> Self {
        let order = DEFAULT_QUADRATURE_ORDER;
        Self {
            quadrature_order: order,
            quadrature: select_quadrature(order),
        }
    }

    /// Create with custom quadrature order for higher precision.
    ///
    /// # Arguments
    /// * `order` - Quadrature order (5, 7, or 10). Higher order = more accuracy.
    #[must_use]
    pub fn with_quadrature_order(order: u8) -> Self {
        Self {
            quadrature_order: order,
            quadrature: select_quadrature(order),
        }
    }

    /// Smooth correlation boundary to avoid numerical discontinuities.
    ///
    /// Clamps correlation to [0.01, 0.99].
    fn smooth_correlation(&self, correlation: f64) -> f64 {
        correlation.clamp(MIN_CORRELATION, MAX_CORRELATION)
    }
}

impl Copula for GaussianCopula {
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        let z = factor_realization.first().copied().unwrap_or(0.0);
        let rho = self.smooth_correlation(correlation);

        // Handle extreme correlation cases
        if rho < 1e-10 {
            // Independent case: conditional = unconditional
            return norm_cdf(default_threshold);
        }
        if rho > 1.0 - 1e-10 {
            // Perfect correlation: deterministic given Z
            let threshold_adj = default_threshold - z;
            return norm_cdf(threshold_adj);
        }

        let sqrt_rho = rho.sqrt();
        let sqrt_1mr = (1.0 - rho).sqrt();

        // P(default | Z) = Φ((Φ⁻¹(PD) - √ρ·Z) / √(1-ρ))
        let conditional_threshold = (default_threshold - sqrt_rho * z) / sqrt_1mr;

        // Clip to prevent CDF overflow
        let clipped = conditional_threshold.clamp(-CDF_CLIP, CDF_CLIP);
        norm_cdf(clipped)
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // Gauss-Hermite quadrature over standard normal factor Z
        // Uses cached quadrature for performance
        self.quadrature.integrate(|z| f(&[z]))
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn model_name(&self) -> &'static str {
        "One-Factor Gaussian Copula"
    }

    fn tail_dependence(&self, _correlation: f64) -> f64 {
        // Gaussian copula has zero tail dependence
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::math::standard_normal_inv_cdf;

    #[test]
    fn test_gaussian_copula_creation() {
        let copula = GaussianCopula::new();
        assert_eq!(copula.num_factors(), 1);
        assert_eq!(copula.model_name(), "One-Factor Gaussian Copula");
    }

    #[test]
    fn test_conditional_default_prob_at_zero_factor() {
        let copula = GaussianCopula::new();
        let pd = 0.05; // 5% default probability
        let threshold = standard_normal_inv_cdf(pd);
        let correlation = 0.30;

        // At Z=0, the conditional probability is Φ(Φ⁻¹(PD) / √(1-ρ))
        let cond_prob = copula.conditional_default_prob(threshold, &[0.0], correlation);

        // Should be a valid probability between 0 and 1
        assert!(cond_prob > 0.0 && cond_prob < 1.0);
        // At Z=0 with positive correlation, conditional should differ from unconditional
        assert!(
            cond_prob < pd,
            "At Z=0 with correlation, conditional should differ from unconditional"
        );
    }

    #[test]
    fn test_conditional_default_prob_increases_with_negative_z() {
        let copula = GaussianCopula::new();
        let threshold = standard_normal_inv_cdf(0.05);
        let correlation = 0.30;

        let prob_z_neg = copula.conditional_default_prob(threshold, &[-2.0], correlation);
        let prob_z_zero = copula.conditional_default_prob(threshold, &[0.0], correlation);
        let prob_z_pos = copula.conditional_default_prob(threshold, &[2.0], correlation);

        // Negative Z (bad market) should increase default probability
        assert!(prob_z_neg > prob_z_zero);
        // Positive Z (good market) should decrease default probability
        assert!(prob_z_pos < prob_z_zero);
    }

    #[test]
    fn test_integration_recovers_unconditional() {
        let copula = GaussianCopula::new();
        let pd = 0.05;
        let threshold = standard_normal_inv_cdf(pd);
        let correlation = 0.30;

        // E[P(default|Z)] should equal P(default)
        let integrated_prob =
            copula.integrate_fn(&|z| copula.conditional_default_prob(threshold, z, correlation));

        assert!(
            (integrated_prob - pd).abs() < 0.001,
            "Integrated probability {} should equal unconditional {}",
            integrated_prob,
            pd
        );
    }

    #[test]
    fn test_zero_tail_dependence() {
        let copula = GaussianCopula::new();
        assert_eq!(copula.tail_dependence(0.5), 0.0);
    }

    #[test]
    fn test_extreme_correlation_handling() {
        let copula = GaussianCopula::new();
        let threshold = standard_normal_inv_cdf(0.05);

        // Very low correlation should give near-unconditional probability
        let prob_low = copula.conditional_default_prob(threshold, &[0.0], 0.001);
        assert!((prob_low - 0.05).abs() < 0.001);

        // Very high correlation should be sensitive to Z
        let prob_high_neg_z = copula.conditional_default_prob(threshold, &[-2.0], 0.99);
        let prob_high_pos_z = copula.conditional_default_prob(threshold, &[2.0], 0.99);
        assert!(prob_high_neg_z > 0.5); // Should be very high
        assert!(prob_high_pos_z < 0.01); // Should be very low
    }
}
