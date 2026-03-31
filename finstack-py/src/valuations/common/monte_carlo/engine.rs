//! Python bindings for the Monte Carlo European pricer.
//!
//! Wraps [`EuropeanPricer`] and [`EuropeanPricerConfig`] to give Python users a
//! simple entry point for pricing European-style payoffs under GBM dynamics.
//! A convenience function [`price_european`] further reduces boilerplate.

use super::estimate::PyEstimate;
use super::payoffs::{PyDigital, PyEuropeanCall, PyEuropeanPut, PyForward};
use super::result::PyMonteCarloResult;
use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_monte_carlo::engine::{McEngine, McEngineConfig, PathCaptureConfig};
use finstack_monte_carlo::payoff::vanilla::{Digital, EuropeanCall, EuropeanPut, Forward};
use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::time_grid::TimeGrid;
use finstack_monte_carlo::traits::Payoff;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Configuration for the European Monte Carlo pricer.
///
/// Stores the simulation parameters that do not depend on a specific
/// instrument. Create one configuration and reuse it across multiple
/// ``EuropeanMcPricer.price()`` calls.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "EuropeanPricerConfig",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyEuropeanPricerConfig {
    pub(crate) inner: EuropeanPricerConfig,
}

#[pymethods]
impl PyEuropeanPricerConfig {
    /// Create a pricer configuration.
    ///
    /// Args:
    ///     num_paths: Number of Monte Carlo paths (default 100 000).
    ///     seed: RNG seed for reproducibility (default 42).
    ///     use_parallel: Enable parallel path simulation (default True).
    #[new]
    #[pyo3(signature = (num_paths=100_000, seed=42, use_parallel=true))]
    fn new(num_paths: usize, seed: u64, use_parallel: bool) -> Self {
        Self {
            inner: EuropeanPricerConfig::new(num_paths)
                .with_seed(seed)
                .with_parallel(use_parallel),
        }
    }

    /// Number of Monte Carlo paths.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// RNG seed.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    /// Whether parallel execution is enabled.
    #[getter]
    fn use_parallel(&self) -> bool {
        self.inner.use_parallel
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanPricerConfig(num_paths={}, seed={}, use_parallel={})",
            self.inner.num_paths, self.inner.seed, self.inner.use_parallel
        )
    }
}

/// Compact Monte Carlo pricer for European-style payoffs under GBM.
///
/// Wraps the Rust ``EuropeanPricer`` which uses exact GBM transitions and the
/// Philox counter-based RNG.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "EuropeanMcPricer",
    frozen
)]
pub(crate) struct PyEuropeanMcPricer {
    inner: EuropeanPricer,
}

#[pymethods]
impl PyEuropeanMcPricer {
    /// Create a pricer from a configuration.
    ///
    /// Args:
    ///     config: Pricer configuration.
    #[new]
    fn new(config: &PyEuropeanPricerConfig) -> Self {
        Self {
            inner: EuropeanPricer::new(config.inner.clone()),
        }
    }

    /// Price a European call option.
    ///
    /// Args:
    ///     spot: Current spot price.
    ///     strike: Option strike price.
    ///     r: Risk-free rate (annualised, decimal).
    ///     q: Dividend / foreign rate (annualised, decimal).
    ///     sigma: Volatility (annualised, decimal).
    ///     time_to_maturity: Time to expiry in years.
    ///     num_steps: Number of time-grid steps.
    ///     currency: ISO currency code (e.g. ``"USD"``).
    ///     discount_factor: Present-value multiplier for the payoff.
    ///
    /// Returns:
    ///     ``Estimate`` with the discounted Monte Carlo price.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, r, q, sigma, time_to_maturity, num_steps, currency, discount_factor))]
    fn price_call(
        &self,
        spot: f64,
        strike: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        currency: &str,
        discount_factor: f64,
    ) -> PyResult<PyEstimate> {
        let ccy = parse_ccy(currency)?;
        let process = GbmProcess::with_params(r, q, sigma);
        let payoff = EuropeanCall::new(strike, 1.0, num_steps);
        let est = self
            .inner
            .price(
                &process,
                spot,
                time_to_maturity,
                num_steps,
                &payoff,
                ccy,
                discount_factor,
            )
            .map_err(core_to_py)?;
        Ok(PyEstimate::from_inner(
            finstack_monte_carlo::estimate::Estimate::new(
                est.mean.amount(),
                est.stderr,
                (est.ci_95.0.amount(), est.ci_95.1.amount()),
                est.num_paths,
            ),
        ))
    }

    /// Price a European put option.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, r, q, sigma, time_to_maturity, num_steps, currency, discount_factor))]
    fn price_put(
        &self,
        spot: f64,
        strike: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        currency: &str,
        discount_factor: f64,
    ) -> PyResult<PyEstimate> {
        let ccy = parse_ccy(currency)?;
        let process = GbmProcess::with_params(r, q, sigma);
        let payoff = EuropeanPut::new(strike, 1.0, num_steps);
        let est = self
            .inner
            .price(
                &process,
                spot,
                time_to_maturity,
                num_steps,
                &payoff,
                ccy,
                discount_factor,
            )
            .map_err(core_to_py)?;
        Ok(PyEstimate::from_inner(
            finstack_monte_carlo::estimate::Estimate::new(
                est.mean.amount(),
                est.stderr,
                (est.ci_95.0.amount(), est.ci_95.1.amount()),
                est.num_paths,
            ),
        ))
    }

    fn __repr__(&self) -> String {
        let c = self.inner.config();
        format!(
            "EuropeanMcPricer(num_paths={}, seed={}, parallel={})",
            c.num_paths, c.seed, c.use_parallel
        )
    }
}

/// Price any supported payoff under GBM via the generic McEngine.
///
/// This is the most flexible pricing entry point. The ``payoff`` argument
/// must be one of ``EuropeanCall``, ``EuropeanPut``, ``Digital``, or
/// ``Forward``.
///
/// Args:
///     spot: Initial spot price.
///     r: Risk-free rate (annualised, decimal).
///     q: Dividend / foreign rate (annualised, decimal).
///     sigma: Volatility (annualised, decimal).
///     time_to_maturity: Expiry in years.
///     num_steps: Time-grid steps.
///     num_paths: Number of Monte Carlo paths.
///     payoff: A payoff object (``EuropeanCall``, ``EuropeanPut``,
///         ``Digital``, or ``Forward``).
///     currency: ISO currency code.
///     discount_factor: PV multiplier.
///     seed: RNG seed (default 42).
///     antithetic: Use antithetic variance reduction (default False).
///
/// Returns:
///     ``MonteCarloResult`` with the estimate and optional captured paths.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (spot, r, q, sigma, time_to_maturity, num_steps, num_paths, payoff, currency, discount_factor, seed=42, antithetic=false))]
pub(crate) fn price_european(
    spot: f64,
    r: f64,
    q: f64,
    sigma: f64,
    time_to_maturity: f64,
    num_steps: usize,
    num_paths: usize,
    payoff: &Bound<'_, PyAny>,
    currency: &str,
    discount_factor: f64,
    seed: u64,
    antithetic: bool,
) -> PyResult<PyMonteCarloResult> {
    let ccy = parse_ccy(currency)?;
    let time_grid = TimeGrid::uniform(time_to_maturity, num_steps).map_err(core_to_py)?;
    let config = McEngineConfig {
        num_paths,
        seed,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
        path_capture: PathCaptureConfig::default(),
        antithetic,
    };
    let engine = McEngine::new(config);
    let rng = PhiloxRng::new(seed);
    let process = GbmProcess::with_params(r, q, sigma);
    let disc = finstack_monte_carlo::discretization::exact::ExactGbm::new();
    let initial_state = [spot];

    let mc_result = dispatch_payoff(
        payoff,
        &engine,
        &rng,
        &process,
        &disc,
        &initial_state,
        ccy,
        discount_factor,
        num_steps,
    )?;

    Ok(PyMonteCarloResult::new(mc_result))
}

/// Dispatch pricing to the concrete Rust payoff matching the Python type.
#[allow(clippy::too_many_arguments)]
fn dispatch_payoff(
    payoff: &Bound<'_, PyAny>,
    engine: &McEngine,
    rng: &PhiloxRng,
    process: &GbmProcess,
    disc: &finstack_monte_carlo::discretization::exact::ExactGbm,
    initial_state: &[f64],
    ccy: Currency,
    discount_factor: f64,
    num_steps: usize,
) -> PyResult<finstack_monte_carlo::results::MonteCarloResult> {
    if let Ok(call) = payoff.extract::<PyRef<PyEuropeanCall>>() {
        let inner = EuropeanCall::new(call.strike, call.notional, num_steps);
        return run_engine(
            engine,
            rng,
            process,
            disc,
            initial_state,
            &inner,
            ccy,
            discount_factor,
        );
    }
    if let Ok(put) = payoff.extract::<PyRef<PyEuropeanPut>>() {
        let inner = EuropeanPut::new(put.strike, put.notional, num_steps);
        return run_engine(
            engine,
            rng,
            process,
            disc,
            initial_state,
            &inner,
            ccy,
            discount_factor,
        );
    }
    if let Ok(dig) = payoff.extract::<PyRef<PyDigital>>() {
        let inner = if dig.is_call {
            Digital::call(dig.strike, dig.payout, num_steps)
        } else {
            Digital::put(dig.strike, dig.payout, num_steps)
        };
        return run_engine(
            engine,
            rng,
            process,
            disc,
            initial_state,
            &inner,
            ccy,
            discount_factor,
        );
    }
    if let Ok(fwd) = payoff.extract::<PyRef<PyForward>>() {
        let inner = if fwd.is_long {
            Forward::long(fwd.forward_price, fwd.notional, num_steps)
        } else {
            Forward::short(fwd.forward_price, fwd.notional, num_steps)
        };
        return run_engine(
            engine,
            rng,
            process,
            disc,
            initial_state,
            &inner,
            ccy,
            discount_factor,
        );
    }
    Err(PyValueError::new_err(
        "payoff must be EuropeanCall, EuropeanPut, Digital, or Forward",
    ))
}

/// Run the engine with a concrete payoff type.
#[allow(clippy::too_many_arguments)]
fn run_engine<P: Payoff>(
    engine: &McEngine,
    rng: &PhiloxRng,
    process: &GbmProcess,
    disc: &finstack_monte_carlo::discretization::exact::ExactGbm,
    initial_state: &[f64],
    payoff: &P,
    ccy: Currency,
    discount_factor: f64,
) -> PyResult<finstack_monte_carlo::results::MonteCarloResult> {
    let est = engine
        .price(
            rng,
            process,
            disc,
            initial_state,
            payoff,
            ccy,
            discount_factor,
        )
        .map_err(core_to_py)?;
    Ok(finstack_monte_carlo::results::MonteCarloResult::new(est))
}

fn parse_ccy(s: &str) -> PyResult<Currency> {
    s.parse()
        .map_err(|_| PyValueError::new_err(format!("Unknown currency code: {s}")))
}
