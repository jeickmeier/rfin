mod callable;
mod distributions;
mod integration;
mod solver;

use crate::core::common::reexport::reexport_from_submodule;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::collections::HashSet;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "math")?;
    module.setattr(
        "__doc__",
        concat!(
            "Mathematical utilities from finstack-core (distributions, integration, solvers).\n\n",
            "This module aggregates bindings for common mathematical routines:\n",
            "- distributions: binomial probabilities and related logarithms\n",
            "- integration: Simpson/trapezoidal rules and Gauss-Legendre/Hermite quadrature\n",
            "- solver: Newton, Brent, and a hybrid strategy for root finding\n",
            "All functions accept Python callables where appropriate and return floats."
        ),
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let dist_exports = distributions::register(py, &module)?;
    reexport_from_submodule(&module, "distributions", &dist_exports)?;
    exports.extend(dist_exports.iter().copied());

    let integration_exports = integration::register(py, &module)?;
    reexport_from_submodule(&module, "integration", &integration_exports)?;
    exports.extend(integration_exports.iter().copied());

    let solver_exports = solver::register(py, &module)?;
    reexport_from_submodule(&module, "solver", &solver_exports)?;
    exports.extend(solver_exports.iter().copied());

    exports = {
        let mut seen = HashSet::new();
        exports
            .into_iter()
            .filter(|item| seen.insert(*item))
            .collect()
    };
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
