//! McEngine binding (configured via `PyTimeGrid`) plus module-level
//! convenience pricing functions.

use super::results::PyMonteCarloResult;
use super::time_grid::PyTimeGrid;
use crate::bindings::core::currency::extract_currency;
use crate::errors::core_to_py;
use finstack_monte_carlo::engine::{McEngine, McEngineConfig};
use finstack_monte_carlo::pricer::european::EuropeanPricer;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::str::FromStr;

/// The core Monte Carlo engine for full control over simulation.
#[pyclass(name = "McEngine", module = "finstack.monte_carlo", frozen)]
pub struct PyMcEngine {
    inner: McEngine,
    seed: u64,
}

#[pymethods]
impl PyMcEngine {
    /// Build an engine from a time grid configuration.
    #[new]
    #[pyo3(signature = (num_paths, time_grid, seed=42, use_parallel=false, antithetic=false))]
    fn new(
        num_paths: usize,
        time_grid: &PyTimeGrid,
        seed: u64,
        use_parallel: bool,
        antithetic: bool,
    ) -> Self {
        let config = McEngineConfig::new(num_paths, time_grid.inner.clone())
            .with_seed(seed)
            .with_parallel(use_parallel)
            .with_antithetic(antithetic);
        Self {
            inner: McEngine::new(config),
            seed,
        }
    }

    /// Price a European call under GBM.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, currency=None))]
    fn price_european_call(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = resolve_currency(currency)?;
        let t_max = self.inner.config().time_grid.t_max();
        let num_steps = self.inner.config().time_grid.num_steps();
        let pricer = EuropeanPricer::new(self.inner.config().num_paths)
            .with_seed(self.seed)
            .with_parallel(self.inner.config().use_parallel);
        pricer
            .price_gbm_call(spot, strike, rate, div_yield, vol, t_max, num_steps, ccy)
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    /// Price a European put under GBM.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, currency=None))]
    fn price_european_put(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = resolve_currency(currency)?;
        let t_max = self.inner.config().time_grid.t_max();
        let num_steps = self.inner.config().time_grid.num_steps();
        let pricer = EuropeanPricer::new(self.inner.config().num_paths)
            .with_seed(self.seed)
            .with_parallel(self.inner.config().use_parallel);
        pricer
            .price_gbm_put(spot, strike, rate, div_yield, vol, t_max, num_steps, ccy)
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        let c = self.inner.config();
        format!(
            "McEngine(paths={}, steps={}, T={:.4})",
            c.num_paths,
            c.time_grid.num_steps(),
            c.time_grid.t_max()
        )
    }
}

// ---------------------------------------------------------------------------
// Module-level convenience functions
// ---------------------------------------------------------------------------

/// Resolve an optional currency argument, defaulting to USD.
pub(super) fn resolve_currency(
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<finstack_core::currency::Currency> {
    match currency {
        Some(obj) => extract_currency(obj),
        None => finstack_core::currency::Currency::from_str("USD")
            .map_err(|e| PyValueError::new_err(format!("Failed to resolve default currency: {e}"))),
    }
}

/// Price a European call option via Monte Carlo under GBM dynamics.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_paths=100_000, seed=42, num_steps=252, currency=None))]
fn price_european_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyMonteCarloResult> {
    let ccy = resolve_currency(currency)?;
    let pricer = EuropeanPricer::new(num_paths)
        .with_seed(seed)
        .with_parallel(false);
    pricer
        .price_gbm_call(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        .map(PyMonteCarloResult::from_inner)
        .map_err(core_to_py)
}

/// Price a European put option via Monte Carlo under GBM dynamics.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_paths=100_000, seed=42, num_steps=252, currency=None))]
fn price_european_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyMonteCarloResult> {
    let ccy = resolve_currency(currency)?;
    let pricer = EuropeanPricer::new(num_paths)
        .with_seed(seed)
        .with_parallel(false);
    pricer
        .price_gbm_put(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        .map(PyMonteCarloResult::from_inner)
        .map_err(core_to_py)
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMcEngine>()?;
    m.add_function(wrap_pyfunction!(price_european_call, m)?)?;
    m.add_function(wrap_pyfunction!(price_european_put, m)?)?;
    Ok(())
}
