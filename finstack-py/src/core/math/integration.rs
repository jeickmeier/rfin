use finstack_core::math::integration as core_integration;
use finstack_core::math::integration::GaussHermiteQuadrature;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

use crate::errors::core_to_py;

use super::callable::CallableAdapter;

#[pyclass(
    name = "GaussHermiteQuadrature",
    module = "finstack.core.math.integration"
)]
/// Pre-computed Gauss-Hermite nodes and weights for standard-normal integration.
///
/// Provides 5, 7, or 10-point quadrature rules for evaluating expectations under
/// the standard normal density. Useful for approximating integrals of the form
/// ``E[f(Z)]`` where ``Z ~ N(0, 1)``.
///
/// Examples:
///     >>> from finstack.core.math.integration import GaussHermiteQuadrature
///     >>> quad = GaussHermiteQuadrature.order_7()
///     >>> quad.order
///     7
///     >>> quad.integrate(lambda x: x * x)
///     1.0
pub struct PyGaussHermiteQuadrature {
    inner: GaussHermiteQuadrature,
}

#[pymethods]
impl PyGaussHermiteQuadrature {
    #[classmethod]
    #[pyo3(text_signature = "(/)")]
    /// Create the 5-point Gauss-Hermite quadrature rule.
    ///
    /// Returns:
    ///     GaussHermiteQuadrature: Quadrature rule with five evaluation points and
    ///     corresponding weights.
    pub fn order_5(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: GaussHermiteQuadrature::order_5(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(/)")]
    /// Create the 7-point Gauss-Hermite quadrature rule.
    ///
    /// Returns:
    ///     GaussHermiteQuadrature: Quadrature rule with seven evaluation points and
    ///     corresponding weights.
    pub fn order_7(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: GaussHermiteQuadrature::order_7(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(/)")]
    /// Create the 10-point Gauss-Hermite quadrature rule.
    ///
    /// Returns:
    ///     GaussHermiteQuadrature: Quadrature rule with ten evaluation points and
    ///     corresponding weights.
    pub fn order_10(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: GaussHermiteQuadrature::order_10(),
        }
    }

    #[getter]
    /// Number of quadrature points in this rule.
    ///
    /// Returns:
    ///     int: Quadrature order (length of `points`).
    pub fn order(&self) -> usize {
        self.inner.points.len()
    }

    #[getter]
    /// Quadrature evaluation points scaled for the standard normal density.
    ///
    /// Returns:
    ///     list[float]: Array of evaluation abscissae.
    pub fn points(&self) -> Vec<f64> {
        self.inner.points.to_vec()
    }

    #[getter]
    /// Quadrature weights paired with `points`.
    ///
    /// Returns:
    ///     list[float]: Non-negative weights that sum to ``sqrt(pi)``.
    pub fn weights(&self) -> Vec<f64> {
        self.inner.weights.to_vec()
    }

    #[pyo3(text_signature = "($self, func)")]
    /// Integrate a callable against the standard normal distribution.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Python function accepting a point `x`
    ///         and returning `f(x)`.
    ///
    /// Returns:
    ///     float: Approximation of ``∫ f(x) φ(x) dx`` over ``(-∞, ∞)``.
    ///
    /// Examples:
    ///     >>> quad = GaussHermiteQuadrature.order_5()
    ///     >>> quad.integrate(lambda x: x ** 2)
    ///     1.0
    pub fn integrate(&self, func: Bound<'_, PyAny>) -> PyResult<f64> {
        let adapter = CallableAdapter::new(func)?;
        let closure = adapter.closure();
        adapter.run_value(|| self.inner.integrate(closure))
    }

    #[pyo3(text_signature = "($self, func, /, tolerance)")]
    /// Integrate with automatic refinement of the quadrature order.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Callable evaluated at each quadrature point.
    ///     tolerance (float): Maximum acceptable difference when upgrading the
    ///         quadrature order.
    ///
    /// Returns:
    ///     float: Refined integral estimate meeting the requested tolerance.
    pub fn integrate_adaptive(&self, func: Bound<'_, PyAny>, tolerance: f64) -> PyResult<f64> {
        let adapter = CallableAdapter::new(func)?;
        let closure = adapter.closure();
        adapter.run_value(|| self.inner.integrate_adaptive(closure, tolerance))
    }

    /// Return a string representation that highlights the quadrature order.
    pub fn __repr__(&self) -> String {
        format!("GaussHermiteQuadrature(order={})", self.order())
    }
}

#[pyfunction(name = "simpson_rule", text_signature = "(func, a, b, intervals)")]
/// Simpson's composite rule for integrating a callable on ``[a, b]``.
///
/// Args:
///     func (Callable[[float], float]): Function to evaluate at grid points.
///     a (float): Lower integration bound.
///     b (float): Upper integration bound.
///     intervals (int): Even number of sub-intervals used by Simpson's rule.
///
/// Returns:
///     float: Integral estimate across ``[a, b]``.
///
/// Raises:
///     ValueError: If ``intervals`` is zero or odd.
pub fn simpson_rule_py(func: Bound<'_, PyAny>, a: f64, b: f64, intervals: usize) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || core_integration::simpson_rule(closure, a, b, intervals),
        core_to_py,
    )
}

#[pyfunction(
    name = "adaptive_simpson",
    text_signature = "(func, a, b, tol, max_depth)"
)]
/// Adaptive Simpson integration with automatic refinement.
///
/// Args:
///     func (Callable[[float], float]): Callable evaluated at requested points.
///     a (float): Lower bound of the integration interval.
///     b (float): Upper bound of the integration interval.
///     tol (float): Target absolute error tolerance.
///     max_depth (int): Maximum recursion depth controlling refinement.
///
/// Returns:
///     float: Integral estimate satisfying the tolerance when possible.
pub fn adaptive_simpson_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || core_integration::adaptive_simpson(closure, a, b, tol, max_depth),
        core_to_py,
    )
}

#[pyfunction(
    name = "adaptive_quadrature",
    text_signature = "(func, a, b, tol, max_depth)"
)]
/// Adaptive Simpson quadrature (alias of `adaptive_simpson`).
///
/// Args:
///     func (Callable[[float], float]): Callable evaluated at requested points.
///     a (float): Lower bound of the integration interval.
///     b (float): Upper bound of the integration interval.
///     tol (float): Target absolute error tolerance.
///     max_depth (int): Maximum recursion depth for refinement.
///
/// Returns:
///     float: Integral estimate identical to `adaptive_simpson`.
pub fn adaptive_quadrature_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> PyResult<f64> {
    adaptive_simpson_py(func, a, b, tol, max_depth)
}

#[pyfunction(
    name = "gauss_legendre_integrate",
    text_signature = "(func, a, b, order)"
)]
/// Gauss-Legendre quadrature on ``[a, b]`` with fixed order.
///
/// Args:
///     func (Callable[[float], float]): Function evaluated at node locations.
///     a (float): Lower integration bound.
///     b (float): Upper integration bound.
///     order (int): Supported quadrature order (2, 4, 8, or 16).
///
/// Returns:
///     float: Integral approximation over ``[a, b]``.
pub fn gauss_legendre_integrate_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    order: usize,
) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || core_integration::gauss_legendre_integrate(closure, a, b, order),
        core_to_py,
    )
}

#[pyfunction(
    name = "gauss_legendre_integrate_composite",
    text_signature = "(func, a, b, order, panels)"
)]
/// Composite Gauss-Legendre quadrature with multiple panels.
///
/// Args:
///     func (Callable[[float], float]): Function evaluated for each sub-interval.
///     a (float): Lower bound.
///     b (float): Upper bound.
///     order (int): Individual panel quadrature order.
///     panels (int): Number of sub-intervals to tile across ``[a, b]``.
///
/// Returns:
///     float: Integrated value across the full interval.
pub fn gauss_legendre_integrate_composite_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    order: usize,
    panels: usize,
) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || core_integration::gauss_legendre_integrate_composite(closure, a, b, order, panels),
        core_to_py,
    )
}

#[pyfunction(
    name = "gauss_legendre_integrate_adaptive",
    text_signature = "(func, a, b, order, tol, max_depth)"
)]
/// Adaptive Gauss-Legendre quadrature with panel refinement.
///
/// Args:
///     func (Callable[[float], float]): Function to integrate.
///     a (float): Lower bound of the integration domain.
///     b (float): Upper bound of the integration domain.
///     order (int): Base quadrature order (2, 4, 8, or 16).
///     tol (float): Error tolerance governing panel refinement.
///     max_depth (int): Maximum number of recursive refinements.
///
/// Returns:
///     float: Integral approximation with adaptive panel splitting.
pub fn gauss_legendre_integrate_adaptive_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    order: usize,
    tol: f64,
    max_depth: usize,
) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || {
            core_integration::gauss_legendre_integrate_adaptive(
                closure, a, b, order, tol, max_depth,
            )
        },
        core_to_py,
    )
}

#[pyfunction(name = "trapezoidal_rule", text_signature = "(func, a, b, intervals)")]
/// Trapezoidal rule for integrating a callable on ``[a, b]``.
///
/// Args:
///     func (Callable[[float], float]): Function evaluated at grid points.
///     a (float): Lower bound of the integration interval.
///     b (float): Upper bound of the integration interval.
///     intervals (int): Number of sub-intervals to apply.
///
/// Returns:
///     float: Integral approximation from the trapezoidal rule.
pub fn trapezoidal_rule_py(
    func: Bound<'_, PyAny>,
    a: f64,
    b: f64,
    intervals: usize,
) -> PyResult<f64> {
    let adapter = CallableAdapter::new(func)?;
    let closure = adapter.closure();
    adapter.run_core(
        || core_integration::trapezoidal_rule(closure, a, b, intervals),
        core_to_py,
    )
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "integration")?;
    module.setattr(
        "__doc__",
        concat!(
            "Numerical integration helpers mirroring finstack-core capabilities.\n\n",
            "Includes Simpson's rule, trapezoidal rule, Gauss-Legendre quadrature (fixed, composite, adaptive), and\n",
            "Gauss-Hermite quadrature for standard-normal expectations. All functions accept Python callables and\n",
            "return float approximations."
        ),
    )?;

    module.add_class::<PyGaussHermiteQuadrature>()?;
    module.add_function(wrap_pyfunction!(simpson_rule_py, &module)?)?;
    module.add_function(wrap_pyfunction!(adaptive_simpson_py, &module)?)?;
    module.add_function(wrap_pyfunction!(adaptive_quadrature_py, &module)?)?;
    module.add_function(wrap_pyfunction!(gauss_legendre_integrate_py, &module)?)?;
    module.add_function(wrap_pyfunction!(
        gauss_legendre_integrate_composite_py,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(
        gauss_legendre_integrate_adaptive_py,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(trapezoidal_rule_py, &module)?)?;

    let exports = [
        "GaussHermiteQuadrature",
        "simpson_rule",
        "adaptive_simpson",
        "adaptive_quadrature",
        "gauss_legendre_integrate",
        "gauss_legendre_integrate_composite",
        "gauss_legendre_integrate_adaptive",
        "trapezoidal_rule",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
