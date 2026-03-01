use crate::errors::core_to_py;
use finstack_core::math::solver_multi::{
    LevenbergMarquardtSolver, LmSolution, LmStats, LmTerminationReason, MultiSolver,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use super::callable::VectorCallableAdapter;

/// Termination reason for the Levenberg-Marquardt solver.
///
/// Indicates why the solver stopped iterating. Use the class attributes
/// to compare against a returned :attr:`LmStats.termination_reason`.
///
/// Examples:
///     >>> from finstack.core.math.solver_multi import LmTerminationReason
///     >>> reason = LmTerminationReason.CONVERGED_RESIDUAL_NORM
#[pyclass(
    module = "finstack.core.math.solver_multi",
    name = "LmTerminationReason",
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyLmTerminationReason {
    inner: LmTerminationReason,
}

#[pymethods]
impl PyLmTerminationReason {
    /// Residual norm fell below the configured tolerance.
    #[classattr]
    const CONVERGED_RESIDUAL_NORM: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::ConvergedResidualNorm,
    };

    /// Relative residual reduction fell below the configured tolerance.
    #[classattr]
    const CONVERGED_RELATIVE_REDUCTION: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::ConvergedRelativeReduction,
    };

    /// Gradient norm fell below the configured tolerance.
    #[classattr]
    const CONVERGED_GRADIENT: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::ConvergedGradient,
    };

    /// Parameter update step became smaller than ``min_step_size``.
    #[classattr]
    const STEP_TOO_SMALL: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::StepTooSmall,
    };

    /// Solver exhausted the iteration budget.
    #[classattr]
    const MAX_ITERATIONS: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::MaxIterations,
    };

    /// Solver encountered an unrecoverable numerical failure.
    #[classattr]
    const NUMERICAL_FAILURE: PyLmTerminationReason = PyLmTerminationReason {
        inner: LmTerminationReason::NumericalFailure,
    };

    fn __repr__(&self) -> String {
        match &self.inner {
            LmTerminationReason::ConvergedResidualNorm => {
                "LmTerminationReason.CONVERGED_RESIDUAL_NORM".to_string()
            }
            LmTerminationReason::ConvergedRelativeReduction => {
                "LmTerminationReason.CONVERGED_RELATIVE_REDUCTION".to_string()
            }
            LmTerminationReason::ConvergedGradient => {
                "LmTerminationReason.CONVERGED_GRADIENT".to_string()
            }
            LmTerminationReason::StepTooSmall => "LmTerminationReason.STEP_TOO_SMALL".to_string(),
            LmTerminationReason::MaxIterations => "LmTerminationReason.MAX_ITERATIONS".to_string(),
            LmTerminationReason::NumericalFailure => {
                "LmTerminationReason.NUMERICAL_FAILURE".to_string()
            }
        }
    }
}

impl From<LmTerminationReason> for PyLmTerminationReason {
    fn from(inner: LmTerminationReason) -> Self {
        Self { inner }
    }
}

/// Solver statistics for diagnostics and monitoring.
///
/// Provides detailed information about solver convergence behaviour
/// including iteration counts, residual norms, and the termination reason.
///
/// Examples:
///     >>> from finstack.core.math.solver_multi import LevenbergMarquardtSolver
///     >>> solver = LevenbergMarquardtSolver(tolerance=1e-10)
///     >>> solution = solver.solve_system_with_stats(
///     ...     lambda p: [p[0] - 3.0, p[1] - 2.0],
///     ...     [0.0, 0.0], 2
///     ... )
///     >>> print(solution.stats.iterations)
#[pyclass(
    module = "finstack.core.math.solver_multi",
    name = "LmStats",
    from_py_object
)]
#[derive(Clone)]
pub struct PyLmStats {
    inner: LmStats,
}

#[pymethods]
impl PyLmStats {
    /// Number of accepted LM iterations.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    /// Total residual evaluations performed (including Jacobian probes).
    #[getter]
    fn residual_evals(&self) -> usize {
        self.inner.residual_evals
    }

    /// Total Jacobian evaluations performed.
    #[getter]
    fn jacobian_evals(&self) -> usize {
        self.inner.jacobian_evals
    }

    /// Reason why the solver terminated.
    #[getter]
    fn termination_reason(&self) -> PyLmTerminationReason {
        self.inner.termination_reason.clone().into()
    }

    /// Final residual norm when termination occurred.
    #[getter]
    fn final_residual_norm(&self) -> f64 {
        self.inner.final_residual_norm
    }

    /// Norm of the final accepted (or attempted) step.
    #[getter]
    fn final_step_norm(&self) -> f64 {
        self.inner.final_step_norm
    }

    /// Final damping parameter (lambda) at termination.
    #[getter]
    fn lambda_final(&self) -> f64 {
        self.inner.lambda_final
    }

    /// Number of times lambda hit the upper or lower bound.
    #[getter]
    fn lambda_bound_hits(&self) -> usize {
        self.inner.lambda_bound_hits
    }

    fn __repr__(&self) -> String {
        format!(
            "LmStats(iterations={}, residual_evals={}, jacobian_evals={}, termination_reason={}, final_residual_norm={:.6e}, final_step_norm={:.6e}, lambda_final={:.6e}, lambda_bound_hits={})",
            self.inner.iterations,
            self.inner.residual_evals,
            self.inner.jacobian_evals,
            self.termination_reason().__repr__(),
            self.inner.final_residual_norm,
            self.inner.final_step_norm,
            self.inner.lambda_final,
            self.inner.lambda_bound_hits,
        )
    }
}

impl From<LmStats> for PyLmStats {
    fn from(inner: LmStats) -> Self {
        Self { inner }
    }
}

/// Solution vector plus solver statistics.
///
/// Returned by :meth:`LevenbergMarquardtSolver.solve_system_with_stats`
/// to provide both the solved parameters and diagnostic information.
///
/// Examples:
///     >>> solution = solver.solve_system_with_stats(residuals, [0.0, 0.0], 2)
///     >>> print(solution.params)    # solved parameters
///     >>> print(solution.stats)     # diagnostics
#[pyclass(
    module = "finstack.core.math.solver_multi",
    name = "LmSolution",
    from_py_object
)]
#[derive(Clone)]
pub struct PyLmSolution {
    inner: LmSolution,
}

#[pymethods]
impl PyLmSolution {
    /// Solved parameter vector.
    #[getter]
    fn params(&self) -> Vec<f64> {
        self.inner.params.clone()
    }

    /// Detailed solver diagnostics.
    #[getter]
    fn stats(&self) -> PyLmStats {
        self.inner.stats.clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "LmSolution(params={:?}, stats={})",
            self.inner.params,
            self.stats().__repr__(),
        )
    }
}

impl From<LmSolution> for PyLmSolution {
    fn from(inner: LmSolution) -> Self {
        Self { inner }
    }
}

#[pyclass(
    module = "finstack.core.math.solver_multi",
    name = "LevenbergMarquardtSolver"
)]
pub struct PyLevenbergMarquardtSolver {
    inner: LevenbergMarquardtSolver,
}

#[pymethods]
impl PyLevenbergMarquardtSolver {
    #[new]
    #[pyo3(
        signature = (
            tolerance=None,
            max_iterations=None,
            lambda_init=None,
            lambda_factor=None,
            fd_step=None,
            min_step_size=None
        )
    )]
    fn py_new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        lambda_init: Option<f64>,
        lambda_factor: Option<f64>,
        fd_step: Option<f64>,
        min_step_size: Option<f64>,
    ) -> Self {
        let mut inner = LevenbergMarquardtSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(max_iter) = max_iterations {
            inner.max_iterations = max_iter;
        }
        if let Some(lambda) = lambda_init {
            inner.lambda_init = lambda;
        }
        if let Some(factor) = lambda_factor {
            inner.lambda_factor = factor;
        }
        if let Some(step) = fd_step {
            inner.fd_step = step;
        }
        if let Some(min_step) = min_step_size {
            inner.min_step_size = min_step;
        }
        Self { inner }
    }

    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[setter]
    fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[setter]
    fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    #[getter]
    fn lambda_init(&self) -> f64 {
        self.inner.lambda_init
    }

    #[setter]
    fn set_lambda_init(&mut self, value: f64) {
        self.inner.lambda_init = value;
    }

    #[getter]
    fn lambda_factor(&self) -> f64 {
        self.inner.lambda_factor
    }

    #[setter]
    fn set_lambda_factor(&mut self, value: f64) {
        self.inner.lambda_factor = value;
    }

    #[getter]
    fn fd_step(&self) -> f64 {
        self.inner.fd_step
    }

    #[setter]
    fn set_fd_step(&mut self, value: f64) {
        self.inner.fd_step = value;
    }

    #[getter]
    fn min_step_size(&self) -> f64 {
        self.inner.min_step_size
    }

    #[setter]
    fn set_min_step_size(&mut self, value: f64) {
        self.inner.min_step_size = value;
    }

    #[pyo3(signature = (objective, initial, bounds=None))]
    fn minimize(
        &self,
        objective: Bound<'_, PyAny>,
        initial: Vec<f64>,
        bounds: Option<Vec<(f64, f64)>>,
    ) -> PyResult<Vec<f64>> {
        let adapter = VectorCallableAdapter::new(objective)?;
        let closure = adapter.objective_closure();
        let bounds_storage = bounds.and_then(|b| if b.is_empty() { None } else { Some(b) });
        let bounds_slice = bounds_storage.as_deref();
        adapter.run_core(
            || self.inner.minimize(closure, &initial, bounds_slice),
            core_to_py,
        )
    }

    /// Solve system of equations using Levenberg-Marquardt.
    ///
    /// Parameters
    /// ----------
    /// residuals : callable
    ///     Function that takes params (list[float]) and returns residuals (list[float]).
    /// initial : list[float]
    ///     Initial parameter guess.
    /// n_residuals : int
    ///     Number of residuals (equations) in the system.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Parameter vector that minimizes ||residuals(params)||^2.
    fn solve_system(
        &self,
        residuals: Bound<'_, PyAny>,
        initial: Vec<f64>,
        n_residuals: usize,
    ) -> PyResult<Vec<f64>> {
        let adapter = VectorCallableAdapter::new(residuals)?;
        let residual_closure = adapter.residual_closure();
        adapter.run_core(
            || {
                self.inner
                    .solve_system_with_dim_stats(residual_closure, &initial, n_residuals)
                    .map(|solution| solution.params)
            },
            core_to_py,
        )
    }

    /// Solve system of equations and return full diagnostics.
    ///
    /// Like :meth:`solve_system`, but returns an :class:`LmSolution` containing
    /// both the solved parameters and an :class:`LmStats` object with
    /// convergence diagnostics.
    ///
    /// Parameters
    /// ----------
    /// residuals : callable
    ///     Function that takes params (list[float]) and returns residuals (list[float]).
    /// initial : list[float]
    ///     Initial parameter guess.
    /// n_residuals : int
    ///     Number of residuals (equations) in the system.
    ///
    /// Returns
    /// -------
    /// LmSolution
    ///     Solution object with ``params`` and ``stats`` attributes.
    fn solve_system_with_stats(
        &self,
        residuals: Bound<'_, PyAny>,
        initial: Vec<f64>,
        n_residuals: usize,
    ) -> PyResult<PyLmSolution> {
        let adapter = VectorCallableAdapter::new(residuals)?;
        let residual_closure = adapter.residual_closure();
        adapter.run_core(
            || {
                self.inner
                    .solve_system_with_dim_stats(residual_closure, &initial, n_residuals)
                    .map(PyLmSolution::from)
            },
            core_to_py,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "LevenbergMarquardtSolver(tolerance={}, max_iterations={}, lambda_init={}, lambda_factor={}, fd_step={}, min_step_size={})",
            self.inner.tolerance,
            self.inner.max_iterations,
            self.inner.lambda_init,
            self.inner.lambda_factor,
            self.inner.fd_step,
            self.inner.min_step_size
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "solver_multi")?;
    module.setattr(
        "__doc__",
        "Multi-dimensional solvers (Levenberg-Marquardt) for calibration tasks.",
    )?;
    module.add_class::<PyLevenbergMarquardtSolver>()?;
    module.add_class::<PyLmTerminationReason>()?;
    module.add_class::<PyLmStats>()?;
    module.add_class::<PyLmSolution>()?;
    let exports = [
        "LevenbergMarquardtSolver",
        "LmTerminationReason",
        "LmStats",
        "LmSolution",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
