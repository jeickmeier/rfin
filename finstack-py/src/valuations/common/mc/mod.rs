//! Monte Carlo simulation infrastructure Python bindings.
//!
//! This module provides Python bindings for Monte Carlo path generation,
//! stochastic processes, discretization schemes, and result structures.

pub(crate) mod generator;
pub(crate) mod params;
pub(crate) mod paths;
pub(crate) mod result;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Register the Monte Carlo submodule with all classes at the mc level.
pub(crate) fn register(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mc_module = PyModule::new(py, "mc")?;
    mc_module.setattr(
        "__doc__",
        "Monte Carlo simulation infrastructure for path generation and pricing.",
    )?;

    // Register all MC classes directly to the mc module (not as sub-submodules)
    mc_module.add_class::<params::PyProcessParams>()?;
    mc_module.add_class::<paths::PyCashflowType>()?;
    mc_module.add_class::<paths::PyPathPoint>()?;
    mc_module.add_class::<paths::PySimulatedPath>()?;
    mc_module.add_class::<paths::PyPathDataset>()?;
    mc_module.add_class::<paths::PyPathDatasetIterator>()?;
    mc_module.add_class::<result::PyMonteCarloResult>()?;
    mc_module.add_class::<generator::PyMonteCarloPathGenerator>()?;

    let exports = vec![
        "ProcessParams",
        "CashflowType",
        "PathPoint",
        "SimulatedPath",
        "PathDataset",
        "MonteCarloResult",
        "MonteCarloPathGenerator",
    ];

    mc_module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&mc_module)?;
    parent.setattr("mc", &mc_module)?;

    Ok(exports)
}
