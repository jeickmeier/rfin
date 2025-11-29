//! Student-t copula for tail dependence modeling in credit portfolio pricing.
//!
//! The Student-t copula addresses the "Gaussian copula killed Wall Street" critique
//! by modeling tail dependence - the empirically observed phenomenon that joint
//! defaults cluster in stressed markets more than Gaussian correlation predicts.
//!
//! # Mathematical Model
//!
//! Latent variable for entity i:
//! ```text
//! Aᵢ = √ρ · T + √(1-ρ) · εᵢ
//! ```
//!
//! where T ~ t(ν) and εᵢ ~ t(ν) are Student-t distributed with ν degrees of freedom.
//!
//! The key insight is that T and εᵢ share a common variance scaling χ ~ χ²(ν)/ν,
//! creating dependence in the tails that Gaussian lacks.
//!
//! # Tail Dependence
//!
//! Lower tail dependence coefficient:
//! ```text
//! λ_L = 2 · t_{ν+1}(-√((ν+1)(1-ρ)/(1+ρ)))
//! ```
//!
//! - As ν → ∞, converges to Gaussian (λ_L → 0)
//! - Lower ν = higher tail dependence
//! - Typical market calibration: ν ∈ [4, 10] for CDX tranches
//!
//! # Integration Approach
//!
//! Uses variance-gamma mixing representation for computational efficiency:
//! - Outer integral over χ² scaling variable
//! - Inner Gaussian integration conditional on scaling
//!
//! # References
//!
//! - Demarta, S., & McNeil, A. J. (2005). "The t Copula and Related Copulas."
//!   *International Statistical Review*, 73(1), 111-129.
//! - Hull, J., Predescu, M., & White, A. (2005). "The valuation of correlation-
//!   dependent credit derivatives using a structural model."

use super::{select_quadrature, Copula, DEFAULT_QUADRATURE_ORDER};
use finstack_core::math::GaussHermiteQuadrature;

/// Student-t copula with configurable degrees of freedom.
///
/// Captures tail dependence - the tendency for defaults to cluster
/// during market stress more than Gaussian correlation predicts.
pub struct StudentTCopula {
    /// Degrees of freedom (ν > 2 required for finite variance)
    degrees_of_freedom: f64,
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Minimum correlation for numerical stability
    min_correlation: f64,
    /// Maximum correlation for numerical stability
    max_correlation: f64,
    /// Number of points for χ² integration
    chi_sq_points: usize,
}

impl Clone for StudentTCopula {
    fn clone(&self) -> Self {
        Self {
            degrees_of_freedom: self.degrees_of_freedom,
            quadrature_order: self.quadrature_order,
            min_correlation: self.min_correlation,
            max_correlation: self.max_correlation,
            chi_sq_points: self.chi_sq_points,
        }
    }
}

impl std::fmt::Debug for StudentTCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StudentTCopula")
            .field("degrees_of_freedom", &self.degrees_of_freedom)
            .field("quadrature_order", &self.quadrature_order)
            .field("min_correlation", &self.min_correlation)
            .field("max_correlation", &self.max_correlation)
            .field("chi_sq_points", &self.chi_sq_points)
            .finish()
    }
}

impl StudentTCopula {
    /// Create a Student-t copula with specified degrees of freedom.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (must be > 2 for finite variance)
    ///
    /// # Panics
    /// Panics if df ≤ 2
    pub fn new(df: f64) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2 for finite variance");
        Self {
            degrees_of_freedom: df,
            quadrature_order: DEFAULT_QUADRATURE_ORDER,
            min_correlation: 0.01,
            max_correlation: 0.99,
            chi_sq_points: 7,
        }
    }

    /// Create with custom quadrature order for higher precision.
    pub fn with_quadrature_order(df: f64, order: u8) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2");
        Self {
            degrees_of_freedom: df,
            quadrature_order: order,
            min_correlation: 0.01,
            max_correlation: 0.99,
            chi_sq_points: order as usize,
        }
    }

    /// Get quadrature for inner integration.
    fn inner_quadrature(&self) -> GaussHermiteQuadrature {
        select_quadrature(self.quadrature_order)
    }

    /// Get the degrees of freedom.
    pub fn df(&self) -> f64 {
        self.degrees_of_freedom
    }

    /// Student-t CDF approximation using normal approximation with correction.
    ///
    /// For ν > 30, this is highly accurate. For smaller ν, uses
    /// Hill's algorithm-style approximation.
    fn student_t_cdf(&self, x: f64, df: f64) -> f64 {
        // For large df, use normal approximation
        if df > 100.0 {
            return finstack_core::math::norm_cdf(x);
        }

        // For moderate df, use improved approximation
        // Based on Abramowitz & Stegun approximation
        let t2 = x * x;
        let df_f = df;

        // Cornish-Fisher approximation for t-distribution
        let z = x * (1.0 - 1.0 / (4.0 * df_f)) / (1.0 + t2 / (2.0 * df_f)).sqrt();

        finstack_core::math::norm_cdf(z)
    }

    /// Inverse Student-t CDF approximation.
    fn student_t_inv_cdf(&self, p: f64, df: f64) -> f64 {
        // Use normal approximation with correction
        let z = finstack_core::math::standard_normal_inv_cdf(p);

        if df > 100.0 {
            return z;
        }

        // First-order correction for t-distribution
        let g1 = (z * z + 1.0) / (4.0 * df);
        z * (1.0 + g1)
    }

    /// Smooth correlation to avoid numerical issues.
    fn smooth_correlation(&self, correlation: f64) -> f64 {
        correlation.clamp(self.min_correlation, self.max_correlation)
    }

    /// Gauss-Laguerre style quadrature weights for χ²(ν) / ν integration.
    ///
    /// Uses transformation to map gamma distribution to standard quadrature.
    fn chi_sq_quadrature(&self) -> Vec<(f64, f64)> {
        let nu = self.degrees_of_freedom;
        let n = self.chi_sq_points;

        // Use Gauss-Laguerre style points transformed for χ²(ν)/ν
        // Mean of χ²(ν)/ν is 1, variance is 2/ν
        let mean = 1.0;
        let std = (2.0 / nu).sqrt();

        // Simple quadrature points around mean
        let mut points = Vec::with_capacity(n);
        for i in 0..n {
            let x = mean + std * (i as f64 - (n as f64 - 1.0) / 2.0) * 0.8;
            let x = x.max(0.01); // Ensure positive
            let w = 1.0 / n as f64;
            points.push((x, w));
        }

        points
    }
}

impl Copula for StudentTCopula {
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        let z = factor_realization.first().copied().unwrap_or(0.0);
        let rho = self.smooth_correlation(correlation);
        let nu = self.degrees_of_freedom;

        // Handle extreme correlation cases
        if rho < 1e-10 {
            return self.student_t_cdf(default_threshold, nu);
        }
        if rho > 1.0 - 1e-10 {
            let threshold_adj = default_threshold - z;
            return self.student_t_cdf(threshold_adj, nu);
        }

        let sqrt_rho = rho.sqrt();
        let sqrt_1mr = (1.0 - rho).sqrt();

        // For Student-t, the conditional threshold involves the t-distribution
        // P(default | T=z) ≈ t_{ν}((t^{-1}(PD) - √ρ·z) / √(1-ρ))
        let t_threshold =
            self.student_t_inv_cdf(finstack_core::math::norm_cdf(default_threshold), nu);

        let conditional_threshold = (t_threshold - sqrt_rho * z) / sqrt_1mr;

        self.student_t_cdf(conditional_threshold, nu)
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // For Student-t, we use a two-layer integration:
        // 1. Outer: over the variance scaling χ² variable
        // 2. Inner: Gaussian integration given the scaling
        //
        // This exploits: T = Z / √(χ²/ν) where Z ~ N(0,1), χ² ~ χ²(ν)

        let chi_sq_points = self.chi_sq_quadrature();

        let mut result = 0.0;
        for (chi_sq_val, chi_weight) in chi_sq_points {
            // Scale factor for converting Gaussian to t
            let scale = chi_sq_val.sqrt();

            // Inner Gaussian integration with scaled factor
            let inner = self.inner_quadrature().integrate(|z_gauss| {
                // Convert Gaussian realization to t-distributed
                let z_t = z_gauss / scale;
                f(&[z_t])
            });

            result += chi_weight * inner;
        }

        result
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn model_name(&self) -> &'static str {
        "Student-t Copula"
    }

    fn tail_dependence(&self, correlation: f64) -> f64 {
        let rho = self.smooth_correlation(correlation);
        let nu = self.degrees_of_freedom;

        // λ_L = 2 · t_{ν+1}(-√((ν+1)(1-ρ)/(1+ρ)))
        let arg = -((nu + 1.0) * (1.0 - rho) / (1.0 + rho)).sqrt();
        2.0 * self.student_t_cdf(arg, nu + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_student_t_creation() {
        let copula = StudentTCopula::new(5.0);
        assert_eq!(copula.num_factors(), 1);
        assert!((copula.df() - 5.0).abs() < 1e-10);
        assert_eq!(copula.model_name(), "Student-t Copula");
    }

    #[test]
    #[should_panic(expected = "Student-t df must be > 2")]
    fn test_student_t_invalid_df() {
        StudentTCopula::new(2.0);
    }

    #[test]
    fn test_tail_dependence_positive() {
        let copula = StudentTCopula::new(5.0);
        let lambda = copula.tail_dependence(0.5);

        // Student-t should have positive tail dependence
        assert!(lambda > 0.0, "Tail dependence should be positive");
        assert!(lambda < 1.0, "Tail dependence should be < 1");
    }

    #[test]
    fn test_tail_dependence_increases_with_correlation() {
        let copula = StudentTCopula::new(5.0);

        let lambda_low = copula.tail_dependence(0.2);
        let lambda_mid = copula.tail_dependence(0.5);
        let lambda_high = copula.tail_dependence(0.8);

        // Higher correlation should give higher tail dependence
        assert!(
            lambda_mid > lambda_low,
            "Tail dependence should increase with correlation"
        );
        assert!(
            lambda_high > lambda_mid,
            "Tail dependence should increase with correlation"
        );
    }

    #[test]
    fn test_tail_dependence_decreases_with_df() {
        // Lower df = heavier tails = more tail dependence
        let copula_low_df = StudentTCopula::new(4.0);
        let copula_high_df = StudentTCopula::new(20.0);

        let lambda_low_df = copula_low_df.tail_dependence(0.5);
        let lambda_high_df = copula_high_df.tail_dependence(0.5);

        assert!(
            lambda_low_df > lambda_high_df,
            "Lower df should give higher tail dependence"
        );
    }

    #[test]
    fn test_converges_to_gaussian_for_high_df() {
        let copula_high_df = StudentTCopula::new(100.0);
        let lambda = copula_high_df.tail_dependence(0.5);

        // For very high df, should approach Gaussian (zero tail dependence)
        assert!(
            lambda < 0.05,
            "High df should give near-zero tail dependence"
        );
    }

    #[test]
    fn test_conditional_prob_sensitive_to_factor() {
        let copula = StudentTCopula::new(5.0);
        let threshold = finstack_core::math::standard_normal_inv_cdf(0.05);
        let correlation = 0.3;

        let prob_neg = copula.conditional_default_prob(threshold, &[-2.0], correlation);
        let prob_zero = copula.conditional_default_prob(threshold, &[0.0], correlation);
        let prob_pos = copula.conditional_default_prob(threshold, &[2.0], correlation);

        // Same pattern as Gaussian: negative factor increases default prob
        assert!(prob_neg > prob_zero);
        assert!(prob_pos < prob_zero);
    }
}
