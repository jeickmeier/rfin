use finstack_core::math::solver::{BracketHint, BrentSolver, NewtonSolver, Solver};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use crate::errors::core_to_py;

use super::callable::CallableAdapter;

/// Domain-specific hints for initial bracket sizing in Brent's method.
///
/// Different financial quantities have typical ranges that can dramatically
/// improve convergence speed when the bracket is appropriately sized.
///
/// Examples:
///     >>> from finstack.core.math.solver import BracketHint
///     >>> hint = BracketHint.IMPLIED_VOL
///     >>> print(hint.to_bracket_size())
///     0.2
#[pyclass(
    name = "BracketHint",
    module = "finstack.core.math.solver",
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyBracketHint {
    inner: BracketHint,
}

#[pymethods]
impl PyBracketHint {
    /// Create a custom bracket hint with a specified size.
    ///
    /// Args:
    ///     size (float): Custom bracket size.
    ///
    /// Returns:
    ///     BracketHint: Custom bracket hint.
    #[staticmethod]
    fn custom(size: f64) -> Self {
        Self {
            inner: BracketHint::Custom(size),
        }
    }

    /// Implied volatility hint: sigma typically in [0.01, 2.0], initial bracket +/-0.2.
    #[classattr]
    const IMPLIED_VOL: PyBracketHint = PyBracketHint {
        inner: BracketHint::ImpliedVol,
    };

    /// Interest rate hint: r typically in [-0.05, 0.30], initial bracket +/-0.02.
    #[classattr]
    const RATE: PyBracketHint = PyBracketHint {
        inner: BracketHint::Rate,
    };

    /// Credit spread hint: spread typically in [0, 0.05], initial bracket +/-0.005.
    #[classattr]
    const SPREAD: PyBracketHint = PyBracketHint {
        inner: BracketHint::Spread,
    };

    /// Yield-to-maturity hint: similar to rates, initial bracket +/-0.02.
    #[classattr]
    const YTM: PyBracketHint = PyBracketHint {
        inner: BracketHint::Ytm,
    };

    /// Internal Rate of Return hint: typically in [-0.5, 1.0], initial bracket +/-0.5.
    #[classattr]
    const XIRR: PyBracketHint = PyBracketHint {
        inner: BracketHint::Xirr,
    };

    /// Convert hint to its corresponding initial bracket size.
    ///
    /// Returns:
    ///     float: Bracket size for this hint.
    fn to_bracket_size(&self) -> f64 {
        self.inner.to_bracket_size()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            BracketHint::ImpliedVol => "BracketHint.IMPLIED_VOL".to_string(),
            BracketHint::Rate => "BracketHint.RATE".to_string(),
            BracketHint::Spread => "BracketHint.SPREAD".to_string(),
            BracketHint::Ytm => "BracketHint.YTM".to_string(),
            BracketHint::Xirr => "BracketHint.XIRR".to_string(),
            BracketHint::Custom(size) => format!("BracketHint.custom({size})"),
            _ => format!("BracketHint.custom({})", self.inner.to_bracket_size()),
        }
    }
}

#[pyclass(name = "NewtonSolver", module = "finstack.core.math.solver")]
/// Newton-Raphson root finder with automatic finite-difference derivatives.
///
/// Best used when a good initial guess is available and the derivative is
/// reasonably well-behaved near the root.
///
/// Examples:
///     >>> from finstack.core.math.solver import NewtonSolver
///     >>> solver = NewtonSolver(tolerance=1e-10)
///     >>> round(solver.solve(lambda x: x * x - 2.0, 1.0), 12)
///     1.414213562373
pub struct PyNewtonSolver {
    inner: NewtonSolver,
}

#[pymethods]
impl PyNewtonSolver {
    #[new]
    #[pyo3(text_signature = "(*, tolerance=1e-12, max_iterations=50, fd_step=1e-8)")]
    /// Construct a Newton-Raphson solver with optional parameter overrides.
    ///
    /// Args:
    ///     tolerance (float, optional): Absolute tolerance applied to the
    ///         function value and iterate updates. Defaults to 1e-12.
    ///     max_iterations (int, optional): Maximum Newton iterations before
    ///         giving up. Defaults to 50.
    ///     fd_step (float, optional): Finite-difference step used to estimate
    ///         the derivative. Defaults to 1e-8.
    pub fn py_new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        fd_step: Option<f64>,
    ) -> Self {
        let mut inner = NewtonSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(iter) = max_iterations {
            inner.max_iterations = iter;
        }
        if let Some(step) = fd_step {
            inner.fd_step = step;
        }
        Self { inner }
    }

    #[getter]
    /// Convergence tolerance for the Newton updates.
    ///
    /// Returns:
    ///     float: Residual tolerance.
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[setter]
    /// Update the convergence tolerance.
    ///
    /// Args:
    ///     value (float): New absolute tolerance.
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    #[getter]
    /// Maximum number of Newton iterations.
    ///
    /// Returns:
    ///     int: Iteration cap.
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[setter]
    /// Set the maximum number of Newton iterations.
    ///
    /// Args:
    ///     value (int): New iteration limit.
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    #[getter]
    /// Finite-difference step used to approximate derivatives.
    ///
    /// Returns:
    ///     float: Central difference step size.
    pub fn fd_step(&self) -> f64 {
        self.inner.fd_step
    }

    #[setter]
    /// Set the finite-difference step size for derivative estimation.
    ///
    /// Args:
    ///     value (float): Step size for central differences.
    pub fn set_fd_step(&mut self, value: f64) {
        self.inner.fd_step = value;
    }

    #[getter]
    /// Minimum absolute derivative threshold.
    ///
    /// Newton steps are rejected when the estimated derivative falls below
    /// this absolute guard, preventing division-by-near-zero divergence.
    ///
    /// Returns:
    ///     float: Absolute minimum derivative threshold.
    pub fn min_derivative(&self) -> f64 {
        self.inner.min_derivative
    }

    #[getter]
    /// Relative minimum derivative threshold.
    ///
    /// The derivative must satisfy ``|f'(x)| >= min_derivative_rel * |f(x)|``
    /// in addition to the absolute guard. This prevents steps that are
    /// unreasonably large relative to the function value.
    ///
    /// Returns:
    ///     float: Relative minimum derivative threshold.
    pub fn min_derivative_rel(&self) -> f64 {
        self.inner.min_derivative_rel
    }

    #[pyo3(text_signature = "($self, func, initial_guess)")]
    /// Solve `func(x) = 0` starting from `initial_guess`.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Function whose root is sought.
    ///     initial_guess (float): Starting point for Newton iterations.
    ///
    /// Returns:
    ///     float: Root approximation.
    ///
    /// Raises:
    ///     ValueError: If the solver does not converge or encounters invalid values.
    pub fn solve(&self, func: Bound<'_, PyAny>, initial_guess: f64) -> PyResult<f64> {
        let adapter = CallableAdapter::new(func)?;
        let closure = adapter.closure();
        adapter.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            core_to_py,
        )
    }

    #[pyo3(text_signature = "($self, func, func_derivative, initial_guess)")]
    /// Solve `func(x) = 0` using an analytic derivative for faster convergence.
    ///
    /// Recommended over :meth:`solve` when derivatives are cheaply available,
    /// providing roughly 2x fewer function evaluations per iteration.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Function whose root is sought.
    ///     func_derivative (Callable[[float], float]): Derivative of *func*.
    ///     initial_guess (float): Starting point for Newton iterations.
    ///
    /// Returns:
    ///     float: Root approximation.
    ///
    /// Raises:
    ///     ValueError: If the solver does not converge or encounters invalid values.
    pub fn solve_with_derivative(
        &self,
        func: Bound<'_, PyAny>,
        func_derivative: Bound<'_, PyAny>,
        initial_guess: f64,
    ) -> PyResult<f64> {
        let f_adapter = CallableAdapter::new(func)?;
        let fp_adapter = CallableAdapter::new(func_derivative)?;
        let f_closure = f_adapter.closure();
        let fp_closure = fp_adapter.closure();
        // We need to run both adapters through the panic-catching path.
        // Use the f_adapter for the outer panic catch (it handles both).
        f_adapter.run_core(
            || {
                self.inner
                    .solve_with_derivative(f_closure, fp_closure, initial_guess)
            },
            core_to_py,
        )
    }

    /// String representation showing solver configuration.
    pub fn __repr__(&self) -> String {
        format!(
            "NewtonSolver(tolerance={}, max_iterations={}, fd_step={})",
            self.inner.tolerance, self.inner.max_iterations, self.inner.fd_step
        )
    }
}

#[pyclass(name = "BrentSolver", module = "finstack.core.math.solver")]
/// Brent's bracketing root finder with automatic bracket discovery.
///
/// Prefer when robustness is critical and a bracketing interval can be located
/// near the root.
pub struct PyBrentSolver {
    inner: BrentSolver,
}

#[pymethods]
impl PyBrentSolver {
    #[new]
    #[pyo3(
        text_signature = "(*, tolerance=1e-12, max_iterations=100, bracket_expansion=2.0, initial_bracket_size=None)"
    )]
    /// Construct a Brent solver with optional configuration.
    ///
    /// Args:
    ///     tolerance (float, optional): Absolute tolerance for the root. Defaults to 1e-12.
    ///     max_iterations (int, optional): Maximum iterations before aborting. Defaults to 100.
    ///     bracket_expansion (float, optional): Multiplier used when expanding the search interval. Defaults to 2.0.
    ///     initial_bracket_size (float, optional): Symmetric window around the initial guess to seed bracketing.
    pub fn py_new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        bracket_expansion: Option<f64>,
        initial_bracket_size: Option<f64>,
    ) -> Self {
        let mut inner = BrentSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(iter) = max_iterations {
            inner.max_iterations = iter;
        }
        if let Some(expansion) = bracket_expansion {
            inner.bracket_expansion = expansion;
        }
        if let Some(size) = initial_bracket_size {
            inner.initial_bracket_size = Some(size);
        }
        Self { inner }
    }

    #[getter]
    /// Convergence tolerance for the Brent iterations.
    ///
    /// Returns:
    ///     float: Residual tolerance applied to the root.
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[setter]
    /// Set the convergence tolerance.
    ///
    /// Args:
    ///     value (float): New absolute tolerance.
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    #[getter]
    /// Maximum iteration count for Brent's method.
    ///
    /// Returns:
    ///     int: Iteration cap before aborting.
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[setter]
    /// Set the iteration limit.
    ///
    /// Args:
    ///     value (int): New iteration limit.
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    #[getter]
    /// Bracket expansion multiplier used during discovery.
    ///
    /// Returns:
    ///     float: Factor applied when widening the bracket.
    pub fn bracket_expansion(&self) -> f64 {
        self.inner.bracket_expansion
    }

    #[setter]
    /// Set the bracket expansion multiplier.
    ///
    /// Args:
    ///     value (float): Replacement multiplier when widening the bracket.
    pub fn set_bracket_expansion(&mut self, value: f64) {
        self.inner.bracket_expansion = value;
    }

    #[getter]
    /// Initial symmetric bracket size around the guess (if any).
    ///
    /// Returns:
    ///     float | None: Starting half-width for bracketing, or ``None`` for adaptive sizing.
    pub fn initial_bracket_size(&self) -> Option<f64> {
        self.inner.initial_bracket_size
    }

    #[setter]
    /// Override the initial bracket size.
    ///
    /// Args:
    ///     value (float | None): Half-width to apply; ``None`` reverts to adaptive selection.
    pub fn set_initial_bracket_size(&mut self, value: Option<f64>) {
        self.inner.initial_bracket_size = value;
    }

    #[getter]
    /// Hard bounds ``(min, max)`` for the bracket search.
    ///
    /// These limit the domain explored during bracket discovery.
    /// Default is ``(-1e6, 1e6)``.
    ///
    /// Returns:
    ///     tuple[float, float]: ``(bracket_min, bracket_max)`` bounds.
    pub fn bracket_bounds(&self) -> (f64, f64) {
        (self.inner.bracket_min, self.inner.bracket_max)
    }

    #[pyo3(text_signature = "($self, hint)")]
    /// Apply a domain-specific bracket hint for faster convergence.
    ///
    /// Sets the initial bracket size based on a :class:`BracketHint`.
    ///
    /// Args:
    ///     hint (BracketHint): Domain hint such as ``BracketHint.IMPLIED_VOL``.
    pub fn set_bracket_hint(&mut self, hint: &PyBracketHint) {
        self.inner.initial_bracket_size = Some(hint.inner.to_bracket_size());
    }

    #[pyo3(text_signature = "($self, func, initial_guess)")]
    /// Solve `func(x) = 0` using Brent's method.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Function whose root is sought.
    ///     initial_guess (float): Starting point used to seed the bracketing phase.
    ///
    /// Returns:
    ///     float: Root approximation.
    ///
    /// Raises:
    ///     ValueError: If the solver cannot bracket the root or fails to converge.
    pub fn solve(&self, func: Bound<'_, PyAny>, initial_guess: f64) -> PyResult<f64> {
        let adapter = CallableAdapter::new(func)?;
        let closure = adapter.closure();
        adapter.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            core_to_py,
        )
    }

    /// String representation summarising solver parameters.
    pub fn __repr__(&self) -> String {
        format!(
            "BrentSolver(tolerance={}, max_iterations={}, bracket_expansion={}, initial_bracket_size={:?})",
            self.inner.tolerance,
            self.inner.max_iterations,
            self.inner.bracket_expansion,
            self.inner.initial_bracket_size
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "solver")?;
    module.setattr(
        "__doc__",
        concat!(
            "Root-finding solvers from finstack-core (Newton, Brent).\n\n",
            "Use Newton for fast local convergence when a good initial guess is available;\n",
            "use Brent for robust bracketing. All solvers accept Python callables and return float roots."
        ),
    )?;

    module.add_class::<PyNewtonSolver>()?;
    module.add_class::<PyBrentSolver>()?;
    module.add_class::<PyBracketHint>()?;

    let exports = ["NewtonSolver", "BrentSolver", "BracketHint"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
