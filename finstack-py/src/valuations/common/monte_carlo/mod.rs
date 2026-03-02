//! Monte Carlo simulation infrastructure Python bindings.
//!
//! This module provides Python bindings for Monte Carlo path generation,
//! stochastic processes, discretization schemes, and result structures.

pub(crate) mod discretization;
pub(crate) mod estimate;
pub(crate) mod generator;
pub(crate) mod params;
pub(crate) mod paths;
pub(crate) mod processes;
pub(crate) mod result;
pub(crate) mod time_grid;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Register the Monte Carlo submodule with all classes at the mc level.
pub(crate) fn register(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mc_module = PyModule::new(py, "monte_carlo")?;
    mc_module.setattr(
        "__doc__",
        "Monte Carlo simulation infrastructure for path generation and pricing.",
    )?;

    // Register all MC classes directly to the mc module (not as sub-submodules)

    // --- Existing types ---
    mc_module.add_class::<params::PyProcessParams>()?;
    mc_module.add_class::<paths::PyCashflowType>()?;
    mc_module.add_class::<paths::PyPathPoint>()?;
    mc_module.add_class::<paths::PySimulatedPath>()?;
    mc_module.add_class::<paths::PyPathDataset>()?;
    mc_module.add_class::<paths::PyPathDatasetIterator>()?;
    mc_module.add_class::<result::PyMonteCarloResult>()?;
    mc_module.add_class::<generator::PyMonteCarloPathGenerator>()?;

    // --- New building blocks ---

    // Time grid
    mc_module.add_class::<time_grid::PyTimeGrid>()?;

    // Estimate & convergence diagnostics
    mc_module.add_class::<estimate::PyEstimate>()?;
    mc_module.add_class::<estimate::PyConvergenceDiagnostics>()?;

    // Stochastic process parameters
    mc_module.add_class::<processes::PyGbmParams>()?;
    mc_module.add_class::<processes::PyHestonParams>()?;
    mc_module.add_class::<processes::PyCirParams>()?;
    mc_module.add_class::<processes::PyHullWhite1FParams>()?;
    mc_module.add_class::<processes::PyMertonJumpParams>()?;
    mc_module.add_class::<processes::PySchwartzSmithParams>()?;
    mc_module.add_class::<processes::PyBrownianParams>()?;
    mc_module.add_class::<processes::PyMultiOuParams>()?;

    // Discretization scheme descriptors
    mc_module.add_class::<discretization::PyExactGbmScheme>()?;
    mc_module.add_class::<discretization::PyEulerMaruyamaScheme>()?;
    mc_module.add_class::<discretization::PyLogEulerScheme>()?;
    mc_module.add_class::<discretization::PyMilsteinScheme>()?;
    mc_module.add_class::<discretization::PyLogMilsteinScheme>()?;
    mc_module.add_class::<discretization::PyQeHestonScheme>()?;
    mc_module.add_class::<discretization::PyQeCirScheme>()?;
    mc_module.add_class::<discretization::PyExactHullWhite1FScheme>()?;
    mc_module.add_class::<discretization::PyJumpEulerScheme>()?;
    mc_module.add_class::<discretization::PyExactSchwartzSmithScheme>()?;

    let exports = vec![
        // Existing types
        "ProcessParams",
        "CashflowType",
        "PathPoint",
        "SimulatedPath",
        "PathDataset",
        "MonteCarloResult",
        "MonteCarloPathGenerator",
        // New building blocks
        "TimeGrid",
        "Estimate",
        "ConvergenceDiagnostics",
        // Process parameters
        "GbmParams",
        "HestonParams",
        "CirParams",
        "HullWhite1FParams",
        "MertonJumpParams",
        "SchwartzSmithParams",
        "BrownianParams",
        "MultiOuParams",
        // Discretization schemes
        "ExactGbmScheme",
        "EulerMaruyamaScheme",
        "LogEulerScheme",
        "MilsteinScheme",
        "LogMilsteinScheme",
        "QeHestonScheme",
        "QeCirScheme",
        "ExactHullWhite1FScheme",
        "JumpEulerScheme",
        "ExactSchwartzSmithScheme",
    ];

    mc_module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&mc_module)?;
    parent.setattr("monte_carlo", &mc_module)?;

    Ok(exports)
}
