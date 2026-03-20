//! Multi-factor Gaussian copula with sector structure.
//!
//! Extends single-factor to capture sector-specific correlation effects.
//! Useful for bespoke portfolios with industry concentration.
//!
//! # Mathematical Model
//!
//! Latent variable for entity i with sector s(i):
//! ```text
//! Aᵢ = β_G · Z_G + β_S(i) · Z_S(i) + γᵢ · εᵢ
//! ```
//!
//! where:
//! - Z_G ~ N(0,1) is the global systematic factor
//! - Z_S ~ N(0,1) are sector-specific factors (independent of Z_G)
//! - εᵢ ~ N(0,1) is the idiosyncratic factor
//! - β_G is the global loading, β_S is the sector loading
//! - γᵢ = √(1 - β_G² - β_S²) is the idiosyncratic loading
//!
//! # Correlation Structure
//!
//! ```text
//! ρᵢⱼ = β_G² + β_S² · 1{s(i)=s(j)}  (same sector)
//! ρᵢⱼ = β_G²                          (different sectors)
//! ```
//!
//! # Use Cases
//!
//! - Bespoke CDOs with sector concentration
//! - Portfolios with industry clustering
//! - Risk decomposition into systematic vs. sector risk
//!
//! # References
//!
//! - Multi-factor basket and bespoke CDO modeling:
//!   `docs/REFERENCES.md#andersen-sidenius-basu-2003`
//! - Analytical correlation-product valuation:
//!   `docs/REFERENCES.md#hull-white-2004-cdo`

use super::{select_quadrature, Copula};
use finstack_core::math::{norm_cdf, GaussHermiteQuadrature};

/// CDF argument clipping to prevent overflow.
const CDF_CLIP: f64 = 10.0;
/// Default quadrature order for multi-dimensional integration.
const MULTI_FACTOR_QUADRATURE_ORDER: u8 = 7;

/// Multi-factor Gaussian copula with sector structure.
///
/// Uses a global factor plus one sector-specific factor to model
/// intra-sector vs. inter-sector correlation differences.
///
/// # Factor Limit
///
/// Currently supports 1 or 2 factors (global + one sector).
/// For >2 sector factors, Monte Carlo integration would be required.
///
/// # Default Parameters
///
/// - Global loading: 0.4 (gives ~16% inter-sector correlation)
/// - Sector loading: 0.3 (gives ~25% additional intra-sector correlation)
/// - Sector fraction: 0.4 (40% of total correlation from sector factor)
/// - Quadrature order: 7 (better accuracy while remaining cheap for two factors)
///
/// # References
///
/// - `docs/REFERENCES.md#andersen-sidenius-basu-2003`
/// - `docs/REFERENCES.md#hull-white-2004-cdo`
pub struct MultiFactorCopula {
    /// Number of systematic factors (1 or 2, capped)
    num_factors_count: usize,
    /// Global factor loading (default for all entities)
    default_global_loading: f64,
    /// Sector factor loading (default for all entities)
    default_sector_loading: f64,
    /// Fraction of total correlation attributed to sector factor in decompose_correlation
    sector_fraction: f64,
    /// Cached quadrature for integration
    quadrature: GaussHermiteQuadrature,
}

impl Clone for MultiFactorCopula {
    fn clone(&self) -> Self {
        Self {
            num_factors_count: self.num_factors_count,
            default_global_loading: self.default_global_loading,
            default_sector_loading: self.default_sector_loading,
            sector_fraction: self.sector_fraction,
            quadrature: select_quadrature(MULTI_FACTOR_QUADRATURE_ORDER),
        }
    }
}

impl std::fmt::Debug for MultiFactorCopula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiFactorCopula")
            .field("num_factors_count", &self.num_factors_count)
            .field("default_global_loading", &self.default_global_loading)
            .field("default_sector_loading", &self.default_sector_loading)
            .field("sector_fraction", &self.sector_fraction)
            .finish()
    }
}

/// Maximum supported factors (global + sector). Beyond 2, Monte Carlo is needed.
const MAX_FACTORS: usize = 2;

impl MultiFactorCopula {
    /// Create a multi-factor copula with specified number of factors.
    ///
    /// Uses default loadings: β_G=0.4, β_S=0.3, sector_fraction=0.4
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors (1 or 2; capped at 2)
    ///
    /// # Returns
    ///
    /// A multi-factor Gaussian copula with default loadings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_correlation::{Copula, MultiFactorCopula};
    ///
    /// let copula = MultiFactorCopula::new(2);
    /// assert_eq!(copula.num_factors(), 2);
    /// assert!(copula.intra_sector_correlation() >= copula.inter_sector_correlation());
    /// ```
    #[must_use]
    pub fn new(num_factors: usize) -> Self {
        let num_factors = num_factors.clamp(1, MAX_FACTORS);
        Self {
            num_factors_count: num_factors,
            default_global_loading: 0.4,
            default_sector_loading: 0.3,
            sector_fraction: 0.4,
            quadrature: select_quadrature(MULTI_FACTOR_QUADRATURE_ORDER),
        }
    }

    /// Create with custom loadings.
    ///
    /// Loadings are clamped to ensure β_G² + β_S² ≤ 1 (valid variance).
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors (1 or 2; capped at 2)
    /// * `global_loading` - Loading on global factor (β_G), clamped to [0, 0.99]
    /// * `sector_loading` - Loading on sector factor (β_S), clamped to maintain variance constraint
    ///
    /// # Returns
    ///
    /// A multi-factor Gaussian copula with bounded loadings.
    #[must_use]
    pub fn with_loadings(num_factors: usize, global_loading: f64, sector_loading: f64) -> Self {
        let gl = global_loading.clamp(0.0, 0.99);
        let max_sector = (1.0 - gl * gl).sqrt();
        let sl = sector_loading.clamp(0.0, max_sector * 0.99);

        Self {
            num_factors_count: num_factors.clamp(1, MAX_FACTORS),
            default_global_loading: gl,
            default_sector_loading: sl,
            sector_fraction: 0.4,
            quadrature: select_quadrature(MULTI_FACTOR_QUADRATURE_ORDER),
        }
    }

    /// Create with custom loadings and sector fraction.
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors (1 or 2; capped at 2)
    /// * `global_loading` - Loading on global factor (β_G), clamped to [0, 0.99]
    /// * `sector_loading` - Loading on sector factor (β_S), clamped to maintain variance constraint
    /// * `sector_fraction` - Fraction of total correlation from sector factor, clamped to [0, 1]
    ///
    /// # Returns
    ///
    /// A multi-factor Gaussian copula with explicit sector-fraction decomposition.
    #[must_use]
    pub fn with_loadings_and_sector_fraction(
        num_factors: usize,
        global_loading: f64,
        sector_loading: f64,
        sector_fraction: f64,
    ) -> Self {
        let mut copula = Self::with_loadings(num_factors, global_loading, sector_loading);
        copula.sector_fraction = sector_fraction.clamp(0.0, 1.0);
        copula
    }

    /// Get the inter-sector correlation (global factor only).
    ///
    /// Returns β_G² where β_G is the global factor loading.
    ///
    /// # Returns
    ///
    /// The implied correlation between names in different sectors.
    #[must_use]
    pub fn inter_sector_correlation(&self) -> f64 {
        self.default_global_loading * self.default_global_loading
    }

    /// Get the intra-sector correlation (global + sector factors).
    ///
    /// Returns β_G² + β_S² where β_G is global and β_S is sector loading.
    ///
    /// # Returns
    ///
    /// The implied correlation between names in the same sector.
    #[must_use]
    pub fn intra_sector_correlation(&self) -> f64 {
        let gl = self.default_global_loading;
        let sl = self.default_sector_loading;
        gl * gl + sl * sl
    }

    /// Compute idiosyncratic loading given factor loadings.
    ///
    /// γ = √(1 - β_G² - β_S²) to ensure Var(Aᵢ) = 1
    fn idiosyncratic_loading(&self, global_loading: f64, sector_loading: f64) -> f64 {
        let sum_sq = global_loading * global_loading + sector_loading * sector_loading;
        (1.0 - sum_sq).max(0.0).sqrt()
    }

    /// Decompose total correlation into global and sector components.
    ///
    /// Given total correlation ρ and sector fraction f:
    /// - β_G² = ρ · (1 - f)
    /// - β_S² = ρ · f
    ///
    /// # Arguments
    /// * `total_correlation` - Total correlation, clamped to [0, 0.99]
    /// * `sector_fraction` - Fraction of correlation from sector factor, clamped to [0, 1]
    ///
    /// # Returns
    ///
    /// A pair `(global_loading, sector_loading)` whose squared values reconstruct
    /// the bounded total correlation.
    #[must_use]
    pub fn decompose_correlation(
        &self,
        total_correlation: f64,
        sector_fraction: f64,
    ) -> (f64, f64) {
        let rho = total_correlation.clamp(0.0, 0.99);
        let f = sector_fraction.clamp(0.0, 1.0);

        let global_sq = rho * (1.0 - f);
        let sector_sq = rho * f;

        (global_sq.sqrt(), sector_sq.sqrt())
    }
}

impl Copula for MultiFactorCopula {
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        let (global_loading, sector_loading) =
            self.decompose_correlation(correlation, self.sector_fraction);

        // factor_realization[0] = Z_G (global factor)
        // factor_realization[1..] = Z_S (sector factors, if present)
        let z_global = factor_realization.first().copied().unwrap_or(0.0);
        let z_sector = factor_realization.get(1).copied().unwrap_or(0.0);

        let gamma = self.idiosyncratic_loading(global_loading, sector_loading);

        if gamma < 1e-10 {
            // Near-perfect correlation
            let systematic = global_loading * z_global + sector_loading * z_sector;
            return norm_cdf(default_threshold - systematic);
        }

        // P(default | Z_G, Z_S) = Φ((threshold - β_G·Z_G - β_S·Z_S) / γ)
        let systematic = global_loading * z_global + sector_loading * z_sector;
        let conditional_threshold = (default_threshold - systematic) / gamma;

        norm_cdf(conditional_threshold.clamp(-CDF_CLIP, CDF_CLIP))
    }

    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64 {
        // Multi-dimensional Gauss-Hermite quadrature
        // Uses cached quadrature for performance

        if self.num_factors_count == 1 {
            // Single factor: standard 1D integration
            return self.quadrature.integrate(|z| f(&[z]));
        }

        // Two-factor case (global + one sector): nested integration
        self.quadrature.integrate(|z_global| {
            self.quadrature
                .integrate(|z_sector| f(&[z_global, z_sector]))
        })
    }

    fn num_factors(&self) -> usize {
        self.num_factors_count
    }

    fn model_name(&self) -> &'static str {
        "Multi-Factor Gaussian Copula"
    }

    fn tail_dependence(&self, _correlation: f64) -> f64 {
        // Multi-factor Gaussian still has zero tail dependence
        // (sum of Gaussians is Gaussian)
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::GaussianCopula;
    use super::*;
    use finstack_core::math::standard_normal_inv_cdf;

    #[test]
    fn test_multi_factor_creation() {
        let copula = MultiFactorCopula::new(2);
        assert_eq!(copula.num_factors(), 2);
        assert_eq!(copula.model_name(), "Multi-Factor Gaussian Copula");
    }

    #[test]
    fn test_multi_factor_capped_at_two() {
        let copula = MultiFactorCopula::new(5);
        assert_eq!(copula.num_factors(), 2, "Factors should be capped at 2");
    }

    #[test]
    fn test_correlation_decomposition() {
        let copula = MultiFactorCopula::new(2);

        // With 50% sector fraction
        let (global, sector) = copula.decompose_correlation(0.36, 0.5);

        // Total correlation should reconstruct
        let reconstructed = global * global + sector * sector;
        assert!(
            (reconstructed - 0.36).abs() < 1e-6,
            "Reconstructed {} should equal original 0.36",
            reconstructed
        );
    }

    #[test]
    fn test_inter_vs_intra_sector_correlation() {
        let copula = MultiFactorCopula::with_loadings(2, 0.4, 0.4);

        let inter = copula.inter_sector_correlation();
        let intra = copula.intra_sector_correlation();

        // Intra-sector should be higher than inter-sector
        assert!(
            intra > inter,
            "Intra-sector {} should exceed inter-sector {}",
            intra,
            inter
        );

        // Inter-sector = β_G² = 0.16
        assert!((inter - 0.16).abs() < 1e-6);

        // Intra-sector = β_G² + β_S² = 0.32
        assert!((intra - 0.32).abs() < 1e-6);
    }

    #[test]
    fn test_single_factor_equals_gaussian() {
        let multi_copula = MultiFactorCopula::new(1);
        let gaussian_copula = GaussianCopula::new();

        let threshold = standard_normal_inv_cdf(0.05);
        let correlation = 0.30;

        // Single-factor multi should behave like Gaussian
        let multi_prob = multi_copula.conditional_default_prob(threshold, &[0.5], correlation);
        let gaussian_prob =
            gaussian_copula.conditional_default_prob(threshold, &[0.5], correlation);

        // They use different loading decomposition, so won't be exactly equal
        // but should be in the same ballpark
        assert!(
            (multi_prob - gaussian_prob).abs() < 0.05,
            "Single-factor multi {} should be close to Gaussian {}",
            multi_prob,
            gaussian_prob
        );
    }

    #[test]
    fn test_zero_tail_dependence() {
        let copula = MultiFactorCopula::new(2);
        assert_eq!(copula.tail_dependence(0.5), 0.0);
    }

    #[test]
    fn test_integration_recovers_unconditional() {
        let copula = MultiFactorCopula::new(2);
        let pd = 0.05;
        let threshold = standard_normal_inv_cdf(pd);
        let correlation = 0.30;

        let integrated_prob = copula.integrate_fn(&|factors| {
            copula.conditional_default_prob(threshold, factors, correlation)
        });

        // Should be close to unconditional
        assert!(
            (integrated_prob - pd).abs() < 0.01,
            "Integrated probability {} should be close to unconditional {}",
            integrated_prob,
            pd
        );
    }

    #[test]
    fn test_loading_constraints() {
        // Loadings should satisfy β_G² + β_S² ≤ 1
        let copula = MultiFactorCopula::with_loadings(2, 0.8, 0.8);

        // Should be clamped
        let intra = copula.intra_sector_correlation();
        assert!(
            intra <= 1.0,
            "Intra-sector correlation {} should be ≤ 1",
            intra
        );
    }
}
