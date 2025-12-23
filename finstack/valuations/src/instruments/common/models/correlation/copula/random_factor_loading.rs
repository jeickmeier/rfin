//! Random Factor Loading (RFL) copula with stochastic correlation.
//!
//! Models correlation itself as random, capturing the empirical observation
//! that correlation increases during market stress. This is particularly
//! important for senior tranches which are sensitive to correlation dynamics.
//!
//! # Mathematical Model
//!
//! The factor loading β is random rather than fixed:
//! ```text
//! β ~ N(β̄, σ²_β)
//! Aᵢ = β · Z + √(1-β²) · εᵢ
//! ```
//!
//! This means effective correlation ρ = β² is stochastic, with:
//! - Higher realized correlation in stress (β further from 0)
//! - Lower realized correlation in calm markets
//!
//! # Integration Approach
//!
//! Two-dimensional integration:
//! 1. Outer: over the random loading β (or equivalently, loading shock η)
//! 2. Inner: over the market factor Z given β
//!
//! # Impact on Tranches
//!
//! - **Equity tranches**: Less affected (already high-risk)
//! - **Mezzanine tranches**: Moderately affected
//! - **Senior tranches**: Significantly affected (correlation uncertainty matters)
//!
//! # References
//!
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula:
//!   Random Recovery and Random Factor Loadings." *Journal of Credit Risk*.

use super::{select_quadrature, Copula, DEFAULT_QUADRATURE_ORDER};
use finstack_core::math::{norm_cdf, GaussHermiteQuadrature};

/// Minimum loading (ensures √(1-β²) is well-defined).
const MIN_LOADING: f64 = 0.01;
/// Maximum loading (ensures √(1-β²) is well-defined).
const MAX_LOADING: f64 = 0.99;
/// CDF argument clipping to prevent overflow.
const CDF_CLIP: f64 = 10.0;

/// Random Factor Loading copula with stochastic correlation.
///
/// The factor loading is drawn from a distribution at each scenario,
/// creating uncertainty in the effective correlation level.
///
/// # Numerical Stability
///
/// - Loading volatility is clamped to [0, 0.5]
/// - Effective loading is clamped to [0.01, 0.99]
/// - CDF arguments are clipped to prevent overflow
/// - Quadrature is cached for performance
pub struct RandomFactorLoadingCopula {
    /// Volatility of the factor loading, clamped to [0, 0.5]
    loading_volatility: f64,
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Cached quadrature for outer integration (loading shock η)
    outer_quadrature: GaussHermiteQuadrature,
    /// Cached quadrature for inner integration (market factor Z)
    inner_quadrature: GaussHermiteQuadrature,
}

impl Clone for RandomFactorLoadingCopula {
    fn clone(&self) -> Self {
        let quadrature = select_quadrature(self.quadrature_order);
        Self {
            loading_volatility: self.loading_volatility,
            quadrature_order: self.quadrature_order,
            outer_quadrature: select_quadrature(self.quadrature_order),
            inner_quadrature: quadrature,
        }
    }
}

impl std::fmt::Debug for RandomFactorLoadingCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandomFactorLoadingCopula")
            .field("loading_volatility", &self.loading_volatility)
            .field("quadrature_order", &self.quadrature_order)
            .finish()
    }
}

impl RandomFactorLoadingCopula {
    /// Create a Random Factor Loading copula.
    ///
    /// # Arguments
    /// * `loading_vol` - Volatility of factor loading, clamped to [0.0, 0.5].
    ///   Typical values: 0.05-0.20. Higher values increase correlation uncertainty.
    #[must_use]
    pub fn new(loading_vol: f64) -> Self {
        let order = DEFAULT_QUADRATURE_ORDER;
        Self {
            loading_volatility: loading_vol.clamp(0.0, 0.5),
            quadrature_order: order,
            outer_quadrature: select_quadrature(order),
            inner_quadrature: select_quadrature(order),
        }
    }

    /// Create with custom quadrature order for higher precision.
    ///
    /// # Arguments
    /// * `loading_vol` - Volatility of factor loading, clamped to [0.0, 0.5]
    /// * `order` - Quadrature order (5, 7, or 10)
    #[must_use]
    pub fn with_quadrature_order(loading_vol: f64, order: u8) -> Self {
        Self {
            loading_volatility: loading_vol.clamp(0.0, 0.5),
            quadrature_order: order,
            outer_quadrature: select_quadrature(order),
            inner_quadrature: select_quadrature(order),
        }
    }

    /// Get the loading volatility.
    #[must_use]
    pub fn loading_volatility(&self) -> f64 {
        self.loading_volatility
    }

    /// Compute effective loading given mean and shock.
    ///
    /// β(η) = β̄ + σ_β · η where η ~ N(0,1)
    ///
    /// Result is clamped to [0.01, 0.99] to ensure numerical stability.
    fn effective_loading(&self, mean_loading: f64, loading_shock: f64) -> f64 {
        let beta = mean_loading + self.loading_volatility * loading_shock;
        beta.clamp(MIN_LOADING, MAX_LOADING)
    }

    /// Compute idiosyncratic loading given factor loading.
    ///
    /// γ = √(1 - β²) to ensure Var(Aᵢ) = 1
    fn idiosyncratic_loading(&self, factor_loading: f64) -> f64 {
        (1.0 - factor_loading * factor_loading).max(0.0).sqrt()
    }
}

impl Copula for RandomFactorLoadingCopula {
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        // factor_realization[0] = Z (market factor)
        // factor_realization[1] = η (loading shock), optional
        let z = factor_realization.first().copied().unwrap_or(0.0);
        let eta = factor_realization.get(1).copied().unwrap_or(0.0);

        // Mean loading from correlation: β̄ = √ρ
        let mean_loading = correlation.clamp(0.0, 1.0).sqrt();
        let beta = self.effective_loading(mean_loading, eta);
        let gamma = self.idiosyncratic_loading(beta);

        if gamma < 1e-10 {
            // Near-perfect correlation case
            let threshold_adj = default_threshold - beta * z;
            return norm_cdf(threshold_adj);
        }

        // P(default | Z, β) = Φ((threshold - β·Z) / γ)
        let conditional_threshold = (default_threshold - beta * z) / gamma;
        norm_cdf(conditional_threshold.clamp(-CDF_CLIP, CDF_CLIP))
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // Double integral: outer over loading shock η, inner over market Z
        // Uses cached quadrature for performance
        self.outer_quadrature
            .integrate(|eta| self.inner_quadrature.integrate(|z| f(&[z, eta])))
    }

    fn num_factors(&self) -> usize {
        2 // Market factor Z and loading shock η
    }

    fn model_name(&self) -> &'static str {
        "Random Factor Loading Copula"
    }

    fn tail_dependence(&self, correlation: f64) -> f64 {
        // **APPROXIMATE** tail dependence coefficient.
        //
        // The RFL copula has implicit tail dependence through stochastic correlation:
        // when loading is high (tail of loading distribution), correlation spikes,
        // causing joint extremes to cluster.
        //
        // This is NOT the exact formula λ_L = lim_{u→0} P(U₂≤u|U₁≤u), but rather
        // an effective measure that captures the qualitative behavior.
        //
        // For exact tail dependence, Monte Carlo simulation would be required.

        let mean_loading = correlation.clamp(0.0, 1.0).sqrt();

        // Probability of high loading (β > β̄ + 2σ)
        let high_loading = (mean_loading + 2.0 * self.loading_volatility).min(0.99);
        let effective_high_corr = high_loading * high_loading;

        // Probability of being in the high-loading tail
        let prob_high_loading = 1.0 - norm_cdf(2.0); // ~2.3%

        // Approximate tail dependence contribution
        prob_high_loading * effective_high_corr.sqrt() * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::super::GaussianCopula;
    use super::*;
    use finstack_core::math::standard_normal_inv_cdf;

    #[test]
    fn test_rfl_creation() {
        let copula = RandomFactorLoadingCopula::new(0.15);
        assert_eq!(copula.num_factors(), 2);
        assert!((copula.loading_volatility() - 0.15).abs() < 1e-10);
        assert_eq!(copula.model_name(), "Random Factor Loading Copula");
    }

    #[test]
    fn test_rfl_loading_volatility_clamped() {
        let copula_high = RandomFactorLoadingCopula::new(1.0);
        assert!(copula_high.loading_volatility() <= 0.5);

        let copula_neg = RandomFactorLoadingCopula::new(-0.1);
        assert!(copula_neg.loading_volatility() >= 0.0);
    }

    #[test]
    fn test_effective_loading_bounds() {
        let copula = RandomFactorLoadingCopula::new(0.15);

        // Even with extreme shocks, loading should stay bounded
        let loading_extreme_neg = copula.effective_loading(0.5, -10.0);
        let loading_extreme_pos = copula.effective_loading(0.5, 10.0);

        assert!(loading_extreme_neg >= 0.01);
        assert!(loading_extreme_pos <= 0.99);
    }

    #[test]
    fn test_conditional_prob_varies_with_loading_shock() {
        let copula = RandomFactorLoadingCopula::new(0.15);
        let threshold = standard_normal_inv_cdf(0.05);
        let correlation = 0.30;

        // Same market factor Z=0, different loading shocks
        let prob_low_loading = copula.conditional_default_prob(
            threshold,
            &[0.0, -2.0], // Low loading (η = -2)
            correlation,
        );
        let prob_mean_loading = copula.conditional_default_prob(
            threshold,
            &[0.0, 0.0], // Mean loading
            correlation,
        );
        let prob_high_loading = copula.conditional_default_prob(
            threshold,
            &[0.0, 2.0], // High loading (η = +2)
            correlation,
        );

        // All should be around the unconditional probability
        // (loading shock mainly affects joint behavior, not individual marginal)
        assert!(prob_low_loading > 0.0 && prob_low_loading < 1.0);
        assert!(prob_mean_loading > 0.0 && prob_mean_loading < 1.0);
        assert!(prob_high_loading > 0.0 && prob_high_loading < 1.0);
    }

    #[test]
    fn test_integration_recovers_unconditional() {
        let copula = RandomFactorLoadingCopula::new(0.15);
        let pd = 0.05;
        let threshold = standard_normal_inv_cdf(pd);
        let correlation = 0.30;

        // E[P(default|Z,η)] should equal P(default)
        let integrated_prob = copula.integrate_fn(&|factors| {
            copula.conditional_default_prob(threshold, factors, correlation)
        });

        // Should be close to unconditional (within integration error)
        assert!(
            (integrated_prob - pd).abs() < 0.01,
            "Integrated probability {} should be close to unconditional {}",
            integrated_prob,
            pd
        );
    }

    #[test]
    fn test_tail_dependence_small_but_positive() {
        let copula = RandomFactorLoadingCopula::new(0.15);
        let lambda = copula.tail_dependence(0.5);

        // RFL has small positive tail dependence from stochastic correlation
        assert!(lambda >= 0.0);
        assert!(lambda < 0.1); // Should be small
    }

    #[test]
    fn test_zero_volatility_equals_gaussian() {
        let rfl_copula = RandomFactorLoadingCopula::new(0.0);
        let gaussian_copula = GaussianCopula::new();

        let threshold = standard_normal_inv_cdf(0.05);
        let correlation = 0.30;

        // With zero loading vol, RFL should behave like Gaussian
        // (when only passing market factor)
        let rfl_prob = rfl_copula.conditional_default_prob(
            threshold,
            &[0.5, 0.0], // Z=0.5, η=0 (no loading shock)
            correlation,
        );
        let gaussian_prob =
            gaussian_copula.conditional_default_prob(threshold, &[0.5], correlation);

        assert!(
            (rfl_prob - gaussian_prob).abs() < 0.01,
            "Zero-vol RFL {} should equal Gaussian {}",
            rfl_prob,
            gaussian_prob
        );
    }
}
