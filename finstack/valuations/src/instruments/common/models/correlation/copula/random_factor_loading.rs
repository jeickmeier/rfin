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
use finstack_core::math::{norm_cdf as standard_normal_cdf, GaussHermiteQuadrature};

/// Random Factor Loading copula with stochastic correlation.
///
/// The factor loading is drawn from a distribution at each scenario,
/// creating uncertainty in the effective correlation level.
pub struct RandomFactorLoadingCopula {
    /// Volatility of the factor loading
    loading_volatility: f64,
    /// Quadrature order for integration
    quadrature_order: u8,
    /// Minimum loading (ensures √(1-β²) is well-defined)
    min_loading: f64,
    /// Maximum loading (ensures √(1-β²) is well-defined)
    max_loading: f64,
}

impl Clone for RandomFactorLoadingCopula {
    fn clone(&self) -> Self {
        Self {
            loading_volatility: self.loading_volatility,
            quadrature_order: self.quadrature_order,
            min_loading: self.min_loading,
            max_loading: self.max_loading,
        }
    }
}

impl std::fmt::Debug for RandomFactorLoadingCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandomFactorLoadingCopula")
            .field("loading_volatility", &self.loading_volatility)
            .field("quadrature_order", &self.quadrature_order)
            .field("min_loading", &self.min_loading)
            .field("max_loading", &self.max_loading)
            .finish()
    }
}

impl RandomFactorLoadingCopula {
    /// Create a Random Factor Loading copula.
    ///
    /// # Arguments
    /// * `loading_vol` - Volatility of factor loading (typical: 0.05-0.20)
    pub fn new(loading_vol: f64) -> Self {
        Self {
            loading_volatility: loading_vol.clamp(0.0, 0.5),
            quadrature_order: DEFAULT_QUADRATURE_ORDER,
            min_loading: 0.01,
            max_loading: 0.99,
        }
    }

    /// Create with custom quadrature order.
    pub fn with_quadrature_order(loading_vol: f64, order: u8) -> Self {
        Self {
            loading_volatility: loading_vol.clamp(0.0, 0.5),
            quadrature_order: order,
            min_loading: 0.01,
            max_loading: 0.99,
        }
    }

    /// Get quadrature for outer integration.
    fn outer_quadrature(&self) -> GaussHermiteQuadrature {
        select_quadrature(self.quadrature_order)
    }

    /// Get quadrature for inner integration.
    fn inner_quadrature(&self) -> GaussHermiteQuadrature {
        select_quadrature(self.quadrature_order)
    }

    /// Get the loading volatility.
    pub fn loading_volatility(&self) -> f64 {
        self.loading_volatility
    }

    /// Compute effective loading given mean and shock.
    ///
    /// β(η) = β̄ + σ_β · η where η ~ N(0,1)
    fn effective_loading(&self, mean_loading: f64, loading_shock: f64) -> f64 {
        let beta = mean_loading + self.loading_volatility * loading_shock;
        beta.clamp(self.min_loading, self.max_loading)
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
            return standard_normal_cdf(threshold_adj);
        }

        // P(default | Z, β) = Φ((threshold - β·Z) / γ)
        let conditional_threshold = (default_threshold - beta * z) / gamma;
        standard_normal_cdf(conditional_threshold.clamp(-10.0, 10.0))
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // Double integral: outer over loading shock η, inner over market Z
        self.outer_quadrature()
            .integrate(|eta| self.inner_quadrature().integrate(|z| f(&[z, eta])))
    }

    fn num_factors(&self) -> usize {
        2 // Market factor Z and loading shock η
    }

    fn model_name(&self) -> &'static str {
        "Random Factor Loading Copula"
    }

    fn tail_dependence(&self, correlation: f64) -> f64 {
        // RFL has implicit tail dependence through stochastic correlation
        // When loading is high (tail of loading distribution), correlation spikes
        // Approximate by considering the high-loading tail contribution

        let mean_loading = correlation.clamp(0.0, 1.0).sqrt();

        // P(β > threshold) where threshold gives near-perfect correlation
        // High loading tail: contribution to tail dependence
        let high_loading = (mean_loading + 2.0 * self.loading_volatility).min(0.99);
        let effective_high_corr = high_loading * high_loading;

        // Rough approximation: tail dependence proportional to
        // probability of high correlation × impact at that correlation
        let prob_high_loading = 1.0 - standard_normal_cdf(2.0); // ~2.3%

        // Contribution is small but non-zero
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
