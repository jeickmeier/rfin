//! Python bindings for interpolation styles.

use finstack_core::market_data::interp::{InterpStyle, ExtrapolationPolicy};
use pyo3::prelude::*;

/// Interpolation method for curve construction.
///
/// Different interpolation methods provide different trade-offs between
/// smoothness, speed, and financial properties (like arbitrage-freedom).
///
/// Attributes:
///     Linear: Linear interpolation on values (fast, simple)
///     LogLinear: Linear interpolation on log values (constant zero rate)
///     MonotoneConvex: Hagan-West monotone convex interpolation (shape-preserving)
///     CubicHermite: Monotone cubic Hermite spline (smooth, C1 continuous)
///     FlatForward: Piecewise constant forward rates
///
/// Examples:
///     >>> from rfin.market_data import InterpStyle, DiscountCurve
///     >>> from rfin import Date
///     
///     # Use monotone convex for yield curves
///     >>> curve = DiscountCurve(
///     ...     id="USD-OIS",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 2.0],
///     ...     discount_factors=[1.0, 0.98, 0.95],
///     ...     interpolation=InterpStyle.MonotoneConvex
///     ... )
///     
///     # Use log-linear for simple curves
///     >>> simple_curve = DiscountCurve(
///     ...     id="TEST",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0],
///     ...     discount_factors=[1.0, 0.98],
///     ...     interpolation=InterpStyle.LogLinear
///     ... )
#[pyclass(name = "InterpStyle", module = "finstack.market_data", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyInterpStyle {
    /// Linear interpolation on values
    Linear,
    /// Linear interpolation on log values (constant zero rate)
    LogLinear,
    /// Hagan-West monotone convex interpolation
    MonotoneConvex,
    /// Monotone cubic Hermite spline (PCHIP)
    CubicHermite,
    /// Piecewise constant forward rates
    FlatForward,
}

#[pymethods]
impl PyInterpStyle {
    /// Create an InterpStyle from a string representation.
    ///
    /// Args:
    ///     value (str): One of "linear", "log_linear", "monotone_convex",
    ///                  "cubic_hermite", or "flat_forward"
    ///
    /// Returns:
    ///     InterpStyle: The corresponding interpolation style
    ///
    /// Raises:
    ///     ValueError: If the string is not recognized
    #[staticmethod]
    fn from_str(value: &str) -> PyResult<Self> {
        match value.to_lowercase().as_str() {
            "linear" => Ok(PyInterpStyle::Linear),
            "log_linear" | "loglinear" => Ok(PyInterpStyle::LogLinear),
            "monotone_convex" | "monotoneconvex" => Ok(PyInterpStyle::MonotoneConvex),
            "cubic_hermite" | "cubichermite" => Ok(PyInterpStyle::CubicHermite),
            "flat_forward" | "flatforward" => Ok(PyInterpStyle::FlatForward),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown interpolation style: {}",
                value
            ))),
        }
    }

    fn __str__(&self) -> &'static str {
        match self {
            PyInterpStyle::Linear => "Linear",
            PyInterpStyle::LogLinear => "LogLinear",
            PyInterpStyle::MonotoneConvex => "MonotoneConvex",
            PyInterpStyle::CubicHermite => "CubicHermite",
            PyInterpStyle::FlatForward => "FlatForward",
        }
    }

    fn __repr__(&self) -> String {
        format!("InterpStyle.{}", self.__str__())
    }
}

impl PyInterpStyle {
    /// Convert to the core InterpStyle enum
    pub fn to_core(self) -> InterpStyle {
        match self {
            PyInterpStyle::Linear => InterpStyle::Linear,
            PyInterpStyle::LogLinear => InterpStyle::LogLinear,
            PyInterpStyle::MonotoneConvex => InterpStyle::MonotoneConvex,
            PyInterpStyle::CubicHermite => InterpStyle::CubicHermite,
            PyInterpStyle::FlatForward => InterpStyle::FlatFwd,
        }
    }

    /// Create from core InterpStyle
    pub fn from_core(style: InterpStyle) -> Self {
        match style {
            InterpStyle::Linear => PyInterpStyle::Linear,
            InterpStyle::LogLinear => PyInterpStyle::LogLinear,
            InterpStyle::MonotoneConvex => PyInterpStyle::MonotoneConvex,
            InterpStyle::CubicHermite => PyInterpStyle::CubicHermite,
            InterpStyle::FlatFwd => PyInterpStyle::FlatForward,
        }
    }
}

/// Extrapolation policy for curve evaluation beyond the defined knot range.
///
/// Controls how curves behave when queried for values outside the range
/// of input data points. Different policies provide different trade-offs
/// between conservatism and market consistency.
///
/// Attributes:
///     FlatZero: Extend endpoint values (traditional, conservative)
///     FlatForward: Extend forward rates (maintains rate continuity)
///
/// Examples:
///     >>> from rfin.market_data import DiscountCurve, InterpStyle, ExtrapolationPolicy
///     >>> from rfin import Date
///     
///     # Create a curve with flat-zero extrapolation (default)
///     >>> curve_flat_zero = DiscountCurve(
///     ...     id="USD-OIS",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 2.0],
///     ...     discount_factors=[1.0, 0.98, 0.95],
///     ...     extrapolation=ExtrapolationPolicy.FlatZero
///     ... )
///     
///     # Create a curve with flat-forward extrapolation
///     >>> curve_flat_fwd = DiscountCurve(
///     ...     id="USD-OIS",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 2.0],
///     ...     discount_factors=[1.0, 0.98, 0.95],
///     ...     extrapolation=ExtrapolationPolicy.FlatForward
///     ... )
#[pyclass(name = "ExtrapolationPolicy", module = "finstack.market_data", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyExtrapolationPolicy {
    /// Extend endpoint values (flat-zero extrapolation)
    FlatZero,
    /// Extend forward rates (flat-forward extrapolation)
    FlatForward,
}

#[pymethods]
impl PyExtrapolationPolicy {
    /// Create an ExtrapolationPolicy from a string representation.
    ///
    /// Args:
    ///     value (str): One of "flat_zero" or "flat_forward"
    ///
    /// Returns:
    ///     ExtrapolationPolicy: The corresponding extrapolation policy
    ///
    /// Raises:
    ///     ValueError: If the string is not recognized
    #[staticmethod]
    fn from_str(value: &str) -> PyResult<Self> {
        match value.to_lowercase().as_str() {
            "flat_zero" | "flatzero" => Ok(PyExtrapolationPolicy::FlatZero),
            "flat_forward" | "flatforward" => Ok(PyExtrapolationPolicy::FlatForward),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown extrapolation policy: {}",
                value
            ))),
        }
    }

    fn __str__(&self) -> &'static str {
        match self {
            PyExtrapolationPolicy::FlatZero => "FlatZero",
            PyExtrapolationPolicy::FlatForward => "FlatForward",
        }
    }

    fn __repr__(&self) -> String {
        format!("ExtrapolationPolicy.{}", self.__str__())
    }
}

impl PyExtrapolationPolicy {
    /// Convert to the core ExtrapolationPolicy enum
    pub fn to_core(self) -> ExtrapolationPolicy {
        match self {
            PyExtrapolationPolicy::FlatZero => ExtrapolationPolicy::FlatZero,
            PyExtrapolationPolicy::FlatForward => ExtrapolationPolicy::FlatForward,
        }
    }

    /// Create from core ExtrapolationPolicy
    pub fn from_core(policy: ExtrapolationPolicy) -> Self {
        match policy {
            ExtrapolationPolicy::FlatZero => PyExtrapolationPolicy::FlatZero,
            ExtrapolationPolicy::FlatForward => PyExtrapolationPolicy::FlatForward,
        }
    }
}
