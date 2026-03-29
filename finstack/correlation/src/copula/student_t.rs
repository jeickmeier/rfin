//! Student-t copula for tail dependence modeling in credit portfolio pricing.
//!
//! The Student-t copula addresses the "Gaussian copula killed Wall Street" critique
//! by modeling tail dependence - the empirically observed phenomenon that joint
//! defaults cluster in stressed markets more than Gaussian correlation predicts.
//!
//! # Mathematical Model (Standard Multivariate t-Copula)
//!
//! All entities share a common mixing variable W ~ Gamma(ν/2, ν/2):
//! ```text
//! M  = Z_M / √W     (systematic factor, t(ν)-distributed)
//! εᵢ = Zᵢ  / √W     (idiosyncratic, t(ν)-distributed, same W)
//! Aᵢ = √ρ · M + √(1-ρ) · εᵢ
//! ```
//!
//! The shared W creates tail dependence: when W is small (heavy-tail event),
//! ALL variables are simultaneously large in magnitude.
//!
//! # Conditional Default Probability
//!
//! Given the systematic factor M = m:
//! ```text
//! P(default | M=m) = t_{ν+1}( (c - √ρ·m) / √(1-ρ) · √((ν+1)/(ν + m²)) )
//! ```
//!
//! where c = t_ν⁻¹(PD) is the default threshold and the ν+1 degrees of freedom
//! arise from conditioning on M in the multivariate t-distribution.
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
//! Uses variance-gamma mixing representation:
//! - Outer integral over W ~ Gamma(ν/2, ν/2) using Gauss-Laguerre quadrature
//! - Inner Gaussian integration conditional on W
//! - Factor transformation: M = Z / √W converts Gaussian to t-distributed
//!
//! # References
//!
//! - Student-t copula theory:
//!   `docs/REFERENCES.md#demarta-mcneil-2005-t-copula`
//! - Correlation-dependent credit valuation:
//!   `docs/REFERENCES.md#hull-predescu-white-2005`

use super::{select_quadrature, Copula, DEFAULT_QUADRATURE_ORDER};
#[cfg(test)]
use finstack_core::math::student_t_inv_cdf;
use finstack_core::math::{ln_gamma, student_t_cdf, GaussHermiteQuadrature};

/// Minimum correlation for numerical stability.
const MIN_CORRELATION: f64 = 0.01;
/// Maximum correlation for numerical stability.
const MAX_CORRELATION: f64 = 0.99;

/// Student-t copula with configurable degrees of freedom.
///
/// Captures tail dependence - the tendency for defaults to cluster
/// during market stress more than Gaussian correlation predicts.
///
/// Implements the standard multivariate t-copula (shared mixing variable)
/// per Demarta & McNeil (2005), with proper ν+1 conditional degrees of freedom.
///
/// # References
///
/// - `docs/REFERENCES.md#demarta-mcneil-2005-t-copula`
/// - `docs/REFERENCES.md#hull-predescu-white-2005`
pub struct StudentTCopula {
    /// Degrees of freedom (ν > 2 required for finite variance)
    degrees_of_freedom: f64,
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Cached inner quadrature for Gaussian integration given W
    inner_quadrature: GaussHermiteQuadrature,
    /// Cached Gauss-Laguerre quadrature nodes and weights for Gamma(ν/2, ν/2)
    gamma_quadrature: Vec<(f64, f64)>,
}

impl Clone for StudentTCopula {
    fn clone(&self) -> Self {
        Self {
            degrees_of_freedom: self.degrees_of_freedom,
            quadrature_order: self.quadrature_order,
            inner_quadrature: select_quadrature(self.quadrature_order),
            gamma_quadrature: self.gamma_quadrature.clone(),
        }
    }
}

impl std::fmt::Debug for StudentTCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StudentTCopula")
            .field("degrees_of_freedom", &self.degrees_of_freedom)
            .field("quadrature_order", &self.quadrature_order)
            .field("gamma_points", &self.gamma_quadrature.len())
            .finish()
    }
}

impl StudentTCopula {
    /// Create a Student-t copula with specified degrees of freedom.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (must be > 2 for finite variance)
    ///
    /// # Returns
    ///
    /// A Student-t copula using the default quadrature order.
    ///
    /// # Panics
    /// Panics if df ≤ 2
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_correlation::{Copula, StudentTCopula};
    ///
    /// let copula = StudentTCopula::new(5.0);
    /// let lambda = copula.tail_dependence(0.50);
    ///
    /// assert!(lambda > 0.0);
    /// ```
    #[must_use]
    pub fn new(df: f64) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2 for finite variance");
        let order = DEFAULT_QUADRATURE_ORDER;
        Self {
            degrees_of_freedom: df,
            quadrature_order: order,
            inner_quadrature: select_quadrature(order),
            gamma_quadrature: Self::compute_gamma_quadrature(df, order as usize),
        }
    }

    /// Create with custom quadrature order for higher precision.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (must be > 2)
    /// * `order` - Requested quadrature order for the inner Gaussian integration
    ///
    /// # Returns
    ///
    /// A Student-t copula using the requested quadrature order.
    #[must_use]
    pub fn with_quadrature_order(df: f64, order: u8) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2");
        Self {
            degrees_of_freedom: df,
            quadrature_order: order,
            inner_quadrature: select_quadrature(order),
            gamma_quadrature: Self::compute_gamma_quadrature(df, order as usize),
        }
    }

    /// Get the degrees of freedom.
    ///
    /// # Returns
    ///
    /// The Student-t degrees of freedom used by this copula.
    #[must_use]
    pub fn df(&self) -> f64 {
        self.degrees_of_freedom
    }

    /// Smooth correlation to avoid numerical issues.
    fn smooth_correlation(&self, correlation: f64) -> f64 {
        correlation.clamp(MIN_CORRELATION, MAX_CORRELATION)
    }

    /// Compute quadrature for W ~ Gamma(ν/2, ν/2) integration.
    ///
    /// W = χ²(ν)/ν has a Gamma(ν/2, 2/ν) distribution (shape=ν/2, scale=2/ν).
    ///
    /// The density is: f(w) = (ν/2)^{ν/2} / Γ(ν/2) · w^{ν/2-1} · exp(-νw/2)
    ///
    /// Using the substitution u = νw/2 (so w = 2u/ν, dw = 2/ν du):
    /// ∫ g(w) f(w) dw = ∫ g(2u/ν) · u^{ν/2-1} · exp(-u) / Γ(ν/2) du
    ///
    /// Standard Gauss-Laguerre (α=0) integrates ∫ h(u) exp(-u) du, so each
    /// weight must include the u^{ν/2-1} / Γ(ν/2) correction.
    fn compute_gamma_quadrature(nu: f64, n: usize) -> Vec<(f64, f64)> {
        let effective_n = n.max(10);
        let laguerre_nodes_weights: &[(f64, f64)] = &[
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
        ];

        let num_points = laguerre_nodes_weights.len().min(effective_n);
        let alpha = nu / 2.0;
        let ln_gamma_alpha = ln_gamma(alpha);

        laguerre_nodes_weights[..num_points]
            .iter()
            .filter_map(|&(node, laguerre_weight)| {
                if node < 1e-15 {
                    return None;
                }
                // w = 2·node/ν  (transform from Laguerre variable u to Gamma variate w)
                let w = 2.0 * node / nu;

                // Weight correction: u^{α-1} / Γ(α)
                // = exp((α-1)·ln(u) - ln_gamma(α))
                let gamma_correction = ((alpha - 1.0) * node.ln() - ln_gamma_alpha).exp();
                let weight = laguerre_weight * gamma_correction;

                if weight < 1e-30 || !weight.is_finite() {
                    return None;
                }

                Some((w, weight))
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
        if factor_realization.len() != 1 {
            tracing::warn!(
                expected = 1,
                actual = factor_realization.len(),
                "StudentTCopula: factor_realization length mismatch, defaulting missing to 0.0"
            );
        }
        let m = factor_realization.first().copied().unwrap_or(0.0);
        let nu = self.degrees_of_freedom;

        if correlation <= 1e-10 {
            return student_t_cdf(default_threshold, nu);
        }
        if correlation >= 1.0 - 1e-10 {
            let scaling = ((nu + 1.0) / (nu + m * m)).sqrt();
            return student_t_cdf((default_threshold - m) * scaling, nu + 1.0);
        }

        let rho = self.smooth_correlation(correlation);

        let sqrt_rho = rho.sqrt();
        let sqrt_1mr = (1.0 - rho).sqrt();

        // Standard multivariate t-copula conditional (Demarta & McNeil 2005):
        // P(default | M=m) = t_{ν+1}( (c - √ρ·m)/√(1-ρ) · √((ν+1)/(ν+m²)) )
        let base_arg = (default_threshold - sqrt_rho * m) / sqrt_1mr;
        let scaling = ((nu + 1.0) / (nu + m * m)).sqrt();
        let conditional_threshold = base_arg * scaling;

        student_t_cdf(conditional_threshold, nu + 1.0)
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // Two-layer integration using variance-gamma mixing:
        // M ~ t(ν) can be represented as M = Z/√W where Z ~ N(0,1), W ~ Gamma(ν/2, ν/2)
        //
        // E[g(M)] = E_W[ E_Z[ g(Z/√W) | W ] ]
        //
        // Outer: over W using Gauss-Laguerre with Gamma density correction
        // Inner: over Z using Gauss-Hermite (standard normal)

        let mut result = 0.0;
        for &(w_val, w_weight) in &self.gamma_quadrature {
            let inv_sqrt_w = 1.0 / w_val.sqrt();

            let inner = self.inner_quadrature.integrate(|z_gauss| {
                let m = z_gauss * inv_sqrt_w;
                f(&[m])
            });

            result += w_weight * inner;
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
        2.0 * student_t_cdf(arg, nu + 1.0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::math::standard_normal_inv_cdf;

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

        assert!(lambda > 0.0, "Tail dependence should be positive");
        assert!(lambda < 1.0, "Tail dependence should be < 1");
    }

    #[test]
    fn test_tail_dependence_increases_with_correlation() {
        let copula = StudentTCopula::new(5.0);

        let lambda_low = copula.tail_dependence(0.2);
        let lambda_mid = copula.tail_dependence(0.5);
        let lambda_high = copula.tail_dependence(0.8);

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

        assert!(
            lambda < 0.05,
            "High df should give near-zero tail dependence"
        );
    }

    #[test]
    fn test_conditional_prob_sensitive_to_factor() {
        let copula = StudentTCopula::new(5.0);
        let threshold = student_t_inv_cdf(0.05, 5.0);
        let correlation = 0.3;

        let prob_neg = copula.conditional_default_prob(threshold, &[-2.0], correlation);
        let prob_zero = copula.conditional_default_prob(threshold, &[0.0], correlation);
        let prob_pos = copula.conditional_default_prob(threshold, &[2.0], correlation);

        assert!(prob_neg > prob_zero);
        assert!(prob_pos < prob_zero);
    }

    #[test]
    fn test_conditional_prob_uses_nu_plus_one_in_perfect_correlation_limit() {
        let copula = StudentTCopula::new(5.0);
        let nu = 5.0;

        // Test with small and large factor values to exercise the scaling
        for &factor in &[0.35, 3.0, -2.5] {
            let threshold = -1.25;
            let scaling = ((nu + 1.0) / (nu + factor * factor)).sqrt();
            let expected = student_t_cdf((threshold - factor) * scaling, nu + 1.0);

            let prob = copula.conditional_default_prob(threshold, &[factor], 1.0);

            assert!(
                (prob - expected).abs() < 1e-12,
                "perfect-correlation limit should use scaling: factor={}, expected {}, got {}",
                factor,
                expected,
                prob
            );
        }
    }

    #[test]
    fn test_integration_recovers_unconditional() {
        // Critical self-consistency test: E[P(default|M)] must equal PD
        for &df in &[4.0, 5.0, 10.0, 30.0] {
            let copula = StudentTCopula::new(df);
            let pd = 0.05;
            let threshold = student_t_inv_cdf(pd, df);
            let correlation = 0.30;

            let integrated_prob = copula
                .integrate_fn(&|z| copula.conditional_default_prob(threshold, z, correlation));

            assert!(
                (integrated_prob - pd).abs() < 0.005,
                "df={}: Integrated probability {} should equal unconditional {} (error={})",
                df,
                integrated_prob,
                pd,
                (integrated_prob - pd).abs()
            );
        }
    }

    #[test]
    fn test_integration_recovers_unconditional_various_pd() {
        let copula = StudentTCopula::new(5.0);

        for &pd in &[0.01, 0.05, 0.10, 0.20] {
            let threshold = student_t_inv_cdf(pd, 5.0);
            let correlation = 0.30;

            let integrated_prob = copula
                .integrate_fn(&|z| copula.conditional_default_prob(threshold, z, correlation));

            assert!(
                (integrated_prob - pd).abs() < 0.005,
                "pd={}: Integrated probability {} (error={})",
                pd,
                integrated_prob,
                (integrated_prob - pd).abs()
            );
        }
    }

    #[test]
    fn test_tail_dependence_golden_values() {
        let test_cases = [(4.0, 0.5), (5.0, 0.5), (10.0, 0.5)];

        for (df, rho) in test_cases {
            let copula = StudentTCopula::new(df);
            let lambda = copula.tail_dependence(rho);

            assert!(
                (0.0..=1.0).contains(&lambda),
                "Tail dependence for df={}, ρ={}: got {}, expected in [0,1]",
                df,
                rho,
                lambda
            );

            assert!(
                lambda < 0.5,
                "Tail dependence {} seems too high for df={}, ρ={}",
                lambda,
                df,
                rho
            );
        }

        let copula_4 = StudentTCopula::new(4.0);
        let copula_10 = StudentTCopula::new(10.0);
        assert!(
            copula_4.tail_dependence(0.5) > copula_10.tail_dependence(0.5),
            "Lower df should give higher tail dependence"
        );
    }

    #[test]
    fn test_student_t_cdf_accuracy() {
        let cdf = student_t_cdf(-2.0, 5.0);
        assert!(
            (cdf - 0.051).abs() < 0.002,
            "CDF(-2.0, df=5) = {}, expected ~0.051",
            cdf
        );

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
    fn test_gamma_quadrature_properties() {
        for df in [4.0, 5.0, 10.0, 20.0] {
            let copula = StudentTCopula::new(df);
            let points = &copula.gamma_quadrature;

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
                (weight_sum - 1.0).abs() < 0.05,
                "Gamma({}/2) weights sum to {}, expected ~1.0",
                df,
                weight_sum
            );

            assert!(
                points.len() >= 3,
                "Expected at least 3 quadrature points, got {}",
                points.len()
            );
        }
    }

    #[test]
    fn test_high_df_converges_to_gaussian() {
        use finstack_core::math::norm_cdf;

        let df = 50.0;
        let copula = StudentTCopula::new(df);
        let pd = 0.05;
        let threshold = student_t_inv_cdf(pd, df);
        let correlation = 0.30;

        let t_prob = copula.conditional_default_prob(threshold, &[0.0], correlation);

        let gauss_threshold = standard_normal_inv_cdf(pd);
        let sqrt_rho = correlation.sqrt();
        let sqrt_1mr = (1.0 - correlation).sqrt();
        let gauss_prob = norm_cdf((gauss_threshold - sqrt_rho * 0.0) / sqrt_1mr);

        assert!(
            (t_prob - gauss_prob).abs() < 0.02,
            "High-df t ({}) should be close to Gaussian ({})",
            t_prob,
            gauss_prob
        );
    }
}
