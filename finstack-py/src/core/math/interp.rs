//! Interpolation and extrapolation enums used by term structures.
//!
//! This module exposes `InterpStyle` and `ExtrapolationPolicy` to Python,
//! providing canonical labels and parsing helpers. Use these values to
//! configure curve/surface behavior between and beyond known knots.
//!
//! This is the canonical location for interpolation types.
use crate::core::common::labels::normalize_label;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};

/// Enumerate interpolation styles available to term structures.
///
/// Parameters
/// ----------
/// None
///     Use class attributes (e.g. :attr:`InterpStyle.LINEAR`) or :py:meth:`InterpStyle.from_name`.
///
/// Returns
/// -------
/// InterpStyle
///     Enum value defining interpolation behaviour.
#[pyclass(module = "finstack.core.math.interp", name = "InterpStyle", frozen)]
#[derive(Clone, Copy, Debug)]
pub struct PyInterpStyle {
    pub inner: InterpStyle,
}

impl PyInterpStyle {
    pub(crate) const fn new(inner: InterpStyle) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            InterpStyle::Linear => "linear",
            InterpStyle::LogLinear => "log_linear",
            InterpStyle::MonotoneConvex => "monotone_convex",
            InterpStyle::CubicHermite => "cubic_hermite",
            InterpStyle::PiecewiseQuadraticForward => "piecewise_quadratic_forward",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyInterpStyle {
    #[classattr]
    const LINEAR: Self = Self {
        inner: InterpStyle::Linear,
    };
    #[classattr]
    const LOG_LINEAR: Self = Self {
        inner: InterpStyle::LogLinear,
    };
    #[classattr]
    const MONOTONE_CONVEX: Self = Self {
        inner: InterpStyle::MonotoneConvex,
    };
    #[classattr]
    const CUBIC_HERMITE: Self = Self {
        inner: InterpStyle::CubicHermite,
    };
    #[classattr]
    const PIECEWISE_QUADRATIC_FORWARD: Self = Self {
        inner: InterpStyle::PiecewiseQuadraticForward,
    };
    #[classattr]
    const FLAT_FWD: Self = Self {
        inner: InterpStyle::LogLinear,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse an interpolation style from a snake-/kebab-case label.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     One of ``"linear"``, ``"log_linear"``, ``"monotone_convex"``,
    ///     ``"cubic_hermite"``, ``"piecewise_quadratic_forward"``, or ``"flat_fwd"``
    ///     (kebab-case forms also accepted).
    ///
    /// Returns
    /// -------
    /// InterpStyle
    ///     Enum value corresponding to ``name``.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "linear" => Ok(Self::new(InterpStyle::Linear)),
            "log_linear" => Ok(Self::new(InterpStyle::LogLinear)),
            "monotone_convex" => Ok(Self::new(InterpStyle::MonotoneConvex)),
            "cubic_hermite" => Ok(Self::new(InterpStyle::CubicHermite)),
            "piecewise_quadratic_forward" => Ok(Self::new(InterpStyle::PiecewiseQuadraticForward)),
            "flat_fwd" => Ok(Self::new(InterpStyle::LogLinear)),
            other => Err(PyValueError::new_err(format!(
                "Unknown interpolation style: {other}"
            ))),
        }
    }

    #[getter]
    /// Snake-case label for this interpolation style.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("InterpStyle('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Enumerate extrapolation policies used when evaluating beyond curve bounds.
///
/// Parameters
/// ----------
/// None
///     Use class attributes (e.g. :attr:`ExtrapolationPolicy.FLAT_ZERO`) or :py:meth:`ExtrapolationPolicy.from_name`.
///
/// Returns
/// -------
/// ExtrapolationPolicy
///     Enum value describing extrapolation behaviour.
#[pyclass(
    module = "finstack.core.math.interp",
    name = "ExtrapolationPolicy",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyExtrapolationPolicy {
    pub inner: ExtrapolationPolicy,
}

impl PyExtrapolationPolicy {
    pub(crate) const fn new(inner: ExtrapolationPolicy) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            ExtrapolationPolicy::FlatZero => "flat_zero",
            ExtrapolationPolicy::FlatForward => "flat_forward",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyExtrapolationPolicy {
    #[classattr]
    const FLAT_ZERO: Self = Self {
        inner: ExtrapolationPolicy::FlatZero,
    };
    #[classattr]
    const FLAT_FORWARD: Self = Self {
        inner: ExtrapolationPolicy::FlatForward,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse an extrapolation policy from a snake-/kebab-case label.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     One of ``"flat_zero"`` or ``"flat_forward"`` (kebab-case forms also accepted).
    ///
    /// Returns
    /// -------
    /// ExtrapolationPolicy
    ///     Enum value corresponding to ``name``.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "flat_zero" => Ok(Self::new(ExtrapolationPolicy::FlatZero)),
            "flat_forward" => Ok(Self::new(ExtrapolationPolicy::FlatForward)),
            other => Err(PyValueError::new_err(format!(
                "Unknown extrapolation policy: {other}"
            ))),
        }
    }

    #[getter]
    /// Snake-case label for this extrapolation policy.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("ExtrapolationPolicy('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "interp")?;
    module.setattr(
        "__doc__",
        "Interpolation and extrapolation methods for term structures.",
    )?;
    module.add_class::<PyInterpStyle>()?;
    module.add_class::<PyExtrapolationPolicy>()?;
    let exports = ["InterpStyle", "ExtrapolationPolicy"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}

pub(crate) fn parse_interp(style: Option<&str>, default: InterpStyle) -> PyResult<InterpStyle> {
    // Helper used by bindings to parse interpolation labels.
    match style {
        Some(name) => match normalize_label(name).as_str() {
            "linear" => Ok(InterpStyle::Linear),
            "log_linear" => Ok(InterpStyle::LogLinear),
            "monotone_convex" => Ok(InterpStyle::MonotoneConvex),
            "cubic_hermite" => Ok(InterpStyle::CubicHermite),
            "piecewise_quadratic_forward" => Ok(InterpStyle::PiecewiseQuadraticForward),
            "flat_fwd" => Ok(InterpStyle::LogLinear),
            other => Err(PyValueError::new_err(format!(
                "Unknown interpolation style: {other}"
            ))),
        },
        None => Ok(default),
    }
}

pub(crate) fn parse_extrapolation(policy: Option<&str>) -> PyResult<ExtrapolationPolicy> {
    // Helper used by bindings to parse extrapolation policy labels.
    match policy {
        Some(name) => match normalize_label(name).as_str() {
            "flat_zero" => Ok(ExtrapolationPolicy::FlatZero),
            "flat_forward" => Ok(ExtrapolationPolicy::FlatForward),
            other => Err(PyValueError::new_err(format!(
                "Unknown extrapolation policy: {other}"
            ))),
        },
        None => Ok(ExtrapolationPolicy::FlatZero),
    }
}
