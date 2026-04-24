//! Python bindings for the `finstack-monte-carlo` crate.
//!
//! Exposes the GBM convenience subset of the Rust Monte Carlo crate: engine,
//! European/Asian/LSMC pricers, and Black-Scholes formulas. Advanced Rust
//! process, discretization, RNG, payoff, and Greeks types are intentionally not
//! standalone Python types yet; their parameters are passed directly as numeric
//! arguments to the exposed pricer constructors and methods.

mod analytical;
mod engine;
mod pricers;
mod results;
mod time_grid;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `finstack.monte_carlo` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "monte_carlo")?;
    m.setattr(
        "__doc__",
        "Monte Carlo GBM convenience bindings (finstack-monte-carlo).",
    )?;

    results::register(py, &m)?;
    time_grid::register(py, &m)?;
    engine::register(py, &m)?;
    pricers::register(py, &m)?;
    analytical::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "MonteCarloResult",
            "Estimate",
            "TimeGrid",
            "McEngine",
            "EuropeanPricer",
            "PathDependentPricer",
            "LsmcPricer",
            "black_scholes_call",
            "black_scholes_put",
            "price_european_call",
            "price_european_put",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.finstack".to_string(),
        },
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.monte_carlo");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
