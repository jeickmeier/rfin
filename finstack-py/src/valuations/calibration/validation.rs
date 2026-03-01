use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyBaseCorrelationCurve, PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use crate::errors::core_to_py;
use finstack_valuations::calibration::{CurveValidator, SurfaceValidator, ValidationConfig};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

fn resolve_validation_config(config: Option<PyRef<'_, PyValidationConfig>>) -> ValidationConfig {
    config.map(|cfg| cfg.inner.clone()).unwrap_or_default()
}

#[pyfunction]
#[pyo3(signature = (curve, config=None), text_signature = "(curve, config=None)")]
fn validate_discount_curve(
    curve: &PyDiscountCurve,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    curve.inner.validate(&cfg).map_err(core_to_py)
}

#[pyfunction]
#[pyo3(signature = (curve, config=None), text_signature = "(curve, config=None)")]
fn validate_forward_curve(
    curve: &PyForwardCurve,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    curve.inner.validate(&cfg).map_err(core_to_py)
}

#[pyfunction]
#[pyo3(signature = (curve, config=None), text_signature = "(curve, config=None)")]
fn validate_hazard_curve(
    curve: &PyHazardCurve,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    curve.inner.validate(&cfg).map_err(core_to_py)
}

#[pyfunction]
#[pyo3(signature = (curve, config=None), text_signature = "(curve, config=None)")]
fn validate_inflation_curve(
    curve: &PyInflationCurve,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    curve.inner.validate(&cfg).map_err(core_to_py)
}

#[pyfunction]
#[pyo3(signature = (surface, config=None), text_signature = "(surface, config=None)")]
fn validate_vol_surface(
    surface: &PyVolSurface,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    surface.inner.validate(&cfg).map_err(core_to_py)
}

#[pyfunction]
#[pyo3(signature = (curve, config=None), text_signature = "(curve, config=None)")]
fn validate_base_correlation_curve(
    curve: &PyBaseCorrelationCurve,
    config: Option<PyRef<'_, PyValidationConfig>>,
) -> PyResult<()> {
    let cfg = resolve_validation_config(config);
    curve.inner.validate(&cfg).map_err(core_to_py)
}

/// Configuration for curve and surface validation.
///
/// Controls which validation checks are performed and their tolerance levels.
///
/// Attributes:
///     check_forward_positivity: Enable forward rate positivity check
///     min_forward_rate: Minimum allowed forward rate (can be slightly negative)
///     max_forward_rate: Maximum allowed forward rate
///     check_monotonicity: Enable monotonicity checks
///     check_arbitrage: Enable no-arbitrage constraint checks
///     tolerance: Numerical tolerance for comparisons
///
/// Examples:
///     >>> # Default configuration
///     >>> config = ValidationConfig()
///     >>> config.min_forward_rate
///     -0.01
///
///     >>> # Custom configuration allowing more negative rates
///     >>> config = ValidationConfig(min_forward_rate=-0.05)
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "ValidationConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyValidationConfig {
    pub(crate) inner: ValidationConfig,
}

impl PyValidationConfig {
    pub(crate) fn new(inner: ValidationConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationConfig {
    #[new]
    #[pyo3(
        signature = (
            check_forward_positivity=None,
            min_forward_rate=None,
            max_forward_rate=None,
            check_monotonicity=None,
            check_arbitrage=None,
            tolerance=None,
            max_hazard_rate=None,
            min_cpi_growth=None,
            max_cpi_growth=None,
            min_fwd_inflation=None,
            max_fwd_inflation=None,
            max_volatility=None,
            allow_negative_rates=None,
            lenient_arbitrage=None,
            butterfly_upper_ratio=None,
            butterfly_lower_ratio=None
        ),
        text_signature = "(check_forward_positivity=True, min_forward_rate=-0.01, max_forward_rate=0.50, check_monotonicity=True, check_arbitrage=True, tolerance=1e-10, max_hazard_rate=0.50, min_cpi_growth=-0.10, max_cpi_growth=0.50, min_fwd_inflation=-0.20, max_fwd_inflation=0.50, max_volatility=5.0, allow_negative_rates=False, lenient_arbitrage=False, butterfly_upper_ratio=1.25, butterfly_lower_ratio=0.75)"
    )]
    /// Create validation configuration.
    ///
    /// Args:
    ///     check_forward_positivity: Enable forward rate positivity check
    ///     min_forward_rate: Minimum allowed forward rate (default: -0.01)
    ///     max_forward_rate: Maximum allowed forward rate (default: 0.50)
    ///     check_monotonicity: Enable monotonicity checks
    ///     check_arbitrage: Enable no-arbitrage constraint checks
    ///     tolerance: Numerical tolerance for comparisons (default: 1e-10)
    ///     butterfly_upper_ratio: Upper convexity tolerance for butterfly spread validation (default: 1.25)
    ///     butterfly_lower_ratio: Lower convexity tolerance for butterfly spread validation (default: 0.75)
    ///
    /// Returns:
    ///     ValidationConfig: Configuration object
    ///
    /// Raises:
    ///     ValueError: If parameters are inconsistent
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        check_forward_positivity: Option<bool>,
        min_forward_rate: Option<f64>,
        max_forward_rate: Option<f64>,
        check_monotonicity: Option<bool>,
        check_arbitrage: Option<bool>,
        tolerance: Option<f64>,
        max_hazard_rate: Option<f64>,
        min_cpi_growth: Option<f64>,
        max_cpi_growth: Option<f64>,
        min_fwd_inflation: Option<f64>,
        max_fwd_inflation: Option<f64>,
        max_volatility: Option<f64>,
        allow_negative_rates: Option<bool>,
        lenient_arbitrage: Option<bool>,
        butterfly_upper_ratio: Option<f64>,
        butterfly_lower_ratio: Option<f64>,
    ) -> PyResult<Self> {
        let mut config = ValidationConfig::default();

        if let Some(val) = check_forward_positivity {
            config.check_forward_positivity = val;
        }
        if let Some(val) = min_forward_rate {
            config.min_forward_rate = val;
        }
        if let Some(val) = max_forward_rate {
            config.max_forward_rate = val;
        }
        if let Some(val) = check_monotonicity {
            config.check_monotonicity = val;
        }
        if let Some(val) = check_arbitrage {
            config.check_arbitrage = val;
        }
        if let Some(val) = tolerance {
            config.tolerance = val;
        }

        if let Some(val) = max_hazard_rate {
            config.max_hazard_rate = val;
        }
        if let Some(val) = min_cpi_growth {
            config.min_cpi_growth = val;
        }
        if let Some(val) = max_cpi_growth {
            config.max_cpi_growth = val;
        }

        if let Some(val) = min_fwd_inflation {
            config.min_fwd_inflation = val;
        }
        if let Some(val) = max_fwd_inflation {
            config.max_fwd_inflation = val;
        }

        if let Some(val) = max_volatility {
            config.max_volatility = val;
        }

        if let Some(val) = allow_negative_rates {
            config.allow_negative_rates = val;
        }
        if let Some(val) = lenient_arbitrage {
            config.lenient_arbitrage = val;
        }
        if let Some(val) = butterfly_upper_ratio {
            config.butterfly_upper_ratio = val;
        }
        if let Some(val) = butterfly_lower_ratio {
            config.butterfly_lower_ratio = val;
        }

        config.validate().map_err(core_to_py)?;
        Ok(Self::new(config))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create default validation configuration.
    ///
    /// Returns:
    ///     ValidationConfig: Standard validation settings
    fn standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ValidationConfig::default())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn strict(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ValidationConfig::strict())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn negative_rates(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ValidationConfig::negative_rates())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn lenient(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ValidationConfig::lenient())
    }

    fn with_lenient_arbitrage(&self, lenient: bool) -> Self {
        Self::new(self.inner.clone().with_lenient_arbitrage(lenient))
    }

    /// Whether to check forward rate positivity.
    ///
    /// Returns:
    ///     bool: Forward positivity check enabled
    #[getter]
    fn check_forward_positivity(&self) -> bool {
        self.inner.check_forward_positivity
    }

    /// Minimum allowed forward rate.
    ///
    /// Returns:
    ///     float: Minimum forward rate (typically slightly negative)
    #[getter]
    fn min_forward_rate(&self) -> f64 {
        self.inner.min_forward_rate
    }

    /// Maximum allowed forward rate.
    ///
    /// Returns:
    ///     float: Maximum forward rate
    #[getter]
    fn max_forward_rate(&self) -> f64 {
        self.inner.max_forward_rate
    }

    /// Whether to check monotonicity constraints.
    ///
    /// Returns:
    ///     bool: Monotonicity check enabled
    #[getter]
    fn check_monotonicity(&self) -> bool {
        self.inner.check_monotonicity
    }

    /// Whether to check no-arbitrage constraints.
    ///
    /// Returns:
    ///     bool: No-arbitrage check enabled
    #[getter]
    fn check_arbitrage(&self) -> bool {
        self.inner.check_arbitrage
    }

    /// Numerical tolerance for comparisons.
    ///
    /// Returns:
    ///     float: Tolerance value
    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Maximum allowed hazard rate.
    #[getter]
    fn max_hazard_rate(&self) -> f64 {
        self.inner.max_hazard_rate
    }

    /// Minimum allowed annual CPI growth.
    #[getter]
    fn min_cpi_growth(&self) -> f64 {
        self.inner.min_cpi_growth
    }

    /// Maximum allowed annual CPI growth.
    #[getter]
    fn max_cpi_growth(&self) -> f64 {
        self.inner.max_cpi_growth
    }

    /// Minimum allowed forward inflation.
    #[getter]
    fn min_fwd_inflation(&self) -> f64 {
        self.inner.min_fwd_inflation
    }

    /// Maximum allowed forward inflation.
    #[getter]
    fn max_fwd_inflation(&self) -> f64 {
        self.inner.max_fwd_inflation
    }

    /// Maximum allowed volatility (e.g. 5.0 = 500%).
    #[getter]
    fn max_volatility(&self) -> f64 {
        self.inner.max_volatility
    }

    /// Allow negative-rate environments (DF > 1.0 at short end).
    #[getter]
    fn allow_negative_rates(&self) -> bool {
        self.inner.allow_negative_rates
    }

    /// When true, arbitrage violations produce warnings instead of errors.
    #[getter]
    fn lenient_arbitrage(&self) -> bool {
        self.inner.lenient_arbitrage
    }

    /// Upper convexity tolerance for butterfly spread validation.
    #[getter]
    fn butterfly_upper_ratio(&self) -> f64 {
        self.inner.butterfly_upper_ratio
    }

    /// Lower convexity tolerance for butterfly spread validation.
    #[getter]
    fn butterfly_lower_ratio(&self) -> f64 {
        self.inner.butterfly_lower_ratio
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationConfig(min_forward_rate={:.4}, max_forward_rate={:.2}, tolerance={:.2e}, allow_negative_rates={}, lenient_arbitrage={})",
            self.inner.min_forward_rate,
            self.inner.max_forward_rate,
            self.inner.tolerance,
            self.inner.allow_negative_rates,
            self.inner.lenient_arbitrage
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(pyo3::wrap_pyfunction!(validate_discount_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_forward_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_hazard_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_inflation_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_vol_surface, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(
        validate_base_correlation_curve,
        module
    )?)?;

    module.add_class::<PyValidationConfig>()?;

    let exports = [
        "ValidationConfig",
        "validate_base_correlation_curve",
        "validate_discount_curve",
        "validate_forward_curve",
        "validate_hazard_curve",
        "validate_inflation_curve",
        "validate_vol_surface",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    Ok(exports.to_vec())
}
