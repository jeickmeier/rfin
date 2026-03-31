//! Python bindings for factor models.

use finstack_correlation::{
    FactorModel, FactorSpec, MultiFactorModel, SingleFactorModel, TwoFactorModel,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::ValidationError;

// ---------------------------------------------------------------------------
// SingleFactorModel
// ---------------------------------------------------------------------------

/// Single-factor model (common market factor).
///
/// Models all correlation through a single systematic factor.
#[pyclass(
    name = "SingleFactorModel",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PySingleFactorModel {
    pub(crate) inner: SingleFactorModel,
}

#[pymethods]
impl PySingleFactorModel {
    /// Create a single-factor model.
    ///
    /// Parameters
    /// ----------
    /// volatility : float
    ///     Factor volatility (clamped to [0.01, 2.0]).
    /// mean_reversion : float
    ///     Mean reversion speed (clamped to [0.0, 10.0]).
    #[new]
    fn new(volatility: f64, mean_reversion: f64) -> Self {
        Self {
            inner: SingleFactorModel::new(volatility, mean_reversion),
        }
    }

    /// Factor volatility.
    #[getter]
    fn volatility(&self) -> f64 {
        self.inner.volatility()
    }

    /// Mean reversion speed.
    #[getter]
    fn mean_reversion(&self) -> f64 {
        self.inner.mean_reversion()
    }

    /// Number of factors (always 1).
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    fn factor_names(&self) -> Vec<&'static str> {
        self.inner.factor_names()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Diagonal factor contribution for a single z draw.
    fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    fn __repr__(&self) -> String {
        format!(
            "SingleFactorModel(vol={:.4}, mr={:.4})",
            self.inner.volatility(),
            self.inner.mean_reversion()
        )
    }
}

// ---------------------------------------------------------------------------
// TwoFactorModel
// ---------------------------------------------------------------------------

/// Two-factor model for prepayment and credit.
///
/// Captures the empirical negative correlation between prepayment and default.
#[pyclass(
    name = "TwoFactorModel",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyTwoFactorModel {
    pub(crate) inner: TwoFactorModel,
}

#[pymethods]
impl PyTwoFactorModel {
    /// Create a two-factor model.
    ///
    /// Parameters
    /// ----------
    /// prepay_vol : float
    ///     Prepayment factor volatility (clamped to [0.01, 2.0]).
    /// credit_vol : float
    ///     Credit factor volatility (clamped to [0.01, 2.0]).
    /// correlation : float
    ///     Correlation between factors (clamped to [-0.99, 0.99]).
    #[new]
    fn new(prepay_vol: f64, credit_vol: f64, correlation: f64) -> Self {
        Self {
            inner: TwoFactorModel::new(prepay_vol, credit_vol, correlation),
        }
    }

    /// Standard RMBS calibration (prepay=0.20, credit=0.25, corr=-0.30).
    #[staticmethod]
    fn rmbs_standard() -> Self {
        Self {
            inner: TwoFactorModel::rmbs_standard(),
        }
    }

    /// Standard CLO calibration (prepay=0.15, credit=0.30, corr=-0.20).
    #[staticmethod]
    fn clo_standard() -> Self {
        Self {
            inner: TwoFactorModel::clo_standard(),
        }
    }

    /// Prepayment factor volatility.
    #[getter]
    fn prepay_vol(&self) -> f64 {
        self.inner.prepay_vol()
    }

    /// Credit factor volatility.
    #[getter]
    fn credit_vol(&self) -> f64 {
        self.inner.credit_vol()
    }

    /// Factor correlation.
    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Cholesky L[1][0] coefficient.
    #[getter]
    fn cholesky_l10(&self) -> f64 {
        self.inner.cholesky_l10()
    }

    /// Cholesky L[1][1] coefficient.
    #[getter]
    fn cholesky_l11(&self) -> f64 {
        self.inner.cholesky_l11()
    }

    /// Number of factors (always 2).
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    fn factor_names(&self) -> Vec<&'static str> {
        self.inner.factor_names()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Diagonal factor contribution for a single z draw.
    fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    fn __repr__(&self) -> String {
        format!(
            "TwoFactorModel(prepay_vol={:.4}, credit_vol={:.4}, corr={:.4})",
            self.inner.prepay_vol(),
            self.inner.credit_vol(),
            self.inner.correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// MultiFactorModel
// ---------------------------------------------------------------------------

/// Multi-factor model with custom correlation structure.
///
/// Supports arbitrary number of factors with a validated correlation matrix.
/// Uses pivoted Cholesky decomposition for correlated factor generation.
#[pyclass(
    name = "MultiFactorModel",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyMultiFactorModel {
    pub(crate) inner: MultiFactorModel,
}

#[pymethods]
impl PyMultiFactorModel {
    /// Create a multi-factor model with validation.
    ///
    /// Parameters
    /// ----------
    /// num_factors : int
    ///     Number of factors (must be >= 1).
    /// volatilities : list[float]
    ///     Factor volatilities (one per factor).
    /// correlations : list[float]
    ///     Correlation matrix (flattened row-major, n×n values).
    ///
    /// Raises
    /// ------
    /// ValidationError
    ///     If the correlation matrix is invalid.
    #[new]
    fn new(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> PyResult<Self> {
        let inner = MultiFactorModel::new(num_factors, volatilities, correlations)
            .map_err(|e| ValidationError::new_err(format!("Invalid correlation matrix: {e}")))?;
        Ok(Self { inner })
    }

    /// Create an uncorrelated (identity) multi-factor model.
    ///
    /// Parameters
    /// ----------
    /// num_factors : int
    ///     Number of factors.
    /// volatilities : list[float]
    ///     Factor volatilities.
    #[staticmethod]
    fn uncorrelated(num_factors: usize, volatilities: Vec<f64>) -> Self {
        Self {
            inner: MultiFactorModel::uncorrelated(num_factors, volatilities),
        }
    }

    /// Generate correlated factor values from independent standard normal draws.
    ///
    /// Parameters
    /// ----------
    /// independent_z : list[float]
    ///     Vector of n independent standard normal values.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Vector of n correlated factor values (scaled by volatilities).
    fn generate_correlated_factors(&self, independent_z: Vec<f64>) -> Vec<f64> {
        self.inner.generate_correlated_factors(&independent_z)
    }

    /// Number of factors.
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    fn factor_names(&self) -> Vec<&'static str> {
        self.inner.factor_names()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Diagonal factor contribution for a single z draw.
    fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    /// Cholesky factor matrix (flattened row-major).
    fn cholesky_factor_matrix(&self) -> Vec<f64> {
        self.inner.cholesky_factor().factor_matrix().to_vec()
    }

    fn __repr__(&self) -> String {
        format!("MultiFactorModel(num_factors={})", self.inner.num_factors())
    }
}

// ---------------------------------------------------------------------------
// FactorSpec
// ---------------------------------------------------------------------------

/// Factor model specification for configuration and serialization.
#[pyclass(
    name = "FactorSpec",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyFactorSpec {
    pub(crate) inner: FactorSpec,
}

#[pymethods]
impl PyFactorSpec {
    /// Create a single factor specification.
    ///
    /// Parameters
    /// ----------
    /// volatility : float
    ///     Factor volatility (clamped to [0.01, 2.0]).
    /// mean_reversion : float
    ///     Mean reversion speed (clamped to [0.0, 10.0]).
    #[staticmethod]
    fn single_factor(volatility: f64, mean_reversion: f64) -> Self {
        Self {
            inner: FactorSpec::single_factor(volatility, mean_reversion),
        }
    }

    /// Create a two-factor specification.
    ///
    /// Parameters
    /// ----------
    /// prepay_vol : float
    ///     Prepayment factor volatility.
    /// credit_vol : float
    ///     Credit factor volatility.
    /// correlation : float
    ///     Correlation between factors.
    #[staticmethod]
    fn two_factor(prepay_vol: f64, credit_vol: f64, correlation: f64) -> Self {
        Self {
            inner: FactorSpec::two_factor(prepay_vol, credit_vol, correlation),
        }
    }

    /// Number of factors implied by this specification.
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Serialization failed: {e}"))
        })
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: FactorSpec = serde_json::from_str(json).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Deserialization failed: {e}"))
        })?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    m: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    m.add_class::<PySingleFactorModel>()?;
    m.add_class::<PyTwoFactorModel>()?;
    m.add_class::<PyMultiFactorModel>()?;
    m.add_class::<PyFactorSpec>()?;

    Ok(vec![
        "SingleFactorModel",
        "TwoFactorModel",
        "MultiFactorModel",
        "FactorSpec",
    ])
}
