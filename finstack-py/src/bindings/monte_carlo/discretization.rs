//! Discretisation scheme bindings (parameter-holding wrappers).

use pyo3::prelude::*;

/// Exact (log-normal) GBM discretisation.
#[pyclass(name = "ExactGbm", module = "finstack.monte_carlo", frozen)]
pub struct PyExactGbm;

#[pymethods]
impl PyExactGbm {
    #[new]
    fn new() -> Self {
        Self
    }
    fn __repr__(&self) -> String {
        "ExactGbm()".to_string()
    }
}

/// Exact multi-asset GBM discretisation.
#[pyclass(name = "ExactMultiGbm", module = "finstack.monte_carlo", frozen)]
pub struct PyExactMultiGbm;

#[pymethods]
impl PyExactMultiGbm {
    #[new]
    fn new() -> Self {
        Self
    }
    fn __repr__(&self) -> String {
        "ExactMultiGbm()".to_string()
    }
}

/// Euler-Maruyama discretisation.
#[pyclass(name = "EulerMaruyama", module = "finstack.monte_carlo", frozen)]
pub struct PyEulerMaruyama;

#[pymethods]
impl PyEulerMaruyama {
    #[new]
    fn new() -> Self {
        Self
    }
    fn __repr__(&self) -> String {
        "EulerMaruyama()".to_string()
    }
}

/// Log-Euler discretisation.
#[pyclass(name = "LogEuler", module = "finstack.monte_carlo", frozen)]
pub struct PyLogEuler;

#[pymethods]
impl PyLogEuler {
    #[new]
    fn new() -> Self {
        Self
    }
    fn __repr__(&self) -> String {
        "LogEuler()".to_string()
    }
}

/// Milstein discretisation.
#[pyclass(name = "Milstein", module = "finstack.monte_carlo", frozen)]
pub struct PyMilstein;

#[pymethods]
impl PyMilstein {
    #[new]
    fn new() -> Self {
        Self
    }
    fn __repr__(&self) -> String {
        "Milstein()".to_string()
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExactGbm>()?;
    m.add_class::<PyExactMultiGbm>()?;
    m.add_class::<PyEulerMaruyama>()?;
    m.add_class::<PyLogEuler>()?;
    m.add_class::<PyMilstein>()?;
    Ok(())
}
