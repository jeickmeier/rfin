//! Factor models for correlated behavior in credit portfolios.
//!
//! Factor models drive the correlation between prepayment and default events
//! through common systematic factors. This module provides:
//!
//! - [`FactorModel`] trait: Common interface for all factor models
//! - [`SingleFactorModel`]: One-factor model (common market factor)
//! - [`TwoFactorModel`]: Two-factor model (prepayment + credit factors)
//! - [`MultiFactorModel`]: N-factor model with custom correlation matrix
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
//!
//! # Correlation Matrix Requirements
//!
//! For `MultiFactorModel`, the correlation matrix must be:
//! - **Symmetric**: ρᵢⱼ = ρⱼᵢ
//! - **Unit diagonal**: ρᵢᵢ = 1
//! - **Positive semi-definite**: All eigenvalues ≥ 0 (verified via Cholesky)

use finstack_core::math::linalg::{
    cholesky_decomposition as core_cholesky_decomposition, CholeskyError,
};

/// Tolerance for correlation matrix validation.
const CORRELATION_TOLERANCE: f64 = 1e-10;

/// Error types for correlation matrix validation.
#[derive(Clone, Debug, PartialEq)]
pub enum CorrelationMatrixError {
    /// Matrix size does not match expected n×n.
    InvalidSize {
        /// Expected number of factors (n for n×n matrix).
        expected: usize,
        /// Actual length of the matrix array.
        actual: usize,
    },
    /// Diagonal element is not 1.
    DiagonalNotOne {
        /// Index of the invalid diagonal element.
        index: usize,
        /// Actual value found on diagonal.
        value: f64,
    },
    /// Matrix is not symmetric.
    NotSymmetric {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Absolute difference |ρ[i,j] - ρ[j,i]|.
        diff: f64,
    },
    /// Matrix is not positive semi-definite (Cholesky failed).
    NotPositiveSemiDefinite {
        /// Row where Cholesky decomposition failed.
        row: usize,
    },
    /// Correlation value out of bounds [-1, 1].
    OutOfBounds {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Out-of-bounds value.
        value: f64,
    },
}

impl std::fmt::Display for CorrelationMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize { expected, actual } => {
                write!(f, "Invalid matrix size: expected {0}×{0}={1}, got {actual}", expected, expected * expected)
            }
            Self::DiagonalNotOne { index, value } => {
                write!(f, "Diagonal element [{index},{index}] = {value}, expected 1.0")
            }
            Self::NotSymmetric { i, j, diff } => {
                write!(f, "Matrix not symmetric: |ρ[{i},{j}] - ρ[{j},{i}]| = {diff}")
            }
            Self::NotPositiveSemiDefinite { row } => {
                write!(f, "Matrix not positive semi-definite: Cholesky failed at row {row}")
            }
            Self::OutOfBounds { i, j, value } => {
                write!(f, "Correlation ρ[{i},{j}] = {value} out of bounds [-1, 1]")
            }
        }
    }
}

impl std::error::Error for CorrelationMatrixError {}

/// Validate a correlation matrix for use in factor models.
///
/// Checks:
/// - Correct size (n×n flattened)
/// - Unit diagonal
/// - Symmetry
/// - All values in [-1, 1]
/// - Positive semi-definiteness (via Cholesky)
///
/// # Arguments
/// * `matrix` - Flattened row-major correlation matrix
/// * `n` - Number of factors (matrix should be n×n)
///
/// # Returns
/// `Ok(())` if valid, or the first error found.
pub fn validate_correlation_matrix(matrix: &[f64], n: usize) -> Result<(), CorrelationMatrixError> {
    // Check size
    if matrix.len() != n * n {
        return Err(CorrelationMatrixError::InvalidSize {
            expected: n,
            actual: matrix.len(),
        });
    }

    // Check diagonal = 1 and bounds
    for i in 0..n {
        let diag = matrix[i * n + i];
        if (diag - 1.0).abs() > CORRELATION_TOLERANCE {
            return Err(CorrelationMatrixError::DiagonalNotOne { index: i, value: diag });
        }
    }

    // Check symmetry and bounds for off-diagonal
    for i in 0..n {
        for j in (i + 1)..n {
            let rho_ij = matrix[i * n + j];
            let rho_ji = matrix[j * n + i];

            // Check bounds
            if !(-1.0 - CORRELATION_TOLERANCE..=1.0 + CORRELATION_TOLERANCE).contains(&rho_ij) {
                return Err(CorrelationMatrixError::OutOfBounds { i, j, value: rho_ij });
            }

            // Check symmetry
            let diff = (rho_ij - rho_ji).abs();
            if diff > CORRELATION_TOLERANCE {
                return Err(CorrelationMatrixError::NotSymmetric { i, j, diff });
            }
        }
    }

    // Check positive semi-definiteness via Cholesky
    cholesky_decompose(matrix, n)?;

    Ok(())
}

/// Perform Cholesky decomposition of a correlation matrix.
///
/// Returns the lower triangular matrix L such that Σ = L·Lᵀ.
/// The result is flattened row-major.
///
/// # Arguments
/// * `matrix` - Flattened row-major correlation/covariance matrix
/// * `n` - Matrix dimension
///
/// # Returns
/// Lower triangular Cholesky factor L, or error if not PSD.
pub fn cholesky_decompose(matrix: &[f64], n: usize) -> Result<Vec<f64>, CorrelationMatrixError> {
    if matrix.len() != n * n {
        return Err(CorrelationMatrixError::InvalidSize {
            expected: n,
            actual: matrix.len(),
        });
    }

    match core_cholesky_decomposition(matrix, n) {
        Ok(l) => Ok(l),
        Err(CholeskyError::NotPositiveDefinite { row, .. }) => {
            Err(CorrelationMatrixError::NotPositiveSemiDefinite { row })
        }
        Err(CholeskyError::Singular { row, .. }) => {
            Err(CorrelationMatrixError::NotPositiveSemiDefinite { row })
        }
        Err(CholeskyError::DimensionMismatch { expected, actual }) => {
            Err(CorrelationMatrixError::InvalidSize { expected, actual })
        }
        // CholeskyError is non-exhaustive; handle any future variants
        Err(_) => Err(CorrelationMatrixError::NotPositiveSemiDefinite { row: 0 }),
    }
}

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
    ///
    /// # Arguments
    /// * `volatility` - Factor volatility, clamped to [0.01, 2.0]
    /// * `mean_reversion` - Mean reversion speed, clamped to [0.0, 10.0]
    #[must_use]
    pub fn single_factor(volatility: f64, mean_reversion: f64) -> Self {
        FactorSpec::SingleFactor {
            volatility: volatility.clamp(0.01, 2.0),
            mean_reversion: mean_reversion.clamp(0.0, 10.0),
        }
    }

    /// Create a two-factor specification.
    ///
    /// # Arguments
    /// * `prepay_vol` - Prepayment factor volatility, clamped to [0.01, 2.0]
    /// * `credit_vol` - Credit factor volatility, clamped to [0.01, 2.0]
    /// * `correlation` - Correlation between factors, clamped to [-0.99, 0.99]
    #[must_use]
    pub fn two_factor(prepay_vol: f64, credit_vol: f64, correlation: f64) -> Self {
        FactorSpec::TwoFactor {
            prepay_vol: prepay_vol.clamp(0.01, 2.0),
            credit_vol: credit_vol.clamp(0.01, 2.0),
            correlation: correlation.clamp(-0.99, 0.99),
        }
    }

    /// Build a factor model from this specification.
    #[must_use]
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
    /// * `volatility` - Factor volatility, clamped to [0.01, 2.0]
    /// * `mean_reversion` - Mean reversion speed (0 = random walk), clamped to [0.0, 10.0]
    #[must_use]
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
    /// * `prepay_vol` - Prepayment factor volatility, clamped to [0.01, 2.0]
    /// * `credit_vol` - Credit factor volatility, clamped to [0.01, 2.0]
    /// * `correlation` - Correlation between factors (typically negative for RMBS), clamped to [-0.99, 0.99]
    #[must_use]
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
    ///
    /// Uses: prepay_vol=0.20, credit_vol=0.25, correlation=-0.30
    #[must_use]
    pub fn rmbs_standard() -> Self {
        Self::new(0.20, 0.25, -0.30)
    }

    /// Standard CLO calibration.
    ///
    /// Uses: prepay_vol=0.15, credit_vol=0.30, correlation=-0.20
    #[must_use]
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
/// Uses Cholesky decomposition internally for generating correlated factors.
///
/// # Correlation Matrix Requirements
///
/// The correlation matrix must be:
/// - Symmetric: ρᵢⱼ = ρⱼᵢ
/// - Unit diagonal: ρᵢᵢ = 1
/// - Positive semi-definite: All eigenvalues ≥ 0
///
/// Invalid matrices will be replaced with the identity matrix.
/// Use [`MultiFactorModel::try_new`] for explicit validation.
#[derive(Clone, Debug)]
pub struct MultiFactorModel {
    num_factors: usize,
    volatilities: Vec<f64>,
    correlation_matrix: Vec<f64>,
    /// Cholesky factor L (lower triangular, row-major) for correlated sampling.
    /// For generating correlated factors from independent normals z:
    /// correlated_z = L · z
    cholesky_factor: Vec<f64>,
}

impl MultiFactorModel {
    /// Create a multi-factor model.
    ///
    /// If the correlation matrix is invalid (wrong size, not PSD, etc.),
    /// falls back to an identity correlation matrix (uncorrelated factors).
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors (must be ≥ 1)
    /// * `volatilities` - Factor volatilities (one per factor)
    /// * `correlations` - Correlation matrix (flattened row-major, n×n values)
    #[must_use]
    pub fn new(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> Self {
        // Try validated construction first, fall back to identity on error
        Self::try_new(num_factors, volatilities.clone(), correlations)
            .unwrap_or_else(|_| Self::uncorrelated(num_factors, volatilities))
    }

    /// Create a multi-factor model with validation.
    ///
    /// Returns an error if the correlation matrix is invalid.
    ///
    /// # Arguments
    /// * `num_factors` - Number of factors (must be ≥ 1)
    /// * `volatilities` - Factor volatilities (one per factor)
    /// * `correlations` - Correlation matrix (flattened row-major, n×n values)
    ///
    /// # Errors
    /// Returns [`CorrelationMatrixError`] if the matrix is invalid.
    pub fn try_new(
        num_factors: usize,
        volatilities: Vec<f64>,
        correlations: Vec<f64>,
    ) -> Result<Self, CorrelationMatrixError> {
        let n = num_factors.max(1);

        // Validate or create default volatilities
        let vols = if volatilities.len() == n {
            volatilities.iter().map(|v| v.clamp(0.01, 10.0)).collect()
        } else {
            vec![1.0; n]
        };

        // Validate correlation matrix
        if correlations.len() != n * n {
            return Err(CorrelationMatrixError::InvalidSize {
                expected: n,
                actual: correlations.len(),
            });
        }

        validate_correlation_matrix(&correlations, n)?;

        // Compute Cholesky decomposition
        let cholesky = cholesky_decompose(&correlations, n)?;

        Ok(Self {
            num_factors: n,
            volatilities: vols,
            correlation_matrix: correlations,
            cholesky_factor: cholesky,
        })
    }

    /// Create an uncorrelated (identity) multi-factor model.
    ///
    /// All factors are independent (correlation = 0 for i ≠ j).
    #[must_use]
    pub fn uncorrelated(num_factors: usize, volatilities: Vec<f64>) -> Self {
        let n = num_factors.max(1);

        let vols = if volatilities.len() == n {
            volatilities.iter().map(|v| v.clamp(0.01, 10.0)).collect()
        } else {
            vec![1.0; n]
        };

        // Identity correlation matrix
        let mut corrs = vec![0.0; n * n];
        for i in 0..n {
            corrs[i * n + i] = 1.0;
        }

        // Identity is its own Cholesky factor
        let cholesky = corrs.clone();

        Self {
            num_factors: n,
            volatilities: vols,
            correlation_matrix: corrs,
            cholesky_factor: cholesky,
        }
    }

    /// Get the Cholesky factor L for correlated factor generation.
    ///
    /// To generate n correlated factors from n independent standard normals z:
    /// ```text
    /// correlated_z = L · z
    /// factor_i = correlated_z[i] * volatility[i]
    /// ```
    #[must_use]
    pub fn cholesky_factor(&self) -> &[f64] {
        &self.cholesky_factor
    }

    /// Generate correlated factor values from independent standard normal draws.
    ///
    /// # Arguments
    /// * `independent_z` - Vector of n independent standard normal values
    ///
    /// # Returns
    /// Vector of n correlated factor values (scaled by volatilities).
    #[must_use]
    pub fn generate_correlated_factors(&self, independent_z: &[f64]) -> Vec<f64> {
        let n = self.num_factors;
        let mut result = vec![0.0; n];

        // Apply Cholesky: correlated_z = L · z
        // Loop index `i` is needed for both result indexing and inner loop bound
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let mut sum = 0.0;
            for j in 0..=i {
                sum += self.cholesky_factor[i * n + j] * independent_z.get(j).copied().unwrap_or(0.0);
            }
            // Scale by volatility
            result[i] = sum * self.volatilities[i];
        }

        result
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
        // For a single z draw, return the factor value using the Cholesky factor.
        // Note: For truly correlated factor generation, use generate_correlated_factors()
        // with all independent z values at once.
        //
        // This method computes: factor[i] = L[i,i] * z * volatility[i]
        // which gives the contribution from the independent component.
        if factor_index < self.num_factors {
            let n = self.num_factors;
            let l_ii = self.cholesky_factor[factor_index * n + factor_index];
            z * l_ii * self.volatilities[factor_index]
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

    // ========== Correlation Matrix Validation Tests ==========

    #[test]
    fn test_validate_identity_matrix() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        assert!(validate_correlation_matrix(&identity, 2).is_ok());
    }

    #[test]
    fn test_validate_valid_correlation_matrix() {
        // Valid 3x3 correlation matrix
        let corr = vec![
            1.0, 0.5, 0.3,
            0.5, 1.0, 0.4,
            0.3, 0.4, 1.0,
        ];
        assert!(validate_correlation_matrix(&corr, 3).is_ok());
    }

    #[test]
    fn test_validate_invalid_size() {
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let result = validate_correlation_matrix(&corr, 3);
        assert!(matches!(result, Err(CorrelationMatrixError::InvalidSize { .. })));
    }

    #[test]
    fn test_validate_diagonal_not_one() {
        let corr = vec![0.9, 0.5, 0.5, 1.0];
        let result = validate_correlation_matrix(&corr, 2);
        assert!(matches!(result, Err(CorrelationMatrixError::DiagonalNotOne { index: 0, .. })));
    }

    #[test]
    fn test_validate_not_symmetric() {
        let corr = vec![1.0, 0.5, 0.3, 1.0]; // Off-diagonals don't match
        let result = validate_correlation_matrix(&corr, 2);
        assert!(matches!(result, Err(CorrelationMatrixError::NotSymmetric { .. })));
    }

    #[test]
    fn test_validate_not_psd() {
        // Non-PSD matrix: high correlations that violate PSD constraint
        let corr = vec![
            1.0, 0.9, 0.9,
            0.9, 1.0, -0.5,
            0.9, -0.5, 1.0,
        ];
        let result = validate_correlation_matrix(&corr, 3);
        assert!(matches!(result, Err(CorrelationMatrixError::NotPositiveSemiDefinite { .. })));
    }

    #[test]
    fn test_validate_out_of_bounds() {
        let corr = vec![1.0, 1.5, 1.5, 1.0];
        let result = validate_correlation_matrix(&corr, 2);
        assert!(matches!(result, Err(CorrelationMatrixError::OutOfBounds { .. })));
    }

    // ========== Cholesky Decomposition Tests ==========

    #[test]
    fn test_cholesky_identity() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        let l = cholesky_decompose(&identity, 2).expect("identity matrix should decompose");

        // Identity is its own Cholesky factor
        assert!((l[0] - 1.0).abs() < 1e-10);
        assert!(l[1].abs() < 1e-10);
        assert!(l[2].abs() < 1e-10);
        assert!((l[3] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_2x2() {
        // [[1, 0.6], [0.6, 1]]
        let corr = vec![1.0, 0.6, 0.6, 1.0];
        let l = cholesky_decompose(&corr, 2).expect("2x2 correlation matrix should decompose");

        // L should be [[1, 0], [0.6, 0.8]]
        assert!((l[0] - 1.0).abs() < 1e-10);
        assert!(l[1].abs() < 1e-10);
        assert!((l[2] - 0.6).abs() < 1e-10);
        assert!((l[3] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_reconstructs_original() {
        let corr = vec![
            1.0, 0.5, 0.3,
            0.5, 1.0, 0.4,
            0.3, 0.4, 1.0,
        ];
        let l = cholesky_decompose(&corr, 3).expect("3x3 correlation matrix should decompose");

        // Verify L * L^T = original
        let n = 3;
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..n {
                    sum += l[i * n + k] * l[j * n + k];
                }
                assert!(
                    (sum - corr[i * n + j]).abs() < 1e-10,
                    "LLᵀ[{},{}] = {} but expected {}",
                    i, j, sum, corr[i * n + j]
                );
            }
        }
    }

    // ========== MultiFactorModel Tests ==========

    #[test]
    fn test_multi_factor_valid_matrix() {
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let vols = vec![0.2, 0.3];
        let model = MultiFactorModel::try_new(2, vols, corr)
            .expect("valid 2x2 correlation matrix should create model");

        assert_eq!(model.num_factors(), 2);
        assert!(model.cholesky_factor().len() == 4);
    }

    #[test]
    fn test_multi_factor_invalid_falls_back_to_identity() {
        // Invalid matrix (not symmetric)
        let corr = vec![1.0, 0.5, 0.3, 1.0];
        let vols = vec![0.2, 0.3];
        let model = MultiFactorModel::new(2, vols, corr);

        // Should fall back to identity
        let corr_matrix = model.correlation_matrix();
        assert!((corr_matrix[0] - 1.0).abs() < 1e-10);
        assert!(corr_matrix[1].abs() < 1e-10);
        assert!(corr_matrix[2].abs() < 1e-10);
        assert!((corr_matrix[3] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_multi_factor_uncorrelated() {
        let model = MultiFactorModel::uncorrelated(3, vec![0.1, 0.2, 0.3]);

        assert_eq!(model.num_factors(), 3);

        // Check identity correlation
        let corr = model.correlation_matrix();
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((corr[i * 3 + j] - expected).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_generate_correlated_factors() {
        let corr = vec![1.0, 0.6, 0.6, 1.0];
        let vols = vec![1.0, 1.0]; // Unit volatilities for easy verification
        let model = MultiFactorModel::try_new(2, vols, corr)
            .expect("correlated model should create successfully");

        // Generate with independent z = [1.0, 0.0]
        let factors = model.generate_correlated_factors(&[1.0, 0.0]);

        // With Cholesky [[1,0],[0.6,0.8]] and z=[1,0]:
        // correlated_z = [1*1 + 0*0, 0.6*1 + 0.8*0] = [1, 0.6]
        assert!((factors[0] - 1.0).abs() < 1e-10);
        assert!((factors[1] - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_generate_correlated_factors_with_volatility() {
        let corr = vec![1.0, 0.0, 0.0, 1.0]; // Identity
        let vols = vec![0.2, 0.3];
        let model = MultiFactorModel::try_new(2, vols, corr)
            .expect("identity correlation model should create successfully");

        let factors = model.generate_correlated_factors(&[1.0, 1.0]);

        // With identity correlation: factors = z * vol
        assert!((factors[0] - 0.2).abs() < 1e-10);
        assert!((factors[1] - 0.3).abs() < 1e-10);
    }
}
