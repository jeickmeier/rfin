use crate::core::error::core_to_py;
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use finstack_valuations::calibration::{
    CurveValidator, SurfaceValidator, ValidationConfig, ValidationError,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_discount_curve(curve: &PyDiscountCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_forward_curve(curve: &PyForwardCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_hazard_curve(curve: &PyHazardCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_inflation_curve(curve: &PyInflationCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(surface)")]
fn validate_vol_surface(surface: &PyVolSurface) -> PyResult<()> {
    surface.inner.validate().map_err(core_to_py)
}

/// Validation error details with constraint information.
///
/// Provides structured information about validation failures including
/// the constraint violated, location, and relevant numerical values.
///
/// Attributes:
///     constraint: Name of the violated constraint (e.g., "monotonicity", "no_arbitrage")
///     location: Location identifier (e.g., curve ID, time point)
///     details: Human-readable description of the violation
///     values: Dictionary of relevant numerical values
///
/// Examples:
///     >>> # Typically constructed by validation logic
///     >>> error = ValidationError(
///     ...     constraint="monotonicity",
///     ...     location="USD-OIS",
///     ...     details="Discount factors not decreasing",
///     ...     values={"t": 2.0, "df": 0.96, "prev_df": 0.95}
///     ... )
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "ValidationError",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyValidationError {
    pub(crate) inner: ValidationError,
}

impl PyValidationError {
    pub(crate) fn new(inner: ValidationError) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationError {
    #[new]
    #[pyo3(text_signature = "(constraint, location, details, values=None)")]
    /// Create a validation error.
    ///
    /// Args:
    ///     constraint: Name of the violated constraint
    ///     location: Location identifier (curve ID, time point, etc.)
    ///     details: Human-readable description
    ///     values: Optional dictionary of relevant numerical values
    ///
    /// Returns:
    ///     ValidationError: Structured validation error
    fn ctor(
        constraint: &str,
        location: &str,
        details: &str,
        values: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut error = ValidationError::new(constraint, location, details);
        
        if let Some(dict) = values {
            for (key, value) in dict.iter() {
                let key_str = key.extract::<String>()?;
                let val_f64 = value.extract::<f64>()?;
                error = error.with_value(key_str, val_f64);
            }
        }
        
        Ok(Self::new(error))
    }

    /// Name of the violated constraint.
    ///
    /// Returns:
    ///     str: Constraint name
    #[getter]
    fn constraint(&self) -> &str {
        &self.inner.constraint
    }

    /// Location identifier for the violation.
    ///
    /// Returns:
    ///     str: Location (curve ID, time point, etc.)
    #[getter]
    fn location(&self) -> &str {
        &self.inner.location
    }

    /// Human-readable description of the violation.
    ///
    /// Returns:
    ///     str: Detailed description
    #[getter]
    fn details(&self) -> &str {
        &self.inner.details
    }

    /// Dictionary of relevant numerical values.
    ///
    /// Returns:
    ///     dict[str, float]: Numerical values related to the error
    #[getter]
    fn values(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.values.iter() {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationError(constraint='{}', location='{}', details='{}')",
            self.inner.constraint, self.inner.location, self.inner.details
        )
    }
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
    frozen
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
            tolerance=None
        ),
        text_signature = "(check_forward_positivity=True, min_forward_rate=-0.01, max_forward_rate=0.50, check_monotonicity=True, check_arbitrage=True, tolerance=1e-10)"
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
    ///
    /// Returns:
    ///     ValidationConfig: Configuration object
    ///
    /// Raises:
    ///     ValueError: If parameters are inconsistent
    fn ctor(
        check_forward_positivity: Option<bool>,
        min_forward_rate: Option<f64>,
        max_forward_rate: Option<f64>,
        check_monotonicity: Option<bool>,
        check_arbitrage: Option<bool>,
        tolerance: Option<f64>,
    ) -> PyResult<Self> {
        let mut config = ValidationConfig::default();
        
        if let Some(val) = check_forward_positivity {
            config.check_forward_positivity = val;
        }
        if let Some(val) = min_forward_rate {
            if val > 0.0 {
                return Err(PyValueError::new_err(
                    "min_forward_rate should be non-positive or slightly negative"
                ));
            }
            config.min_forward_rate = val;
        }
        if let Some(val) = max_forward_rate {
            if val <= 0.0 {
                return Err(PyValueError::new_err("max_forward_rate must be positive"));
            }
            config.max_forward_rate = val;
        }
        if let Some(val) = check_monotonicity {
            config.check_monotonicity = val;
        }
        if let Some(val) = check_arbitrage {
            config.check_arbitrage = val;
        }
        if let Some(val) = tolerance {
            if val <= 0.0 {
                return Err(PyValueError::new_err("tolerance must be positive"));
            }
            config.tolerance = val;
        }
        
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

    fn __repr__(&self) -> String {
        format!(
            "ValidationConfig(min_forward_rate={:.4}, max_forward_rate={:.2}, tolerance={:.2e})",
            self.inner.min_forward_rate, self.inner.max_forward_rate, self.inner.tolerance
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
    
    module.add_class::<PyValidationError>()?;
    module.add_class::<PyValidationConfig>()?;

    let exports = [
        "validate_discount_curve",
        "validate_forward_curve",
        "validate_hazard_curve",
        "validate_inflation_curve",
        "validate_vol_surface",
        "ValidationError",
        "ValidationConfig",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
