//! McEngine and McEngineConfig bindings.

use super::results::PyMonteCarloResult;
use super::time_grid::PyTimeGrid;
use crate::bindings::core::currency::extract_currency;
use crate::errors::core_to_py;
use finstack_monte_carlo::discretization::exact::ExactGbm;
use finstack_monte_carlo::engine::{McEngine, McEngineConfig};
use finstack_monte_carlo::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::time_grid::TimeGrid;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::str::FromStr;

/// Configuration for the Monte Carlo engine.
#[pyclass(name = "McEngineConfig", module = "finstack.monte_carlo", frozen)]
pub struct PyMcEngineConfig {
    num_paths: usize,
    seed: u64,
    time_to_maturity: f64,
    num_steps: usize,
}

#[pymethods]
impl PyMcEngineConfig {
    #[new]
    #[pyo3(signature = (num_paths, seed, time_to_maturity=1.0, num_steps=252))]
    fn new(num_paths: usize, seed: u64, time_to_maturity: f64, num_steps: usize) -> Self {
        Self {
            num_paths,
            seed,
            time_to_maturity,
            num_steps,
        }
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.num_paths
    }
    #[getter]
    fn seed(&self) -> u64 {
        self.seed
    }
    #[getter]
    fn time_to_maturity(&self) -> f64 {
        self.time_to_maturity
    }
    #[getter]
    fn num_steps(&self) -> usize {
        self.num_steps
    }

    /// Price a European call using the generic engine.
    #[pyo3(text_signature = "(self, spot, strike, rate, div_yield, vol, currency)")]
    fn price_call(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: &Bound<'_, PyAny>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = extract_currency(currency)?;
        let payoff = EuropeanCall::new(strike, 1.0, self.num_steps);
        self.run_engine(spot, rate, div_yield, vol, ccy, &payoff)
    }

    /// Price a European put using the generic engine.
    #[pyo3(text_signature = "(self, spot, strike, rate, div_yield, vol, currency)")]
    fn price_put(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: &Bound<'_, PyAny>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = extract_currency(currency)?;
        let payoff = EuropeanPut::new(strike, 1.0, self.num_steps);
        self.run_engine(spot, rate, div_yield, vol, ccy, &payoff)
    }

    fn __repr__(&self) -> String {
        format!(
            "McEngineConfig(paths={}, seed={}, T={:.2}, steps={})",
            self.num_paths, self.seed, self.time_to_maturity, self.num_steps
        )
    }
}

impl PyMcEngineConfig {
    fn run_engine(
        &self,
        spot: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: finstack_core::currency::Currency,
        payoff: &impl finstack_monte_carlo::traits::Payoff,
    ) -> PyResult<PyMonteCarloResult> {
        let time_grid =
            TimeGrid::uniform(self.time_to_maturity, self.num_steps).map_err(core_to_py)?;
        let config = McEngineConfig::new(self.num_paths, time_grid).with_seed(self.seed);
        let engine = McEngine::new(config);
        let rng = PhiloxRng::new(self.seed);
        let process = GbmProcess::with_params(rate, div_yield, vol);
        let disc = ExactGbm::new();
        let df = (-rate * self.time_to_maturity).exp();

        engine
            .price(&rng, &process, &disc, &[spot], payoff, currency, df)
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }
}

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
        let n = self.inner.config().time_grid.num_steps();
        let payoff = EuropeanCall::new(strike, 1.0, n);
        let df = (-rate * t_max).exp();
        self.run_gbm(spot, rate, div_yield, vol, ccy, &payoff, df)
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
        let n = self.inner.config().time_grid.num_steps();
        let payoff = EuropeanPut::new(strike, 1.0, n);
        let df = (-rate * t_max).exp();
        self.run_gbm(spot, rate, div_yield, vol, ccy, &payoff, df)
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

impl PyMcEngine {
    #[allow(clippy::too_many_arguments)]
    fn run_gbm(
        &self,
        spot: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        currency: finstack_core::currency::Currency,
        payoff: &impl finstack_monte_carlo::traits::Payoff,
        discount_factor: f64,
    ) -> PyResult<PyMonteCarloResult> {
        let rng = PhiloxRng::new(self.seed);
        let process = GbmProcess::with_params(rate, div_yield, vol);
        let disc = ExactGbm::new();
        self.inner
            .price(
                &rng,
                &process,
                &disc,
                &[spot],
                payoff,
                currency,
                discount_factor,
            )
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
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
    let payoff = EuropeanCall::new(strike, 1.0, num_steps);
    let df = (-rate * expiry).exp();
    run_european_pricer(
        spot, rate, div_yield, vol, expiry, num_paths, seed, num_steps, ccy, &payoff, df,
    )
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
    let payoff = EuropeanPut::new(strike, 1.0, num_steps);
    let df = (-rate * expiry).exp();
    run_european_pricer(
        spot, rate, div_yield, vol, expiry, num_paths, seed, num_steps, ccy, &payoff, df,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_european_pricer(
    spot: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    currency: finstack_core::currency::Currency,
    payoff: &impl finstack_monte_carlo::traits::Payoff,
    discount_factor: f64,
) -> PyResult<PyMonteCarloResult> {
    use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};

    let config = EuropeanPricerConfig::new(num_paths)
        .with_seed(seed)
        .with_parallel(false);
    let pricer = EuropeanPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol);

    pricer
        .price(
            &process,
            spot,
            expiry,
            num_steps,
            payoff,
            currency,
            discount_factor,
        )
        .map(PyMonteCarloResult::from_inner)
        .map_err(core_to_py)
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMcEngineConfig>()?;
    m.add_class::<PyMcEngine>()?;
    m.add_function(wrap_pyfunction!(price_european_call, m)?)?;
    m.add_function(wrap_pyfunction!(price_european_put, m)?)?;
    Ok(())
}
