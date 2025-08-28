//! Python bindings for interpolation styles.

use finstack_core::market_data::interp::InterpStyle;
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
