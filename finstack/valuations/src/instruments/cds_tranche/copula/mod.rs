//! Copula models for CDS tranche pricing.
//!
//! Provides a trait-based copula abstraction enabling pluggable correlation
//! models while maintaining backward compatibility with one-factor Gaussian.
//!
//! # Supported Models
//!
//! - **Gaussian**: Standard one-factor Gaussian copula (market default)
//! - **Student-t**: Fat-tailed copula capturing tail dependence
//! - **Random Factor Loading (RFL)**: Stochastic correlation model
//! - **Multi-Factor**: Sector-based correlation structure
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula"
//! - Hull, J., & White, A. (2004). "Valuation of a CDO without Monte Carlo"

mod gaussian;
mod multi_factor;
mod random_factor_loading;
mod student_t;

pub use gaussian::GaussianCopula;
pub use multi_factor::MultiFactorCopula;
pub use random_factor_loading::RandomFactorLoadingCopula;
pub use student_t::StudentTCopula;

use finstack_core::math::GaussHermiteQuadrature;

/// Copula model for portfolio default correlation.
///
/// Implementations provide the conditional default probability P(τᵢ ≤ t | M)
/// given the systematic factor(s) M, enabling integration over the factor space.
///
/// # Model Framework
///
/// All copula models follow the latent variable approach:
/// ```text
/// Aᵢ = f(M, εᵢ)  where M = systematic factors, εᵢ = idiosyncratic
/// Default: τᵢ ≤ t ⟺ Aᵢ ≤ threshold(PD(t))
/// ```
///
/// The copula determines the joint distribution of (M, εᵢ).
pub trait Copula: Send + Sync {
    /// Conditional default probability given factor realization(s).
    ///
    /// P(default | Z) = P(Aᵢ ≤ threshold | Z)
    ///
    /// # Arguments
    /// * `default_threshold` - Φ⁻¹(PD) or t⁻¹(PD) depending on copula
    /// * `factor_realization` - Systematic factor value(s)
    /// * `correlation` - Asset correlation parameter(s)
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64;

    /// Integrate expected value E[f(L)] over the factor distribution.
    ///
    /// Uses appropriate quadrature for the copula's factor distribution.
    /// The integrand receives factor values and returns a scalar.
    fn integrate_fn(&self, f: &dyn Fn(&[f64]) -> f64) -> f64;

    /// Number of systematic factors in the model.
    fn num_factors(&self) -> usize;

    /// Model description for diagnostics.
    fn model_name(&self) -> &'static str;

    /// Lower tail dependence coefficient λ_L.
    ///
    /// Measures the probability of joint extreme defaults:
    /// λ_L = lim_{u→0} P(U₂ ≤ u | U₁ ≤ u)
    ///
    /// - Gaussian copula: λ_L = 0 (no tail dependence)
    /// - Student-t copula: λ_L > 0 (positive tail dependence)
    fn tail_dependence(&self, correlation: f64) -> f64;
}

/// Copula model specification for configuration and serialization.
///
/// Allows copula selection without constructing the full model,
/// enabling deferred construction with market data.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", deny_unknown_fields))]
pub enum CopulaSpec {
    /// One-factor Gaussian copula (market standard default).
    ///
    /// Simple and fast, but lacks tail dependence.
    #[default]
    Gaussian,

    /// Student-t copula with specified degrees of freedom.
    ///
    /// Captures tail dependence - joint extreme defaults are more likely
    /// than Gaussian predicts. Lower df = more tail dependence.
    ///
    /// Typical calibration: df ∈ [4, 10] for CDX tranches.
    StudentT {
        /// Degrees of freedom (must be > 2 for finite variance)
        degrees_of_freedom: f64,
    },

    /// Random Factor Loading copula (stochastic correlation).
    ///
    /// Models correlation itself as random, capturing the empirical
    /// observation that correlation increases during market stress.
    RandomFactorLoading {
        /// Volatility of the factor loading (correlation vol proxy)
        loading_volatility: f64,
    },

    /// Multi-factor Gaussian copula with sector structure.
    ///
    /// Uses multiple systematic factors (global + sector-specific)
    /// to capture industry concentration effects.
    MultiFactor {
        /// Number of systematic factors
        num_factors: usize,
    },
}

impl CopulaSpec {
    /// Create a Gaussian copula specification.
    pub fn gaussian() -> Self {
        CopulaSpec::Gaussian
    }

    /// Create a Student-t copula specification.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (typically 4-10 for CDX)
    ///
    /// # Panics
    /// Panics if df ≤ 2 (variance undefined)
    pub fn student_t(df: f64) -> Self {
        assert!(df > 2.0, "Student-t df must be > 2 for finite variance");
        CopulaSpec::StudentT {
            degrees_of_freedom: df,
        }
    }

    /// Create a Random Factor Loading specification.
    ///
    /// # Arguments
    /// * `loading_vol` - Volatility of factor loading (0.05-0.20 typical)
    pub fn random_factor_loading(loading_vol: f64) -> Self {
        CopulaSpec::RandomFactorLoading {
            loading_volatility: loading_vol.clamp(0.0, 0.5),
        }
    }

    /// Create a Multi-factor copula specification.
    pub fn multi_factor(num_factors: usize) -> Self {
        CopulaSpec::MultiFactor { num_factors }
    }

    /// Build a Gaussian copula from this specification.
    /// Returns None if the specification is not Gaussian.
    pub fn build_gaussian(&self) -> Option<GaussianCopula> {
        match self {
            CopulaSpec::Gaussian => Some(GaussianCopula::new()),
            _ => None,
        }
    }

    /// Build a Student-t copula from this specification.
    /// Returns None if the specification is not Student-t.
    pub fn build_student_t(&self) -> Option<StudentTCopula> {
        match self {
            CopulaSpec::StudentT { degrees_of_freedom } => Some(StudentTCopula::new(*degrees_of_freedom)),
            _ => None,
        }
    }

    /// Build a Random Factor Loading copula from this specification.
    /// Returns None if the specification is not RFL.
    pub fn build_rfl(&self) -> Option<RandomFactorLoadingCopula> {
        match self {
            CopulaSpec::RandomFactorLoading { loading_volatility } => {
                Some(RandomFactorLoadingCopula::new(*loading_volatility))
            }
            _ => None,
        }
    }

    /// Build a Multi-factor copula from this specification.
    /// Returns None if the specification is not Multi-factor.
    pub fn build_multi_factor(&self) -> Option<MultiFactorCopula> {
        match self {
            CopulaSpec::MultiFactor { num_factors } => Some(MultiFactorCopula::new(*num_factors)),
            _ => None,
        }
    }

    /// Check if this is a Gaussian copula specification.
    pub fn is_gaussian(&self) -> bool {
        matches!(self, CopulaSpec::Gaussian)
    }

    /// Check if this is a Student-t copula specification.
    pub fn is_student_t(&self) -> bool {
        matches!(self, CopulaSpec::StudentT { .. })
    }

    /// Check if this is a Random Factor Loading copula specification.
    pub fn is_rfl(&self) -> bool {
        matches!(self, CopulaSpec::RandomFactorLoading { .. })
    }

    /// Check if this is a Multi-factor copula specification.
    pub fn is_multi_factor(&self) -> bool {
        matches!(self, CopulaSpec::MultiFactor { .. })
    }
}

/// Default quadrature order for copula integration.
pub(crate) const DEFAULT_QUADRATURE_ORDER: u8 = 7;

/// Select quadrature based on order.
pub(crate) fn select_quadrature(order: u8) -> GaussHermiteQuadrature {
    match order {
        5 => GaussHermiteQuadrature::order_5(),
        7 => GaussHermiteQuadrature::order_7(),
        10 => GaussHermiteQuadrature::order_10(),
        _ => GaussHermiteQuadrature::order_7(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copula_spec_default() {
        let spec = CopulaSpec::default();
        assert_eq!(spec, CopulaSpec::Gaussian);
    }

    #[test]
    fn test_copula_spec_builders() {
        let gaussian = CopulaSpec::gaussian();
        assert!(matches!(gaussian, CopulaSpec::Gaussian));

        let student_t = CopulaSpec::student_t(5.0);
        assert!(matches!(
            student_t,
            CopulaSpec::StudentT {
                degrees_of_freedom: df
            } if (df - 5.0).abs() < 1e-10
        ));

        let rfl = CopulaSpec::random_factor_loading(0.15);
        assert!(matches!(
            rfl,
            CopulaSpec::RandomFactorLoading {
                loading_volatility: v
            } if (v - 0.15).abs() < 1e-10
        ));
    }

    #[test]
    #[should_panic(expected = "Student-t df must be > 2")]
    fn test_student_t_invalid_df() {
        CopulaSpec::student_t(2.0);
    }

    #[test]
    fn test_copula_build() {
        // Test Gaussian
        let gaussian = CopulaSpec::gaussian();
        assert!(gaussian.is_gaussian());
        let g_copula = gaussian.build_gaussian().expect("Should build Gaussian");
        assert_eq!(g_copula.num_factors(), 1);

        // Test Student-t
        let student_t = CopulaSpec::student_t(5.0);
        assert!(student_t.is_student_t());
        let t_copula = student_t.build_student_t().expect("Should build Student-t");
        assert_eq!(t_copula.num_factors(), 1);

        // Test RFL
        let rfl = CopulaSpec::random_factor_loading(0.1);
        assert!(rfl.is_rfl());
        let rfl_copula = rfl.build_rfl().expect("Should build RFL");
        assert_eq!(rfl_copula.num_factors(), 2);

        // Test Multi-factor
        let mf = CopulaSpec::multi_factor(2);
        assert!(mf.is_multi_factor());
        let mf_copula = mf.build_multi_factor().expect("Should build Multi-factor");
        assert_eq!(mf_copula.num_factors(), 2);
    }
}

