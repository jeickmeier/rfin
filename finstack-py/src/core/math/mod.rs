mod callable;
mod distributions;
mod integration;
pub(crate) mod interp;
mod linalg;
mod probability;
mod random;
mod solver;
mod solver_multi;
mod special_functions;
mod stats;
mod summation;

use finstack_core::HashSet;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "math")?;
    module.setattr(
        "__doc__",
        concat!(
            "Mathematical utilities from finstack-core (distributions, integration, solvers, RNGs).\n\n",
            "This module aggregates bindings for common mathematical routines:\n",
            "- distributions: binomial probabilities, PDFs/CDFs, and sampling\n",
            "- probability: correlated Bernoulli distributions and joint probabilities\n",
            "- integration: Simpson/trapezoidal rules and Gauss-Legendre/Hermite quadrature\n",
            "- solver: Newton and Brent root finders\n",
            "- interp: interpolation and extrapolation styles\n",
            "- random: SimpleRng and Box-Muller transforms\n",
            "All functions accept Python callables where appropriate and return floats."
        ),
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let dist_exports = distributions::register(py, &module)?;
    exports.extend(dist_exports.iter().copied());

    let prob_exports = probability::register(py, &module)?;
    exports.extend(prob_exports.iter().copied());

    let integration_exports = integration::register(py, &module)?;
    exports.extend(integration_exports.iter().copied());

    let interp_exports = interp::register(py, &module)?;
    exports.extend(interp_exports.iter().copied());

    let solver_exports = solver::register(py, &module)?;
    exports.extend(solver_exports.iter().copied());

    let multi_exports = solver_multi::register(py, &module)?;
    exports.extend(multi_exports.iter().copied());

    let special_exports = special_functions::register(py, &module)?;
    exports.extend(special_exports.iter().copied());

    let stats_exports = stats::register(py, &module)?;
    exports.extend(stats_exports.iter().copied());

    let random_exports = random::register(py, &module)?;
    exports.extend(random_exports.iter().copied());

    let linalg_exports = linalg::register(py, &module)?;
    exports.extend(linalg_exports.iter().copied());

    let sum_exports = summation::register(py, &module)?;
    exports.extend(sum_exports.iter().copied());

    exports = {
        let mut seen = HashSet::default();
        exports
            .into_iter()
            .filter(|item| seen.insert(*item))
            .collect()
    };
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
