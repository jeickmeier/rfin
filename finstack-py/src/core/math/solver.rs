use finstack_core::math::solver::{BrentSolver, HybridSolver, NewtonSolver, Solver};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use crate::core::error::core_to_py;

use super::callable::CallableAdapter;

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

#[pyclass(name = "HybridSolver", module = "finstack.core.math.solver")]
/// Hybrid solver that attempts Newton first, then falls back to Brent if needed.
///
/// Combines the fast convergence of Newton's method with the robustness of
/// Brent's bracketing strategy.
pub struct PyHybridSolver {
    inner: HybridSolver,
    tolerance: f64,
    max_iterations: usize,
}

#[pymethods]
impl PyHybridSolver {
    #[new]
    #[pyo3(text_signature = "(*, tolerance=1e-12, max_iterations=100)")]
    /// Construct a hybrid solver with optional tolerance and iteration limits.
    ///
    /// Args:
    ///     tolerance (float, optional): Shared tolerance applied to both Newton and Brent components. Defaults to 1e-12.
    ///     max_iterations (int, optional): Maximum iterations attempted by each component before giving up. Defaults to 100.
    pub fn py_new(tolerance: Option<f64>, max_iterations: Option<usize>) -> Self {
        let mut inner = HybridSolver::new();
        let tol = tolerance.unwrap_or(1e-12);
        let max_iter = max_iterations.unwrap_or(100);
        inner = inner.with_tolerance(tol);
        inner = inner.with_max_iterations(max_iter);
        Self {
            inner,
            tolerance: tol,
            max_iterations: max_iter,
        }
    }

    #[getter]
    /// Shared convergence tolerance.
    ///
    /// Returns:
    ///     float: Current tolerance used by both solver stages.
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    #[setter]
    /// Update the shared tolerance for the hybrid solver.
    ///
    /// Args:
    ///     value (float): New tolerance applied to Newton and Brent phases.
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner = std::mem::take(&mut self.inner).with_tolerance(value);
        self.tolerance = value;
    }

    #[getter]
    /// Maximum iterations for both solver components.
    ///
    /// Returns:
    ///     int: Shared iteration limit.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    #[setter]
    /// Set the maximum iterations for both solver components.
    ///
    /// Args:
    ///     value (int): New iteration limit applied to both Newton and Brent phases.
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner = std::mem::take(&mut self.inner).with_max_iterations(value);
        self.max_iterations = value;
    }

    #[pyo3(text_signature = "($self, func, initial_guess)")]
    /// Solve `func(x) = 0` using the hybrid strategy.
    ///
    /// Args:
    ///     func (Callable[[float], float]): Function whose root is sought.
    ///     initial_guess (float): Initial guess supplied to the Newton phase.
    ///
    /// Returns:
    ///     float: Root approximation.
    ///
    /// Raises:
    ///     ValueError: If neither Newton nor Brent converges.
    pub fn solve(&self, func: Bound<'_, PyAny>, initial_guess: f64) -> PyResult<f64> {
        let adapter = CallableAdapter::new(func)?;
        let closure = adapter.closure();
        adapter.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            core_to_py,
        )
    }

    /// String representation combining tolerance and iteration settings.
    pub fn __repr__(&self) -> String {
        format!(
            "HybridSolver(tolerance={}, max_iterations={})",
            self.tolerance, self.max_iterations
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
            "Root-finding solvers from finstack-core (Newton, Brent, hybrid).\n\n",
            "Use Newton for fast local convergence when a good initial guess is available;\n",
            "use Brent for robust bracketing; Hybrid tries Newton first then falls back\n",
            "to Brent. All solvers accept Python callables and return float roots."
        ),
    )?;

    module.add_class::<PyNewtonSolver>()?;
    module.add_class::<PyBrentSolver>()?;
    module.add_class::<PyHybridSolver>()?;

    let exports = ["NewtonSolver", "BrentSolver", "HybridSolver"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
