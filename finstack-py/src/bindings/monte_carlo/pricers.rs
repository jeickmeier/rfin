//! Pricer bindings — European, Path-Dependent, LSMC.

use super::engine::resolve_currency;
use super::results::PyMonteCarloResult;
use crate::errors::core_to_py;
use finstack_monte_carlo::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
use finstack_monte_carlo::process::gbm::GbmProcess;
use pyo3::prelude::*;

/// Convenience pricer for European options under GBM dynamics.
#[pyclass(name = "EuropeanPricer", module = "finstack.monte_carlo", frozen)]
pub struct PyEuropeanPricer {
    num_paths: usize,
    seed: u64,
}

#[pymethods]
impl PyEuropeanPricer {
    #[new]
    #[pyo3(signature = (num_paths=100_000, seed=42))]
    fn new(num_paths: usize, seed: u64) -> Self {
        Self { num_paths, seed }
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.num_paths
    }
    #[getter]
    fn seed(&self) -> u64 {
        self.seed
    }

    /// Price a European call option under GBM.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=252, currency=None))]
    fn price_call(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = resolve_currency(currency)?;
        let payoff = EuropeanCall::new(strike, 1.0, num_steps);
        let df = (-rate * expiry).exp();
        self.run(
            &payoff, spot, rate, div_yield, vol, expiry, num_steps, ccy, df,
        )
    }

    /// Price a European put option under GBM.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=252, currency=None))]
    fn price_put(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        let ccy = resolve_currency(currency)?;
        let payoff = EuropeanPut::new(strike, 1.0, num_steps);
        let df = (-rate * expiry).exp();
        self.run(
            &payoff, spot, rate, div_yield, vol, expiry, num_steps, ccy, df,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanPricer(num_paths={}, seed={})",
            self.num_paths, self.seed,
        )
    }
}

impl PyEuropeanPricer {
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        payoff: &impl finstack_monte_carlo::traits::Payoff,
        spot: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: finstack_core::currency::Currency,
        discount_factor: f64,
    ) -> PyResult<PyMonteCarloResult> {
        let config = EuropeanPricerConfig::new(self.num_paths)
            .with_seed(self.seed)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
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
}

// ---------------------------------------------------------------------------
// Path-dependent pricer
// ---------------------------------------------------------------------------

/// Path-dependent Monte Carlo pricer for exotic payoffs (Asian, barrier, etc.).
#[pyclass(name = "PathDependentPricer", module = "finstack.monte_carlo", frozen)]
pub struct PyPathDependentPricer {
    num_paths: usize,
    seed: u64,
    use_parallel: bool,
}

#[pymethods]
impl PyPathDependentPricer {
    #[new]
    #[pyo3(signature = (num_paths=100_000, seed=42, use_parallel=false))]
    fn new(num_paths: usize, seed: u64, use_parallel: bool) -> Self {
        Self {
            num_paths,
            seed,
            use_parallel,
        }
    }

    /// Price an Asian call under GBM dynamics.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=252, currency=None))]
    fn price_asian_call(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        use finstack_monte_carlo::payoff::asian::{AsianCall, AveragingMethod};
        let ccy = resolve_currency(currency)?;
        let fixing_steps: Vec<usize> = (1..=num_steps).collect();
        let payoff = AsianCall::new(strike, 1.0, AveragingMethod::Arithmetic, fixing_steps);
        let df = (-rate * expiry).exp();
        self.run_gbm(
            &payoff, spot, rate, div_yield, vol, expiry, num_steps, ccy, df,
        )
    }

    /// Price an Asian put under GBM dynamics.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=252, currency=None))]
    fn price_asian_put(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        use finstack_monte_carlo::payoff::asian::{AsianPut, AveragingMethod};
        let ccy = resolve_currency(currency)?;
        let fixing_steps: Vec<usize> = (1..=num_steps).collect();
        let payoff = AsianPut::new(strike, 1.0, AveragingMethod::Arithmetic, fixing_steps);
        let df = (-rate * expiry).exp();
        self.run_gbm(
            &payoff, spot, rate, div_yield, vol, expiry, num_steps, ccy, df,
        )
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.num_paths
    }
    #[getter]
    fn seed(&self) -> u64 {
        self.seed
    }

    fn __repr__(&self) -> String {
        format!(
            "PathDependentPricer(paths={}, seed={}, parallel={})",
            self.num_paths, self.seed, self.use_parallel,
        )
    }
}

impl PyPathDependentPricer {
    #[allow(clippy::too_many_arguments)]
    fn run_gbm(
        &self,
        payoff: &impl finstack_monte_carlo::traits::Payoff,
        spot: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: finstack_core::currency::Currency,
        discount_factor: f64,
    ) -> PyResult<PyMonteCarloResult> {
        use finstack_monte_carlo::pricer::path_dependent::{
            PathDependentPricer, PathDependentPricerConfig,
        };

        let config = PathDependentPricerConfig::new(self.num_paths)
            .with_seed(self.seed)
            .with_parallel(self.use_parallel);
        let pricer = PathDependentPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
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
}

// ---------------------------------------------------------------------------
// LSMC pricer
// ---------------------------------------------------------------------------

/// Longstaff-Schwartz Monte Carlo pricer for American options.
#[pyclass(name = "LsmcPricer", module = "finstack.monte_carlo", frozen)]
pub struct PyLsmcPricer {
    num_paths: usize,
    seed: u64,
}

#[pymethods]
impl PyLsmcPricer {
    #[new]
    #[pyo3(signature = (num_paths=100_000, seed=42))]
    fn new(num_paths: usize, seed: u64) -> Self {
        Self { num_paths, seed }
    }

    /// Price an American put under GBM dynamics.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=50, currency=None))]
    fn price_american_put(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        use finstack_monte_carlo::pricer::lsmc::{AmericanPut, LsmcConfig, LsmcPricer};

        let ccy = resolve_currency(currency)?;
        let exercise = AmericanPut::new(strike).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid AmericanPut: {e}"))
        })?;
        let exercise_dates: Vec<usize> = (1..=num_steps).collect();
        let config = LsmcConfig::new(self.num_paths, exercise_dates).with_seed(self.seed);
        let pricer = LsmcPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;

        pricer
            .price(
                &process,
                spot,
                expiry,
                num_steps,
                &exercise,
                &finstack_monte_carlo::pricer::basis::LaguerreBasis::new(3, strike),
                ccy,
                rate,
            )
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    /// Price an American call under GBM dynamics.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=50, currency=None))]
    fn price_american_call(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: usize,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMonteCarloResult> {
        use finstack_monte_carlo::pricer::lsmc::{AmericanCall, LsmcConfig, LsmcPricer};

        let ccy = resolve_currency(currency)?;
        let exercise = AmericanCall::new(strike).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid AmericanCall: {e}"))
        })?;
        let exercise_dates: Vec<usize> = (1..=num_steps).collect();
        let config = LsmcConfig::new(self.num_paths, exercise_dates).with_seed(self.seed);
        let pricer = LsmcPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;

        pricer
            .price(
                &process,
                spot,
                expiry,
                num_steps,
                &exercise,
                &finstack_monte_carlo::pricer::basis::LaguerreBasis::new(3, strike),
                ccy,
                rate,
            )
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!("LsmcPricer(paths={}, seed={})", self.num_paths, self.seed,)
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEuropeanPricer>()?;
    m.add_class::<PyPathDependentPricer>()?;
    m.add_class::<PyLsmcPricer>()?;
    Ok(())
}
