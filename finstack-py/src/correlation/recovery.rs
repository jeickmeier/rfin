//! Python bindings for recovery models.

use finstack_correlation::{ConstantRecovery, CorrelatedRecovery, RecoveryModel, RecoverySpec};
use pyo3::prelude::*;
use pyo3::types::PyModule;

// ---------------------------------------------------------------------------
// ConstantRecovery
// ---------------------------------------------------------------------------

/// Constant recovery rate model.
///
/// Recovery is fixed regardless of market conditions.
/// ISDA standard is 40%.
#[pyclass(
    name = "ConstantRecovery",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyConstantRecovery {
    pub(crate) inner: ConstantRecovery,
}

#[pymethods]
impl PyConstantRecovery {
    /// Create a constant recovery model.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Recovery rate (clamped to [0, 1]).
    #[new]
    fn new(rate: f64) -> Self {
        Self {
            inner: ConstantRecovery::new(rate),
        }
    }

    /// ISDA standard recovery rate (40%).
    #[staticmethod]
    fn isda_standard() -> Self {
        Self {
            inner: ConstantRecovery::isda_standard(),
        }
    }

    /// Senior secured recovery rate (55%).
    #[staticmethod]
    fn senior_secured() -> Self {
        Self {
            inner: ConstantRecovery::senior_secured(),
        }
    }

    /// Subordinated debt recovery rate (25%).
    #[staticmethod]
    fn subordinated() -> Self {
        Self {
            inner: ConstantRecovery::subordinated(),
        }
    }

    /// Recovery rate.
    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate()
    }

    /// Expected (unconditional) recovery rate.
    fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Recovery rate conditional on market factor (constant for this model).
    fn conditional_recovery(&self, market_factor: f64) -> f64 {
        self.inner.conditional_recovery(market_factor)
    }

    /// Loss given default = 1 - recovery.
    fn lgd(&self) -> f64 {
        self.inner.lgd()
    }

    /// Conditional LGD given market factor.
    fn conditional_lgd(&self, market_factor: f64) -> f64 {
        self.inner.conditional_lgd(market_factor)
    }

    /// Recovery-rate volatility (0 for constant models).
    fn recovery_volatility(&self) -> f64 {
        self.inner.recovery_volatility()
    }

    /// Whether this model is stochastic (always false for constant).
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    fn __repr__(&self) -> String {
        format!("ConstantRecovery(rate={:.4})", self.inner.rate())
    }
}

// ---------------------------------------------------------------------------
// CorrelatedRecovery
// ---------------------------------------------------------------------------

/// Market-correlated stochastic recovery model (Andersen-Sidenius).
///
/// Recovery varies with the systematic market factor, capturing the
/// empirical negative correlation between defaults and recovery.
#[pyclass(
    name = "CorrelatedRecovery",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyCorrelatedRecovery {
    pub(crate) inner: CorrelatedRecovery,
}

#[pymethods]
impl PyCorrelatedRecovery {
    /// Create a correlated recovery model.
    ///
    /// Parameters
    /// ----------
    /// mean_recovery : float
    ///     Mean recovery rate (clamped to [0.05, 0.95]).
    /// recovery_volatility : float
    ///     Recovery volatility (clamped to [0.0, 0.50]).
    /// factor_correlation : float
    ///     Correlation with market factor (clamped to [-1.0, 1.0]).
    #[new]
    fn new(mean_recovery: f64, recovery_volatility: f64, factor_correlation: f64) -> Self {
        Self {
            inner: CorrelatedRecovery::new(mean_recovery, recovery_volatility, factor_correlation),
        }
    }

    /// Create with custom recovery bounds.
    ///
    /// Parameters
    /// ----------
    /// mean_recovery : float
    ///     Mean recovery rate.
    /// recovery_volatility : float
    ///     Recovery volatility.
    /// factor_correlation : float
    ///     Correlation with market factor.
    /// min_recovery : float
    ///     Recovery floor (clamped to [0.0, 0.5]).
    /// max_recovery : float
    ///     Recovery ceiling (clamped to [0.5, 1.0]).
    #[staticmethod]
    fn with_bounds(
        mean_recovery: f64,
        recovery_volatility: f64,
        factor_correlation: f64,
        min_recovery: f64,
        max_recovery: f64,
    ) -> Self {
        Self {
            inner: CorrelatedRecovery::with_bounds(
                mean_recovery,
                recovery_volatility,
                factor_correlation,
                min_recovery,
                max_recovery,
            ),
        }
    }

    /// Market-standard calibration (mean=40%, vol=25%, corr=-40%).
    #[staticmethod]
    fn market_standard() -> Self {
        Self {
            inner: CorrelatedRecovery::market_standard(),
        }
    }

    /// Conservative calibration (mean=40%, vol=30%, corr=-50%).
    #[staticmethod]
    fn conservative() -> Self {
        Self {
            inner: CorrelatedRecovery::conservative(),
        }
    }

    /// Mean recovery rate.
    #[getter]
    fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// Recovery volatility.
    #[getter]
    fn volatility(&self) -> f64 {
        self.inner.volatility()
    }

    /// Factor correlation.
    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Expected (unconditional) recovery rate.
    fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Recovery rate conditional on market factor.
    fn conditional_recovery(&self, market_factor: f64) -> f64 {
        self.inner.conditional_recovery(market_factor)
    }

    /// Loss given default = 1 - recovery.
    fn lgd(&self) -> f64 {
        self.inner.lgd()
    }

    /// Conditional LGD given market factor.
    fn conditional_lgd(&self, market_factor: f64) -> f64 {
        self.inner.conditional_lgd(market_factor)
    }

    /// Recovery-rate volatility scale.
    fn recovery_volatility(&self) -> f64 {
        self.inner.recovery_volatility()
    }

    /// Whether this model is stochastic.
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str {
        self.inner.model_name()
    }

    fn __repr__(&self) -> String {
        format!(
            "CorrelatedRecovery(mean={:.4}, vol={:.4}, corr={:.4})",
            self.inner.mean(),
            self.inner.volatility(),
            self.inner.correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// RecoverySpec
// ---------------------------------------------------------------------------

/// Recovery model specification for configuration and serialization.
#[pyclass(
    name = "RecoverySpec",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyRecoverySpec {
    pub(crate) inner: RecoverySpec,
}

#[pymethods]
impl PyRecoverySpec {
    /// Create a constant recovery specification.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Recovery rate (clamped to [0.0, 1.0]).
    #[staticmethod]
    fn constant(rate: f64) -> Self {
        Self {
            inner: RecoverySpec::constant(rate),
        }
    }

    /// Create a market-correlated recovery specification.
    ///
    /// Parameters
    /// ----------
    /// mean_recovery : float
    ///     Mean recovery rate.
    /// recovery_volatility : float
    ///     Recovery volatility.
    /// factor_correlation : float
    ///     Correlation with market factor.
    #[staticmethod]
    fn market_correlated(
        mean_recovery: f64,
        recovery_volatility: f64,
        factor_correlation: f64,
    ) -> Self {
        Self {
            inner: RecoverySpec::market_correlated(
                mean_recovery,
                recovery_volatility,
                factor_correlation,
            ),
        }
    }

    /// Market-standard stochastic recovery (mean=40%, vol=25%, corr=-40%).
    #[staticmethod]
    fn market_standard_stochastic() -> Self {
        Self {
            inner: RecoverySpec::market_standard_stochastic(),
        }
    }

    /// Expected recovery rate from specification.
    fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
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
        let inner: RecoverySpec = serde_json::from_str(json).map_err(|e| {
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
    m.add_class::<PyConstantRecovery>()?;
    m.add_class::<PyCorrelatedRecovery>()?;
    m.add_class::<PyRecoverySpec>()?;

    Ok(vec![
        "ConstantRecovery",
        "CorrelatedRecovery",
        "RecoverySpec",
    ])
}
