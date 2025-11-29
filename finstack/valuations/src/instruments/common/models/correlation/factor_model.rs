//! Factor models for correlated behavior in credit portfolios.
//!
//! Factor models drive the correlation between prepayment and default events
//! through common systematic factors. This module provides:
//!
//! - [`FactorModel`] trait: Common interface for all factor models
//! - [`SingleFactorModel`]: One-factor model (common market factor)
//! - [`TwoFactorModel`]: Two-factor model (prepayment + credit factors)
//!
//! # Mathematical Framework
//!
//! For entity i, the latent variable depends on factors:
//! ```text
//! Aᵢ = β₁·Z₁ + β₂·Z₂ + ... + γᵢ·εᵢ
//! ```
//!
//! where:
//! - Zₖ are systematic factors (standard normal)
//! - βₖ are factor loadings
//! - εᵢ is the idiosyncratic component
//! - γᵢ = √(1 - Σβₖ²) ensures Var(Aᵢ) = 1

/// Factor model for correlated behavior.
///
/// Implementations provide factor specifications and correlation matrices
/// for analytical pricing and scenario generation.
pub trait FactorModel: Send + Sync + std::fmt::Debug {
    /// Number of factors in the model.
    fn num_factors(&self) -> usize;

    /// Get the factor correlation matrix (flattened row-major).
    ///
    /// For n factors, returns n×n values where element [i,j] = correlation(Zᵢ, Zⱼ).
    fn correlation_matrix(&self) -> &[f64];

    /// Get factor volatilities.
    fn volatilities(&self) -> &[f64];

    /// Get factor names for reporting.
    fn factor_names(&self) -> Vec<&'static str>;

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str;

    /// Compute conditional factor value given a standard normal draw.
    ///
    /// For correlated factors, uses Cholesky decomposition internally.
    fn conditional_factor(&self, factor_index: usize, z: f64) -> f64;
}

/// Factor model specification for configuration and serialization.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", deny_unknown_fields))]
pub enum FactorSpec {
    /// Single factor model (common market factor).
    SingleFactor {
        /// Factor volatility (std dev of innovations)
        volatility: f64,
        /// Mean reversion speed (0 = random walk)
        mean_reversion: f64,
    },

    /// Two-factor model for prepayment and credit.
    TwoFactor {
        /// Prepayment factor volatility
        prepay_vol: f64,
        /// Credit factor volatility
        credit_vol: f64,
        /// Correlation between prepayment and credit factors
        correlation: f64,
    },

    /// Multi-factor model with custom correlation matrix.
    MultiFactor {
        /// Number of factors
        num_factors: usize,
        /// Factor volatilities
        volatilities: Vec<f64>,
        /// Correlation matrix (flattened row-major)
        correlations: Vec<f64>,
    },
}

impl Default for FactorSpec {
    fn default() -> Self {
        FactorSpec::SingleFactor {
            volatility: 1.0,
            mean_reversion: 0.0,
        }
    }
}

impl FactorSpec {
    /// Create a single factor specification.
    pub fn single_factor(volatility: f64, mean_reversion: f64) -> Self {
        FactorSpec::SingleFactor {
            volatility: volatility.clamp(0.01, 2.0),
            mean_reversion: mean_reversion.clamp(0.0, 10.0),
        }
    }

    /// Create a two-factor specification.
    pub fn two_factor(prepay_vol: f64, credit_vol: f64, correlation: f64) -> Self {
        FactorSpec::TwoFactor {
            prepay_vol: prepay_vol.clamp(0.01, 2.0),
            credit_vol: credit_vol.clamp(0.01, 2.0),
            correlation: correlation.clamp(-0.99, 0.99),
        }
    }

    /// Build a factor model from this specification.
    pub fn build(&self) -> Box<dyn FactorModel> {
        match self {
            FactorSpec::SingleFactor {
                volatility,
                mean_reversion,
            } => Box::new(SingleFactorModel::new(*volatility, *mean_reversion)),
            FactorSpec::TwoFactor {
                prepay_vol,
                credit_vol,
                correlation,
            } => Box::new(TwoFactorModel::new(*prepay_vol, *credit_vol, *correlation)),
            FactorSpec::MultiFactor {
                num_factors,
                volatilities,
                correlations,
            } => Box::new(MultiFactorModel::new(
                *num_factors,
                volatilities.clone(),
                correlations.clone(),
            )),
        }
    }

    /// Get the number of factors.
    pub fn num_factors(&self) -> usize {
        match self {
            FactorSpec::SingleFactor { .. } => 1,
            FactorSpec::TwoFactor { .. } => 2,
            FactorSpec::MultiFactor { num_factors, .. } => *num_factors,
        }
    }
}

/// Single-factor model (common market factor).
///
/// Models all correlation through a single systematic factor.
/// Suitable for simple correlation structures.
#[derive(Clone, Debug)]
pub struct SingleFactorModel {
    volatility: f64,
    mean_reversion: f64,
    correlation_matrix: Vec<f64>,
    volatilities: Vec<f64>,
}

impl SingleFactorModel {
    /// Create a single-factor model.
    ///
    /// # Arguments
    /// * `volatility` - Factor volatility
    /// * `mean_reversion` - Mean reversion speed (0 = random walk)
    pub fn new(volatility: f64, mean_reversion: f64) -> Self {
        let vol = volatility.clamp(0.01, 2.0);
        Self {
            volatility: vol,
            mean_reversion: mean_reversion.clamp(0.0, 10.0),
            correlation_matrix: vec![1.0],
            volatilities: vec![vol],
        }
    }

    /// Get the volatility.
    pub fn volatility(&self) -> f64 {
        self.volatility
    }

    /// Get the mean reversion speed.
    pub fn mean_reversion(&self) -> f64 {
        self.mean_reversion
    }
}

impl FactorModel for SingleFactorModel {
    fn num_factors(&self) -> usize {
        1
    }

    fn correlation_matrix(&self) -> &[f64] {
        &self.correlation_matrix
    }

    fn volatilities(&self) -> &[f64] {
        &self.volatilities
    }

    fn factor_names(&self) -> Vec<&'static str> {
        vec!["Market"]
    }

    fn model_name(&self) -> &'static str {
        "Single Factor Model"
    }

    fn conditional_factor(&self, _factor_index: usize, z: f64) -> f64 {
        z * self.volatility
    }
}

/// Two-factor model for prepayment and credit.
///
/// Models prepayment and credit behavior through two correlated factors.
/// Captures the empirical negative correlation between prepayment and default.
#[derive(Clone, Debug)]
pub struct TwoFactorModel {
    prepay_vol: f64,
    credit_vol: f64,
    correlation: f64,
    correlation_matrix: Vec<f64>,
    volatilities: Vec<f64>,
    // Cholesky lower triangular for correlated sampling: L[1][0], L[1][1]
    // Used by external callers for generating correlated factors
    cholesky_l10: f64,
    cholesky_l11: f64,
}

impl TwoFactorModel {
    /// Create a two-factor model.
    ///
    /// # Arguments
    /// * `prepay_vol` - Prepayment factor volatility
    /// * `credit_vol` - Credit factor volatility
    /// * `correlation` - Correlation between factors (typically negative for RMBS)
    pub fn new(prepay_vol: f64, credit_vol: f64, correlation: f64) -> Self {
        let corr = correlation.clamp(-0.99, 0.99);
        let pv = prepay_vol.clamp(0.01, 2.0);
        let cv = credit_vol.clamp(0.01, 2.0);

        // Cholesky decomposition of correlation matrix [[1, ρ], [ρ, 1]]
        // L = [[1, 0], [ρ, √(1-ρ²)]]
        let sqrt_1mr2 = (1.0 - corr * corr).sqrt();

        Self {
            prepay_vol: pv,
            credit_vol: cv,
            correlation: corr,
            // Row-major: [ρ₁₁, ρ₁₂, ρ₂₁, ρ₂₂]
            correlation_matrix: vec![1.0, corr, corr, 1.0],
            volatilities: vec![pv, cv],
            cholesky_l10: corr,
            cholesky_l11: sqrt_1mr2,
        }
    }

    /// Standard RMBS calibration with negative prepay-credit correlation.
    pub fn rmbs_standard() -> Self {
        Self::new(0.20, 0.25, -0.30)
    }

    /// Standard CLO calibration.
    pub fn clo_standard() -> Self {
        Self::new(0.15, 0.30, -0.20)
    }

    /// Get prepayment factor volatility.
    pub fn prepay_vol(&self) -> f64 {
        self.prepay_vol
    }

    /// Get credit factor volatility.
    pub fn credit_vol(&self) -> f64 {
        self.credit_vol
    }

    /// Get factor correlation.
    pub fn correlation(&self) -> f64 {
        self.correlation
    }

    /// Get Cholesky L[1][0] coefficient for correlated factor generation.
    ///
    /// For generating correlated factors from independent normals (z1, z2):
    /// - Factor 1 = z1 * prepay_vol
    /// - Factor 2 = (l10 * z1 + l11 * z2) * credit_vol
    pub fn cholesky_l10(&self) -> f64 {
        self.cholesky_l10
    }

    /// Get Cholesky L[1][1] coefficient for correlated factor generation.
    pub fn cholesky_l11(&self) -> f64 {
        self.cholesky_l11
    }
}

impl FactorModel for TwoFactorModel {
    fn num_factors(&self) -> usize {
        2
    }

    fn correlation_matrix(&self) -> &[f64] {
        &self.correlation_matrix
    }

    fn volatilities(&self) -> &[f64] {
        &self.volatilities
    }

    fn factor_names(&self) -> Vec<&'static str> {
        vec!["Prepayment", "Credit"]
    }

    fn model_name(&self) -> &'static str {
        "Two-Factor Prepay-Credit Model"
    }

    fn conditional_factor(&self, factor_index: usize, z: f64) -> f64 {
        // For two independent standard normals z1, z2:
        // Factor 1 = z1 * prepay_vol
        // Factor 2 = (ρ * z1 + √(1-ρ²) * z2) * credit_vol
        //
        // This function returns the factor value for a single z draw
        // Caller should provide correlated z values for factor 2
        match factor_index {
            0 => z * self.prepay_vol,
            1 => z * self.credit_vol,
            _ => 0.0,
        }
    }
}

/// Multi-factor model with custom correlation structure.
///
/// Supports arbitrary number of factors with custom correlation matrix.
#[derive(Clone, Debug)]
pub struct MultiFactorModel {
    num_factors: usize,
    volatilities: Vec<f64>,
    correlation_matrix: Vec<f64>,
}

impl MultiFactorModel {
    /// Create a multi-factor model.
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors
    /// * `volatilities` - Factor volatilities (one per factor)
    /// * `correlations` - Correlation matrix (flattened row-major, n×n values)
    pub fn new(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> Self {
        let n = num_factors;

        // Validate or create default volatilities
        let vols = if volatilities.len() == n {
            volatilities
        } else {
            vec![1.0; n]
        };

        // Validate or create identity correlation matrix
        let corrs = if correlations.len() == n * n {
            correlations
        } else {
            let mut identity = vec![0.0; n * n];
            for i in 0..n {
                identity[i * n + i] = 1.0;
            }
            identity
        };

        Self {
            num_factors: n,
            volatilities: vols,
            correlation_matrix: corrs,
        }
    }
}

impl FactorModel for MultiFactorModel {
    fn num_factors(&self) -> usize {
        self.num_factors
    }

    fn correlation_matrix(&self) -> &[f64] {
        &self.correlation_matrix
    }

    fn volatilities(&self) -> &[f64] {
        &self.volatilities
    }

    fn factor_names(&self) -> Vec<&'static str> {
        vec!["Factor"; self.num_factors]
    }

    fn model_name(&self) -> &'static str {
        "Multi-Factor Model"
    }

    fn conditional_factor(&self, factor_index: usize, z: f64) -> f64 {
        if factor_index < self.num_factors {
            z * self.volatilities[factor_index]
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_factor_creation() {
        let model = SingleFactorModel::new(0.25, 0.1);
        assert_eq!(model.num_factors(), 1);
        assert!((model.volatility() - 0.25).abs() < 1e-10);
        assert_eq!(model.factor_names(), vec!["Market"]);
    }

    #[test]
    fn test_two_factor_creation() {
        let model = TwoFactorModel::new(0.20, 0.30, -0.30);
        assert_eq!(model.num_factors(), 2);
        assert!((model.prepay_vol() - 0.20).abs() < 1e-10);
        assert!((model.credit_vol() - 0.30).abs() < 1e-10);
        assert!((model.correlation() - (-0.30)).abs() < 1e-10);
    }

    #[test]
    fn test_conditional_factor() {
        let model = SingleFactorModel::new(1.0, 0.0);

        let factor = model.conditional_factor(0, 1.5);
        assert!((factor - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_two_factor_correlation_matrix() {
        let model = TwoFactorModel::new(0.20, 0.30, -0.30);
        let corr = model.correlation_matrix();

        // Check diagonal is 1
        assert!((corr[0] - 1.0).abs() < 1e-10);
        assert!((corr[3] - 1.0).abs() < 1e-10);

        // Check off-diagonal is correlation
        assert!((corr[1] - (-0.30)).abs() < 1e-10);
        assert!((corr[2] - (-0.30)).abs() < 1e-10);
    }

    #[test]
    fn test_factor_spec_build() {
        let spec = FactorSpec::single_factor(0.25, 0.1);
        let model = spec.build();
        assert_eq!(model.num_factors(), 1);

        let spec = FactorSpec::two_factor(0.20, 0.30, -0.30);
        let model = spec.build();
        assert_eq!(model.num_factors(), 2);
    }

    #[test]
    fn test_standard_calibrations() {
        let rmbs = TwoFactorModel::rmbs_standard();
        assert_eq!(rmbs.num_factors(), 2);
        assert!(rmbs.correlation() < 0.0); // Negative correlation

        let clo = TwoFactorModel::clo_standard();
        assert_eq!(clo.num_factors(), 2);
        assert!(clo.correlation() < 0.0);
    }

    #[test]
    fn test_volatilities() {
        let model = TwoFactorModel::new(0.20, 0.30, -0.30);
        let vols = model.volatilities();
        assert_eq!(vols.len(), 2);
        assert!((vols[0] - 0.20).abs() < 1e-10);
        assert!((vols[1] - 0.30).abs() < 1e-10);
    }
}
