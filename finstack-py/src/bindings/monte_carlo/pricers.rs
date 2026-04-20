//! Pricer bindings — European, Path-Dependent, LSMC.

use super::engine::resolve_currency;
use super::results::PyMonteCarloResult;
use crate::errors::core_to_py;
use finstack_monte_carlo::pricer::european::EuropeanPricer;
use finstack_monte_carlo::process::gbm::GbmProcess;
use pyo3::prelude::*;

/// Convenience pricer for European options under GBM dynamics.
#[pyclass(name = "EuropeanPricer", module = "finstack.monte_carlo", frozen)]
pub struct PyEuropeanPricer {
    num_paths: usize,
    seed: u64,
    use_parallel: bool,
}

#[pymethods]
impl PyEuropeanPricer {
    #[new]
    #[pyo3(signature = (num_paths=100_000, seed=42, use_parallel=false))]
    fn new(num_paths: usize, seed: u64, use_parallel: bool) -> Self {
        Self {
            num_paths,
            seed,
            use_parallel,
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
    fn use_parallel(&self) -> bool {
        self.use_parallel
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
        self.build_pricer()
            .price_gbm_call(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
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
        self.build_pricer()
            .price_gbm_put(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanPricer(num_paths={}, seed={}, use_parallel={})",
            self.num_paths, self.seed, self.use_parallel,
        )
    }
}

impl PyEuropeanPricer {
    fn build_pricer(&self) -> EuropeanPricer {
        EuropeanPricer::new(self.num_paths)
            .with_seed(self.seed)
            .with_parallel(self.use_parallel)
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
    use_parallel: bool,
    basis: LsmcBasisKind,
    basis_degree: usize,
}

#[derive(Debug, Clone, Copy)]
enum LsmcBasisKind {
    Laguerre,
    Polynomial,
    NormalizedPolynomial,
}

impl LsmcBasisKind {
    fn parse(name: Option<&str>) -> PyResult<Self> {
        match name.unwrap_or("laguerre").to_ascii_lowercase().as_str() {
            "laguerre" => Ok(Self::Laguerre),
            "polynomial" | "poly" => Ok(Self::Polynomial),
            "normalized_polynomial" | "normalized" | "centered_polynomial" => {
                Ok(Self::NormalizedPolynomial)
            }
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown basis '{other}'; expected one of 'laguerre', 'polynomial', \
                 'normalized_polynomial'"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Laguerre => "laguerre",
            Self::Polynomial => "polynomial",
            Self::NormalizedPolynomial => "normalized_polynomial",
        }
    }
}

/// Polymorphic basis wrapper so the pricer can accept multiple concrete
/// implementations from Python without duplicating the inner pricing calls.
enum AnyBasis {
    Laguerre(finstack_monte_carlo::pricer::basis::LaguerreBasis),
    Polynomial(finstack_monte_carlo::pricer::basis::PolynomialBasis),
    NormalizedPolynomial(finstack_monte_carlo::pricer::basis::NormalizedPolynomialBasis),
}

impl finstack_monte_carlo::pricer::basis::BasisFunctions for AnyBasis {
    fn num_basis(&self) -> usize {
        match self {
            Self::Laguerre(b) => b.num_basis(),
            Self::Polynomial(b) => b.num_basis(),
            Self::NormalizedPolynomial(b) => b.num_basis(),
        }
    }

    fn evaluate(&self, state: f64, out: &mut [f64]) {
        match self {
            Self::Laguerre(b) => b.evaluate(state, out),
            Self::Polynomial(b) => b.evaluate(state, out),
            Self::NormalizedPolynomial(b) => b.evaluate(state, out),
        }
    }

    fn evaluate_with_aux(&self, state: f64, aux: Option<f64>, out: &mut [f64]) {
        match self {
            Self::Laguerre(b) => b.evaluate_with_aux(state, aux, out),
            Self::Polynomial(b) => b.evaluate_with_aux(state, aux, out),
            Self::NormalizedPolynomial(b) => b.evaluate_with_aux(state, aux, out),
        }
    }
}

impl PyLsmcPricer {
    fn build_basis(&self, strike: f64) -> PyResult<AnyBasis> {
        use finstack_monte_carlo::pricer::basis::{
            LaguerreBasis, NormalizedPolynomialBasis, PolynomialBasis,
        };
        let to_py = |e: String| pyo3::exceptions::PyValueError::new_err(e);
        match self.basis {
            LsmcBasisKind::Laguerre => LaguerreBasis::try_new(self.basis_degree, strike)
                .map(AnyBasis::Laguerre)
                .map_err(to_py),
            LsmcBasisKind::Polynomial => PolynomialBasis::try_new(self.basis_degree)
                .map(AnyBasis::Polynomial)
                .map_err(to_py),
            LsmcBasisKind::NormalizedPolynomial => {
                NormalizedPolynomialBasis::try_new(self.basis_degree, strike, strike)
                    .map(AnyBasis::NormalizedPolynomial)
                    .map_err(to_py)
            }
        }
    }
}

#[pymethods]
impl PyLsmcPricer {
    #[new]
    #[pyo3(signature = (
        num_paths=100_000,
        seed=42,
        use_parallel=false,
        basis=None,
        basis_degree=3,
    ))]
    fn new(
        num_paths: usize,
        seed: u64,
        use_parallel: bool,
        basis: Option<&str>,
        basis_degree: usize,
    ) -> PyResult<Self> {
        let basis = LsmcBasisKind::parse(basis)?;
        if basis_degree == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "basis_degree must be a positive integer",
            ));
        }
        Ok(Self {
            num_paths,
            seed,
            use_parallel,
            basis,
            basis_degree,
        })
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
    fn use_parallel(&self) -> bool {
        self.use_parallel
    }
    #[getter]
    fn basis(&self) -> &'static str {
        self.basis.as_str()
    }
    #[getter]
    fn basis_degree(&self) -> usize {
        self.basis_degree
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
        let exercise = AmericanPut::new(strike).map_err(core_to_py)?;
        let exercise_dates: Vec<usize> = (1..=num_steps).collect();
        let config = LsmcConfig::new(self.num_paths, exercise_dates, num_steps)
            .map_err(core_to_py)?
            .with_seed(self.seed)
            .with_parallel(self.use_parallel);
        let pricer = LsmcPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;

        let basis = self.build_basis(strike)?;
        pricer
            .price(
                &process, spot, expiry, num_steps, &exercise, &basis, ccy, rate,
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
        let exercise = AmericanCall::new(strike).map_err(core_to_py)?;
        let exercise_dates: Vec<usize> = (1..=num_steps).collect();
        let config = LsmcConfig::new(self.num_paths, exercise_dates, num_steps)
            .map_err(core_to_py)?
            .with_seed(self.seed)
            .with_parallel(self.use_parallel);
        let pricer = LsmcPricer::new(config);
        let process = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;

        let basis = self.build_basis(strike)?;
        pricer
            .price(
                &process, spot, expiry, num_steps, &exercise, &basis, ccy, rate,
            )
            .map(PyMonteCarloResult::from_inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "LsmcPricer(paths={}, seed={}, use_parallel={}, basis={}, basis_degree={})",
            self.num_paths,
            self.seed,
            self.use_parallel,
            self.basis.as_str(),
            self.basis_degree,
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEuropeanPricer>()?;
    m.add_class::<PyPathDependentPricer>()?;
    m.add_class::<PyLsmcPricer>()?;
    Ok(())
}
