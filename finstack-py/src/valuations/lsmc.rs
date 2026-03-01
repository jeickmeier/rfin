//! Python bindings for LSMC (Longstaff-Schwartz Monte Carlo) pricer.
//!
//! Provides American/Bermudan option pricing via least-squares regression.

use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::mc::process::gbm::GbmProcess;
use finstack_valuations::instruments::common::models::monte_carlo::prelude::{
    AmericanCall, AmericanPut, LaguerreBasis, LsmcConfig, LsmcPricer, PolynomialBasis,
};
use finstack_valuations::instruments::common::models::monte_carlo::results::MoneyEstimate;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

// =============================================================================
// Exercise Payoffs
// =============================================================================

/// American put option exercise payoff.
///
/// Args:
///     strike: Strike price for the put option.
///
/// Examples:
///     >>> put = AmericanPut(strike=100.0)
///     >>> put.strike
///     100.0
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "AmericanPut",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyAmericanPut {
    inner: AmericanPut,
}

#[pymethods]
impl PyAmericanPut {
    #[new]
    fn new(strike: f64) -> PyResult<Self> {
        if strike <= 0.0 {
            return Err(PyValueError::new_err("strike must be positive"));
        }
        Ok(Self {
            inner: AmericanPut { strike },
        })
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    fn __repr__(&self) -> String {
        format!("AmericanPut(strike={})", self.inner.strike)
    }
}

/// American call option exercise payoff.
///
/// Args:
///     strike: Strike price for the call option.
///
/// Examples:
///     >>> call = AmericanCall(strike=100.0)
///     >>> call.strike
///     100.0
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "AmericanCall",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyAmericanCall {
    inner: AmericanCall,
}

#[pymethods]
impl PyAmericanCall {
    #[new]
    fn new(strike: f64) -> PyResult<Self> {
        if strike <= 0.0 {
            return Err(PyValueError::new_err("strike must be positive"));
        }
        Ok(Self {
            inner: AmericanCall { strike },
        })
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    fn __repr__(&self) -> String {
        format!("AmericanCall(strike={})", self.inner.strike)
    }
}

// =============================================================================
// Basis Functions
// =============================================================================

/// Polynomial basis functions for LSMC regression.
///
/// Creates a basis of {1, x, x², ..., x^degree} for regression in the
/// Longstaff-Schwartz algorithm.
///
/// Args:
///     degree: Polynomial degree (must be positive).
///
/// Examples:
///     >>> basis = PolynomialBasis(degree=3)
///     >>> basis.degree
///     3
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "PolynomialBasis",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyPolynomialBasis {
    degree_: usize,
}

#[pymethods]
impl PyPolynomialBasis {
    #[new]
    fn new(degree: usize) -> PyResult<Self> {
        if degree == 0 {
            return Err(PyValueError::new_err("degree must be positive"));
        }
        Ok(Self { degree_: degree })
    }

    #[getter]
    fn degree(&self) -> usize {
        self.degree_
    }

    #[getter]
    fn num_basis(&self) -> usize {
        self.degree_ + 1
    }

    fn __repr__(&self) -> String {
        format!("PolynomialBasis(degree={})", self.degree_)
    }
}

/// Laguerre polynomial basis functions for LSMC regression.
///
/// Creates a basis using Laguerre polynomials normalized by strike,
/// which provides better numerical stability for option pricing.
///
/// Args:
///     degree: Polynomial degree (must be 1-4).
///     strike: Strike price for normalization (must be positive).
///
/// Examples:
///     >>> basis = LaguerreBasis(degree=3, strike=100.0)
///     >>> basis.degree
///     3
///     >>> basis.strike
///     100.0
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "LaguerreBasis",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyLaguerreBasis {
    degree_: usize,
    strike_: f64,
}

#[pymethods]
impl PyLaguerreBasis {
    #[new]
    fn new(degree: usize, strike: f64) -> PyResult<Self> {
        if degree == 0 || degree > 4 {
            return Err(PyValueError::new_err("degree must be 1-4"));
        }
        if strike <= 0.0 {
            return Err(PyValueError::new_err("strike must be positive"));
        }
        Ok(Self {
            degree_: degree,
            strike_: strike,
        })
    }

    #[getter]
    fn degree(&self) -> usize {
        self.degree_
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.strike_
    }

    #[getter]
    fn num_basis(&self) -> usize {
        self.degree_ + 1
    }

    fn __repr__(&self) -> String {
        format!(
            "LaguerreBasis(degree={}, strike={})",
            self.degree_, self.strike_
        )
    }
}

// =============================================================================
// LSMC Configuration and Pricer
// =============================================================================

/// Configuration for LSMC (Longstaff-Schwartz Monte Carlo) pricer.
///
/// Args:
///     num_paths: Number of Monte Carlo paths to simulate.
///     exercise_dates: List of step indices where exercise is allowed.
///     seed: Random seed for reproducibility (default: 42).
///
/// Examples:
///     >>> config = LsmcConfig(
///     ...     num_paths=50000,
///     ...     exercise_dates=[25, 50, 75, 100],
///     ...     seed=42
///     ... )
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "LsmcConfig",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyLsmcConfig {
    inner: LsmcConfig,
}

#[pymethods]
impl PyLsmcConfig {
    #[new]
    #[pyo3(signature = (num_paths, exercise_dates, seed=42))]
    fn new(num_paths: usize, exercise_dates: Vec<usize>, seed: u64) -> PyResult<Self> {
        if num_paths == 0 {
            return Err(PyValueError::new_err("num_paths must be positive"));
        }
        if exercise_dates.is_empty() {
            return Err(PyValueError::new_err(
                "exercise_dates must have at least one element",
            ));
        }
        Ok(Self {
            inner: LsmcConfig::new(num_paths, exercise_dates).with_seed(seed),
        })
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    #[getter]
    fn exercise_dates<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, &self.inner.exercise_dates)
    }

    fn __repr__(&self) -> String {
        format!(
            "LsmcConfig(num_paths={}, exercise_dates={:?}, seed={})",
            self.inner.num_paths, self.inner.exercise_dates, self.inner.seed
        )
    }
}

/// LSMC result containing price estimate and statistics.
///
/// Attributes:
///     mean: Point estimate of the option price.
///     stderr: Standard error of the estimate.
///     ci_95: 95% confidence interval (lower, upper).
///     num_paths: Number of paths used.
#[pyclass(
    module = "finstack.valuations.lsmc",
    name = "LsmcResult",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyLsmcResult {
    mean: Money,
    stderr: f64,
    ci_95_lower: Money,
    ci_95_upper: Money,
    num_paths: usize,
}

#[pymethods]
impl PyLsmcResult {
    #[getter]
    fn mean(&self) -> PyMoney {
        PyMoney::new(self.mean)
    }

    #[getter]
    fn stderr(&self) -> f64 {
        self.stderr
    }

    #[getter]
    fn ci_95(&self) -> (PyMoney, PyMoney) {
        (
            PyMoney::new(self.ci_95_lower),
            PyMoney::new(self.ci_95_upper),
        )
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.num_paths
    }

    /// Relative standard error (stderr / mean).
    fn relative_stderr(&self) -> f64 {
        if self.mean.amount().abs() < 1e-10 {
            f64::INFINITY
        } else {
            self.stderr.abs() / self.mean.amount().abs()
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LsmcResult(mean={}, stderr={:.6}, num_paths={})",
            self.mean, self.stderr, self.num_paths
        )
    }
}

/// Enum tags for exercise type
enum ExerciseType {
    Put,
    Call,
}

/// Enum tags for basis type
enum BasisType {
    Polynomial,
    Laguerre,
}

/// Helper function to price with specific exercise and basis types
fn do_price(
    pricer: &LsmcPricer,
    process: &GbmProcess,
    initial_spot: f64,
    time_to_maturity: f64,
    num_steps: usize,
    strike: f64,
    exercise_type: ExerciseType,
    basis_type: BasisType,
    basis_degree: usize,
    currency: Currency,
    r: f64,
) -> finstack_core::Result<MoneyEstimate> {
    // Use monomorphized calls based on exercise and basis type
    match (exercise_type, basis_type) {
        (ExerciseType::Put, BasisType::Polynomial) => {
            let exercise = AmericanPut { strike };
            let basis = PolynomialBasis::new(basis_degree);
            pricer.price(
                process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                currency,
                r,
            )
        }
        (ExerciseType::Put, BasisType::Laguerre) => {
            let exercise = AmericanPut { strike };
            let basis = LaguerreBasis::new(basis_degree, strike);
            pricer.price(
                process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                currency,
                r,
            )
        }
        (ExerciseType::Call, BasisType::Polynomial) => {
            let exercise = AmericanCall { strike };
            let basis = PolynomialBasis::new(basis_degree);
            pricer.price(
                process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                currency,
                r,
            )
        }
        (ExerciseType::Call, BasisType::Laguerre) => {
            let exercise = AmericanCall { strike };
            let basis = LaguerreBasis::new(basis_degree, strike);
            pricer.price(
                process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                currency,
                r,
            )
        }
    }
}

/// LSMC (Longstaff-Schwartz Monte Carlo) pricer for American/Bermudan options.
///
/// Uses backward induction with least-squares regression to price options
/// with early exercise features.
///
/// Args:
///     config: LSMC configuration with paths, exercise dates, and seed.
///
/// Examples:
///     >>> config = LsmcConfig(num_paths=50000, exercise_dates=[25, 50, 75, 100])
///     >>> pricer = LsmcPricer(config)
///     >>> put = AmericanPut(strike=100.0)
///     >>> basis = LaguerreBasis(degree=3, strike=100.0)
///     >>> result = pricer.price(
///     ...     initial_spot=100.0,
///     ...     r=0.05, q=0.0, sigma=0.20,
///     ...     time_to_maturity=1.0,
///     ...     num_steps=100,
///     ...     exercise=put,
///     ...     basis=basis,
///     ...     currency="USD"
///     ... )
#[pyclass(module = "finstack.valuations.lsmc", name = "LsmcPricer")]
pub struct PyLsmcPricer {
    config: LsmcConfig,
}

#[pymethods]
impl PyLsmcPricer {
    #[new]
    fn new(config: &PyLsmcConfig) -> Self {
        Self {
            config: config.inner.clone(),
        }
    }

    /// Price an American-style option using LSMC.
    ///
    /// Args:
    ///     initial_spot: Initial spot price of the underlying.
    ///     r: Risk-free interest rate (annual, decimal).
    ///     q: Dividend/foreign rate (annual, decimal).
    ///     sigma: Volatility (annual, decimal).
    ///     time_to_maturity: Time to maturity in years.
    ///     num_steps: Number of time steps for discretization.
    ///     exercise: Exercise payoff (AmericanPut or AmericanCall).
    ///     basis: Basis functions for regression (PolynomialBasis or LaguerreBasis).
    ///     currency: Currency code (e.g., "USD").
    ///
    /// Returns:
    ///     LsmcResult: Statistical estimate of the option value.
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid.
    #[pyo3(signature = (initial_spot, r, q, sigma, time_to_maturity, num_steps, exercise, basis, currency))]
    fn price(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        exercise: &Bound<'_, PyAny>,
        basis: &Bound<'_, PyAny>,
        currency: &str,
    ) -> PyResult<PyLsmcResult> {
        // Parse currency
        let currency: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("invalid currency: {}", e)))?;

        // Extract exercise payoff type and strike
        let (exercise_type, strike) = if let Ok(put) = exercise.extract::<PyRef<PyAmericanPut>>() {
            (ExerciseType::Put, put.inner.strike)
        } else if let Ok(call) = exercise.extract::<PyRef<PyAmericanCall>>() {
            (ExerciseType::Call, call.inner.strike)
        } else {
            return Err(PyValueError::new_err(
                "exercise must be AmericanPut or AmericanCall",
            ));
        };

        // Extract basis functions type and degree
        let (basis_type, basis_degree) =
            if let Ok(poly) = basis.extract::<PyRef<PyPolynomialBasis>>() {
                (BasisType::Polynomial, poly.degree_)
            } else if let Ok(lag) = basis.extract::<PyRef<PyLaguerreBasis>>() {
                (BasisType::Laguerre, lag.degree_)
            } else {
                return Err(PyValueError::new_err(
                    "basis must be PolynomialBasis or LaguerreBasis",
                ));
            };

        // Create GBM process
        let process = GbmProcess::with_params(r, q, sigma);

        // Create pricer
        let pricer = LsmcPricer::new(self.config.clone());

        // Price option - the Rust code already handles parallel execution
        let result = do_price(
            &pricer,
            &process,
            initial_spot,
            time_to_maturity,
            num_steps,
            strike,
            exercise_type,
            basis_type,
            basis_degree,
            currency,
            r,
        )
        .map_err(core_to_py)?;

        Ok(PyLsmcResult {
            mean: result.mean,
            stderr: result.stderr,
            ci_95_lower: result.ci_95.0,
            ci_95_upper: result.ci_95.1,
            num_paths: result.num_paths,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "LsmcPricer(num_paths={}, exercise_dates={:?})",
            self.config.num_paths, self.config.exercise_dates
        )
    }
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "lsmc")?;
    module.setattr(
        "__doc__",
        "LSMC (Longstaff-Schwartz Monte Carlo) pricer for American/Bermudan options.",
    )?;

    // Add classes
    module.add_class::<PyAmericanPut>()?;
    module.add_class::<PyAmericanCall>()?;
    module.add_class::<PyPolynomialBasis>()?;
    module.add_class::<PyLaguerreBasis>()?;
    module.add_class::<PyLsmcConfig>()?;
    module.add_class::<PyLsmcResult>()?;
    module.add_class::<PyLsmcPricer>()?;

    let exports = vec![
        "AmericanPut",
        "AmericanCall",
        "PolynomialBasis",
        "LaguerreBasis",
        "LsmcConfig",
        "LsmcResult",
        "LsmcPricer",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
