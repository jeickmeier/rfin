//! Python bindings for the `finstack-monte-carlo` crate.
//!
//! Exposes Monte Carlo pricing infrastructure: engine, processes,
//! discretisation schemes, payoffs, pricers, and analytical formulas.

mod analytical;
mod discretization;
mod engine;
mod payoffs;
mod pricers;
mod processes;
mod results;
mod time_grid;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `finstack.monte_carlo` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "monte_carlo")?;
    m.setattr(
        "__doc__",
        "Monte Carlo pricing bindings (finstack-monte-carlo).",
    )?;

    results::register(py, &m)?;
    time_grid::register(py, &m)?;
    engine::register(py, &m)?;
    processes::register(py, &m)?;
    discretization::register(py, &m)?;
    payoffs::register(py, &m)?;
    pricers::register(py, &m)?;
    analytical::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            // Results
            "MonteCarloResult",
            "Estimate",
            // Time grid
            "TimeGrid",
            // Engine
            "McEngineConfig",
            "McEngine",
            // Processes
            "GbmProcess",
            "MultiGbmProcess",
            "BrownianProcess",
            "HestonProcess",
            "CirProcess",
            "MertonJumpProcess",
            "BatesProcess",
            "SchwartzSmithProcess",
            // Discretisation
            "ExactGbm",
            "ExactMultiGbm",
            "EulerMaruyama",
            "LogEuler",
            "Milstein",
            // Payoffs
            "EuropeanCall",
            "EuropeanPut",
            "DigitalCall",
            "DigitalPut",
            "ForwardLong",
            "ForwardShort",
            "AsianCall",
            "AsianPut",
            "BarrierOption",
            "BasketCall",
            "BasketPut",
            "AmericanPut",
            "AmericanCall",
            // Pricers
            "EuropeanPricer",
            "PathDependentPricer",
            "LsmcPricer",
            // Analytical
            "black_scholes_call",
            "black_scholes_put",
            // Convenience
            "price_european_call",
            "price_european_put",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack".to_string(),
        },
        Err(_) => "finstack".to_string(),
    };
    let qual = format!("{pkg}.monte_carlo");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
