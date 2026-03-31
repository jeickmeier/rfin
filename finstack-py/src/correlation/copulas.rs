//! Python bindings for copula models.

use finstack_correlation::{
    Copula, CopulaSpec, GaussianCopula, MultiFactorCopula, RandomFactorLoadingCopula,
    StudentTCopula,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::ValidationError;

// ---------------------------------------------------------------------------
// GaussianCopula
// ---------------------------------------------------------------------------

/// One-factor Gaussian copula (market standard).
///
/// The industry-standard model for credit index tranche pricing.
/// Zero tail dependence; use with base correlation to capture the smile.
#[pyclass(
    name = "GaussianCopula",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyGaussianCopula {
    pub(crate) inner: GaussianCopula,
}

#[pymethods]
impl PyGaussianCopula {
    /// Create a Gaussian copula with default quadrature (20 points).
    #[new]
    #[pyo3(signature = (quadrature_order=None))]
    fn new(quadrature_order: Option<u8>) -> Self {
        let inner = match quadrature_order {
            Some(order) => GaussianCopula::with_quadrature_order(order),
            None => GaussianCopula::new(),
        };
        Self { inner }
    }

    /// Conditional default probability P(default | Z).
    ///
    /// Parameters
    /// ----------
    /// default_threshold : float
    ///     Φ⁻¹(PD) threshold.
    /// factor_realization : list[float]
    ///     Systematic factor value(s). Length must be 1.
    /// correlation : float
    ///     Asset correlation parameter.
    ///
    /// Returns
    /// -------
    /// float
    ///     Conditional default probability in [0, 1].
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (always 1).
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Lower-tail dependence (always 0 for Gaussian).
    fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    fn __repr__(&self) -> String {
        format!("GaussianCopula(num_factors={})", self.inner.num_factors())
    }
}

// ---------------------------------------------------------------------------
// StudentTCopula
// ---------------------------------------------------------------------------

/// Student-t copula with configurable degrees of freedom.
///
/// Captures tail dependence — joint extreme defaults cluster more than
/// Gaussian predicts. Lower df = more tail dependence.
#[pyclass(
    name = "StudentTCopula",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyStudentTCopula {
    pub(crate) inner: StudentTCopula,
}

#[pymethods]
impl PyStudentTCopula {
    /// Create a Student-t copula.
    ///
    /// Parameters
    /// ----------
    /// degrees_of_freedom : float
    ///     Must be > 2 for finite variance. Typical: 4–10.
    /// quadrature_order : int, optional
    ///     Number of quadrature points (default 20).
    #[new]
    #[pyo3(signature = (degrees_of_freedom, quadrature_order=None))]
    fn new(degrees_of_freedom: f64, quadrature_order: Option<u8>) -> PyResult<Self> {
        if degrees_of_freedom <= 2.0 {
            return Err(ValidationError::new_err(
                "Student-t degrees_of_freedom must be > 2 for finite variance",
            ));
        }
        let inner = match quadrature_order {
            Some(order) => StudentTCopula::with_quadrature_order(degrees_of_freedom, order),
            None => StudentTCopula::new(degrees_of_freedom),
        };
        Ok(Self { inner })
    }

    /// Degrees of freedom.
    #[getter]
    fn degrees_of_freedom(&self) -> f64 {
        self.inner.df()
    }

    /// Conditional default probability P(default | M).
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (always 1).
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Lower-tail dependence coefficient λ_L.
    fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    fn __repr__(&self) -> String {
        format!(
            "StudentTCopula(df={:.1}, num_factors={})",
            self.inner.df(),
            self.inner.num_factors()
        )
    }
}

// ---------------------------------------------------------------------------
// MultiFactorCopula
// ---------------------------------------------------------------------------

/// Multi-factor Gaussian copula with sector structure.
///
/// Uses a global factor plus sector-specific factors to model
/// intra-sector vs. inter-sector correlation differences.
#[pyclass(
    name = "MultiFactorCopula",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyMultiFactorCopula {
    pub(crate) inner: MultiFactorCopula,
}

#[pymethods]
impl PyMultiFactorCopula {
    /// Create a multi-factor copula.
    ///
    /// Parameters
    /// ----------
    /// num_factors : int
    ///     Number of factors (1 or 2; capped at 2).
    /// global_loading : float, optional
    ///     Loading on global factor (default 0.4).
    /// sector_loading : float, optional
    ///     Loading on sector factor (default 0.3).
    /// sector_fraction : float, optional
    ///     Fraction of total correlation from sector factor (default 0.4).
    #[new]
    #[pyo3(signature = (num_factors, global_loading=None, sector_loading=None, sector_fraction=None))]
    fn new(
        num_factors: usize,
        global_loading: Option<f64>,
        sector_loading: Option<f64>,
        sector_fraction: Option<f64>,
    ) -> Self {
        let inner = match (global_loading, sector_loading, sector_fraction) {
            (Some(gl), Some(sl), Some(sf)) => {
                MultiFactorCopula::with_loadings_and_sector_fraction(num_factors, gl, sl, sf)
            }
            (Some(gl), Some(sl), None) => MultiFactorCopula::with_loadings(num_factors, gl, sl),
            _ => MultiFactorCopula::new(num_factors),
        };
        Self { inner }
    }

    /// Inter-sector correlation (β_G²).
    #[getter]
    fn inter_sector_correlation(&self) -> f64 {
        self.inner.inter_sector_correlation()
    }

    /// Intra-sector correlation (β_G² + β_S²).
    #[getter]
    fn intra_sector_correlation(&self) -> f64 {
        self.inner.intra_sector_correlation()
    }

    /// Decompose total correlation into (global_loading, sector_loading).
    fn decompose_correlation(&self, total_correlation: f64, sector_fraction: f64) -> (f64, f64) {
        self.inner
            .decompose_correlation(total_correlation, sector_fraction)
    }

    /// Conditional default probability P(default | Z_G, Z_S).
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors.
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Lower-tail dependence (always 0 for multi-factor Gaussian).
    fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiFactorCopula(num_factors={}, inter={:.4}, intra={:.4})",
            self.inner.num_factors(),
            self.inner.inter_sector_correlation(),
            self.inner.intra_sector_correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// RandomFactorLoadingCopula
// ---------------------------------------------------------------------------

/// Random Factor Loading copula with stochastic correlation.
///
/// Models correlation itself as random, capturing increased correlation
/// during market stress. Important for senior tranche pricing.
#[pyclass(
    name = "RandomFactorLoadingCopula",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyRandomFactorLoadingCopula {
    pub(crate) inner: RandomFactorLoadingCopula,
}

#[pymethods]
impl PyRandomFactorLoadingCopula {
    /// Create an RFL copula.
    ///
    /// Parameters
    /// ----------
    /// loading_volatility : float
    ///     Volatility of the factor loading (clamped to [0, 0.5]). Typical: 0.05–0.20.
    /// quadrature_order : int, optional
    ///     Number of quadrature points (default 20).
    #[new]
    #[pyo3(signature = (loading_volatility, quadrature_order=None))]
    fn new(loading_volatility: f64, quadrature_order: Option<u8>) -> Self {
        let inner = match quadrature_order {
            Some(order) => {
                RandomFactorLoadingCopula::with_quadrature_order(loading_volatility, order)
            }
            None => RandomFactorLoadingCopula::new(loading_volatility),
        };
        Self { inner }
    }

    /// Loading volatility.
    #[getter]
    fn loading_volatility(&self) -> f64 {
        self.inner.loading_volatility()
    }

    /// Conditional default probability P(default | Z, η).
    fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (2: market + loading shock).
    fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    /// Stress-dependence gauge (monotone proxy, not strict λ_L).
    fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    fn __repr__(&self) -> String {
        format!(
            "RandomFactorLoadingCopula(loading_vol={:.4}, num_factors={})",
            self.inner.loading_volatility(),
            self.inner.num_factors()
        )
    }
}

// ---------------------------------------------------------------------------
// CopulaSpec
// ---------------------------------------------------------------------------

/// Copula model specification for configuration and serialization.
///
/// Allows copula selection and deferred construction.
#[pyclass(
    name = "CopulaSpec",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyCopulaSpec {
    pub(crate) inner: CopulaSpec,
}

#[pymethods]
impl PyCopulaSpec {
    /// Create a Gaussian copula specification.
    #[staticmethod]
    fn gaussian() -> Self {
        Self {
            inner: CopulaSpec::gaussian(),
        }
    }

    /// Create a Student-t copula specification.
    ///
    /// Parameters
    /// ----------
    /// degrees_of_freedom : float
    ///     Must be > 2 for finite variance.
    #[staticmethod]
    fn student_t(degrees_of_freedom: f64) -> PyResult<Self> {
        if degrees_of_freedom <= 2.0 {
            return Err(ValidationError::new_err(
                "Student-t degrees_of_freedom must be > 2 for finite variance",
            ));
        }
        Ok(Self {
            inner: CopulaSpec::student_t(degrees_of_freedom),
        })
    }

    /// Create a Random Factor Loading specification.
    ///
    /// Parameters
    /// ----------
    /// loading_volatility : float
    ///     Volatility of factor loading (clamped to [0, 0.5]).
    #[staticmethod]
    fn random_factor_loading(loading_volatility: f64) -> Self {
        Self {
            inner: CopulaSpec::random_factor_loading(loading_volatility),
        }
    }

    /// Create a multi-factor copula specification.
    ///
    /// Parameters
    /// ----------
    /// num_factors : int
    ///     Number of systematic factors.
    #[staticmethod]
    fn multi_factor(num_factors: usize) -> Self {
        Self {
            inner: CopulaSpec::multi_factor(num_factors),
        }
    }

    /// Whether this is a Gaussian copula specification.
    fn is_gaussian(&self) -> bool {
        self.inner.is_gaussian()
    }

    /// Whether this is a Student-t copula specification.
    fn is_student_t(&self) -> bool {
        self.inner.is_student_t()
    }

    /// Whether this is a Random Factor Loading specification.
    fn is_rfl(&self) -> bool {
        self.inner.is_rfl()
    }

    /// Whether this is a multi-factor specification.
    fn is_multi_factor(&self) -> bool {
        self.inner.is_multi_factor()
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
        let inner: CopulaSpec = serde_json::from_str(json).map_err(|e| {
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
    m.add_class::<PyGaussianCopula>()?;
    m.add_class::<PyStudentTCopula>()?;
    m.add_class::<PyMultiFactorCopula>()?;
    m.add_class::<PyRandomFactorLoadingCopula>()?;
    m.add_class::<PyCopulaSpec>()?;

    Ok(vec![
        "GaussianCopula",
        "StudentTCopula",
        "MultiFactorCopula",
        "RandomFactorLoadingCopula",
        "CopulaSpec",
    ])
}
