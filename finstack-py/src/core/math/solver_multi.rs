use crate::errors::core_to_py;
use finstack_core::math::solver_multi::{LevenbergMarquardtSolver, MultiSolver};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use super::callable::VectorCallableAdapter;

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

    fn solve_system(
        &self,
        residuals: Bound<'_, PyAny>,
        initial: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        let adapter = VectorCallableAdapter::new(residuals)?;
        let residual_closure = adapter.residual_closure();
        adapter.run_core(
            || self.inner.solve_system(residual_closure, &initial),
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
    let exports = ["LevenbergMarquardtSolver"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}

