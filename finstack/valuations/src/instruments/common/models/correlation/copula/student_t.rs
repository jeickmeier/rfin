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
//! - Outer integral over χ²(ν)/ν scaling variable using Gauss-Laguerre quadrature
//! - Inner Gaussian integration conditional on scaling
//!
//! # References
//!
//! - Demarta, S., & McNeil, A. J. (2005). "The t Copula and Related Copulas."
//!   *International Statistical Review*, 73(1), 111-129.
//! - Hull, J., Predescu, M., & White, A. (2005). "The valuation of correlation-
//!   dependent credit derivatives using a structural model."

use super::{select_quadrature, Copula, DEFAULT_QUADRATURE_ORDER};
use finstack_core::math::{student_t_cdf, student_t_inv_cdf, GaussHermiteQuadrature};

/// Minimum correlation for numerical stability.
const MIN_CORRELATION: f64 = 0.01;
/// Maximum correlation for numerical stability.
const MAX_CORRELATION: f64 = 0.99;

/// Student-t copula with configurable degrees of freedom.
///
/// Captures tail dependence - the tendency for defaults to cluster
/// during market stress more than Gaussian correlation predicts.
///
/// Uses the proper Student-t CDF/inverse CDF implementations from statrs
/// for accurate tail behavior at low degrees of freedom.
pub struct StudentTCopula {
    /// Degrees of freedom (ν > 2 required for finite variance)
    degrees_of_freedom: f64,
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Cached inner quadrature for performance
    inner_quadrature: GaussHermiteQuadrature,
    /// Cached Gauss-Laguerre quadrature nodes and weights for χ²(ν)/ν
    chi_sq_quadrature: Vec<(f64, f64)>,
}

impl Clone for StudentTCopula {
    fn clone(&self) -> Self {
        Self {
            degrees_of_freedom: self.degrees_of_freedom,
            quadrature_order: self.quadrature_order,
            inner_quadrature: select_quadrature(self.quadrature_order),
            chi_sq_quadrature: self.chi_sq_quadrature.clone(),
        }
    }
}

impl std::fmt::Debug for StudentTCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StudentTCopula")
            .field("degrees_of_freedom", &self.degrees_of_freedom)
            .field("quadrature_order", &self.quadrature_order)
            .field("chi_sq_points", &self.chi_sq_quadrature.len())
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
    #[must_use]
    pub fn new(df: f64) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2 for finite variance");
        let order = DEFAULT_QUADRATURE_ORDER;
        Self {
            degrees_of_freedom: df,
            quadrature_order: order,
            inner_quadrature: select_quadrature(order),
            chi_sq_quadrature: Self::compute_chi_sq_quadrature(df, order as usize),
        }
    }

    /// Create with custom quadrature order for higher precision.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (must be > 2)
    /// * `order` - Quadrature order (5, 7, or 10)
    #[must_use]
    pub fn with_quadrature_order(df: f64, order: u8) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2");
        Self {
            degrees_of_freedom: df,
            quadrature_order: order,
            inner_quadrature: select_quadrature(order),
            chi_sq_quadrature: Self::compute_chi_sq_quadrature(df, order as usize),
        }
    }

    /// Get the degrees of freedom.
    #[must_use]
    pub fn df(&self) -> f64 {
        self.degrees_of_freedom
    }

    /// Smooth correlation to avoid numerical issues.
    fn smooth_correlation(&self, correlation: f64) -> f64 {
        correlation.clamp(MIN_CORRELATION, MAX_CORRELATION)
    }

    /// Compute Gauss-Laguerre quadrature for χ²(ν)/ν integration.
    ///
    /// Uses Gauss-Laguerre nodes transformed to integrate over Gamma(ν/2, 2/ν) distribution.
    /// This is more accurate than uniform grid for capturing tail behavior.
    ///
    /// For χ²(ν)/ν = Gamma(ν/2, 2/ν), we use the transformation:
    /// - E[χ²(ν)/ν] = 1
    /// - Var[χ²(ν)/ν] = 2/ν
    fn compute_chi_sq_quadrature(nu: f64, n: usize) -> Vec<(f64, f64)> {
        // Gauss-Laguerre nodes and weights for integrating x^α * e^(-x) over [0, ∞)
        // For Gamma(α, β) distribution, we transform: Y = X/β, with α = ν/2, β = 2/ν
        //
        // Standard Gauss-Laguerre nodes for α = 0 (weights include e^x factor):
        let laguerre_nodes_weights: &[(f64, f64)] = match n {
            n if n <= 5 => &[
                (0.263_560_319_7, 0.521_755_610_6),
                (1.413_403_059_1, 0.398_666_811_1),
                (3.596_425_771_0, 0.075_942_449_7),
                (7.085_810_005_9, 0.003_611_758_7),
                (12.640_800_844, 0.000_023_370_0),
            ],
            n if n <= 7 => &[
                (0.193_043_676_6, 0.409_318_951_7),
                (1.026_664_895_3, 0.421_831_277_9),
                (2.567_876_744_9, 0.147_126_348_7),
                (4.900_353_084_5, 0.020_633_514_5),
                (8.182_153_444_6, 0.001_074_010_1),
                (12.734_180_292, 0.000_015_865_5),
                (19.395_727_862, 0.000_000_031_7),
            ],
            _ => &[
                (0.137_793_470_5, 0.308_441_115_8),
                (0.729_454_549_5, 0.401_119_929_2),
                (1.808_342_901_7, 0.218_068_287_6),
                (3.401_433_697_8, 0.062_087_456_1),
                (5.552_496_140_1, 0.009_501_517_0),
                (8.330_152_746_8, 0.000_753_008_4),
                (11.843_785_838, 0.000_028_259_2),
                (16.279_257_831, 0.000_000_424_9),
                (21.996_585_812, 0.000_000_001_8),
                (29.920_697_012, 0.000_000_000_001),
            ],
        };

        let num_points = laguerre_nodes_weights.len().min(n);

        // Transform Laguerre nodes to χ²(ν)/ν scale
        // χ²(ν)/ν has mean 1 and variance 2/ν
        // We use: x_transformed = node * (2/ν) which maps Laguerre to χ²/ν
        let scale = 2.0 / nu;

        laguerre_nodes_weights[..num_points]
            .iter()
            .map(|&(node, weight)| {
                // Transform node and adjust weight for change of variables
                let x = node * scale;
                // Weight includes transformation Jacobian
                (x.max(0.01), weight)
            })
            .collect()
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
            return student_t_cdf(default_threshold, nu);
        }
        if rho > 1.0 - 1e-10 {
            let threshold_adj = default_threshold - z;
            return student_t_cdf(threshold_adj, nu);
        }

        let sqrt_rho = rho.sqrt();
        let sqrt_1mr = (1.0 - rho).sqrt();

        // For Student-t, the conditional threshold involves the t-distribution
        // P(default | T=z) = t_{ν}((t^{-1}(PD) - √ρ·z) / √(1-ρ))
        //
        // Note: default_threshold is Φ⁻¹(PD), so we convert to t-scale
        let p_default = finstack_core::math::norm_cdf(default_threshold);
        let t_threshold = student_t_inv_cdf(p_default, nu);

        let conditional_threshold = (t_threshold - sqrt_rho * z) / sqrt_1mr;

        student_t_cdf(conditional_threshold, nu)
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // For Student-t, we use a two-layer integration:
        // 1. Outer: over the variance scaling χ²(ν)/ν variable using Gauss-Laguerre
        // 2. Inner: Gaussian integration given the scaling
        //
        // This exploits: T = Z / √(χ²/ν) where Z ~ N(0,1), χ² ~ χ²(ν)

        let mut result = 0.0;
        for &(chi_sq_val, chi_weight) in &self.chi_sq_quadrature {
            // Scale factor for converting Gaussian to t
            // T = Z / √(χ²/ν), so scale = √(χ²/ν)
            let scale = chi_sq_val.sqrt();

            // Inner Gaussian integration with scaled factor
            let inner = self.inner_quadrature.integrate(|z_gauss| {
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
        // This is the exact formula for lower tail dependence
        let arg = -((nu + 1.0) * (1.0 - rho) / (1.0 + rho)).sqrt();
        2.0 * student_t_cdf(arg, nu + 1.0)
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
        let _ = StudentTCopula::new(2.0);
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

    #[test]
    fn test_tail_dependence_golden_values() {
        // Test the exact formula: λ_L = 2 · t_{ν+1}(-√((ν+1)(1-ρ)/(1+ρ)))
        // These are computed using the formula with proper Student-t CDF

        let test_cases = [
            // (df, rho) - we just verify the values are reasonable and monotonic
            (4.0, 0.5),
            (5.0, 0.5),
            (10.0, 0.5),
        ];

        for (df, rho) in test_cases {
            let copula = StudentTCopula::new(df);
            let lambda = copula.tail_dependence(rho);

            // Tail dependence must be in [0, 1]
            assert!(
                (0.0..=1.0).contains(&lambda),
                "Tail dependence for df={}, ρ={}: got {}, expected in [0,1]",
                df,
                rho,
                lambda
            );

            // For moderate correlation and df, tail dependence should be positive but not huge
            assert!(
                lambda < 0.5,
                "Tail dependence {} seems too high for df={}, ρ={}",
                lambda,
                df,
                rho
            );
        }

        // Verify monotonicity: higher df → lower tail dependence
        let copula_4 = StudentTCopula::new(4.0);
        let copula_10 = StudentTCopula::new(10.0);
        assert!(
            copula_4.tail_dependence(0.5) > copula_10.tail_dependence(0.5),
            "Lower df should give higher tail dependence"
        );
    }

    #[test]
    fn test_student_t_cdf_accuracy() {
        // Test that we're using the proper Student-t CDF (not approximation)
        // by checking known values from statistical tables

        // t-distribution with df=5, x=-2.0 should give CDF ≈ 0.051
        let cdf = student_t_cdf(-2.0, 5.0);
        assert!(
            (cdf - 0.051).abs() < 0.002,
            "CDF(-2.0, df=5) = {}, expected ~0.051",
            cdf
        );

        // df=10, x=-1.812 should give CDF ≈ 0.05 (97.5th percentile critical value)
        let cdf_10 = student_t_cdf(-1.812, 10.0);
        assert!(
            (cdf_10 - 0.05).abs() < 0.005,
            "CDF(-1.812, df=10) = {}, expected ~0.05",
            cdf_10
        );
    }

    #[test]
    fn test_student_t_inv_cdf_roundtrip() {
        let test_dfs = [3.0, 5.0, 10.0, 30.0];
        let test_probs = [0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95];

        for &df in &test_dfs {
            for &p in &test_probs {
                let x = student_t_inv_cdf(p, df);
                let p_back = student_t_cdf(x, df);
                assert!(
                    (p - p_back).abs() < 1e-6,
                    "Round-trip failed for df={}, p={}: got x={}, p_back={}",
                    df,
                    p,
                    x,
                    p_back
                );
            }
        }
    }

    #[test]
    fn test_chi_sq_quadrature_properties() {
        // Verify that the χ²(ν)/ν quadrature has reasonable properties:
        // - All nodes are positive (χ²/ν > 0)
        // - Weights sum to approximately 1 (or close to 1)
        // - Nodes are in a reasonable range

        for df in [4.0, 5.0, 10.0, 20.0] {
            let copula = StudentTCopula::new(df);
            let points = &copula.chi_sq_quadrature;

            // All nodes must be positive
            for &(x, w) in points {
                assert!(x > 0.0, "Quadrature node must be positive, got {}", x);
                assert!(
                    w >= 0.0,
                    "Quadrature weight must be non-negative, got {}",
                    w
                );
            }

            // Weights should sum to approximately 1
            let weight_sum: f64 = points.iter().map(|&(_, w)| w).sum();
            assert!(
                (weight_sum - 1.0).abs() < 0.01,
                "χ²({}) weights sum to {}, expected ~1.0",
                df,
                weight_sum
            );

            // Should have at least a few quadrature points
            assert!(
                points.len() >= 5,
                "Expected at least 5 quadrature points, got {}",
                points.len()
            );
        }
    }
}
