use finstack_valuations::calibration::{SABRCalibrationDerivatives, SABRMarketData};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

/// Internal SABR parameters for bindings.
#[derive(Clone, Debug)]
pub(crate) struct SABRModelParamsData {
    alpha: f64,
    nu: f64,
    rho: f64,
    beta: f64,
}

impl SABRModelParamsData {
    fn new(alpha: f64, nu: f64, rho: f64, beta: f64) -> Self {
        Self {
            alpha,
            nu,
            rho,
            beta,
        }
    }

    fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 1.0)
    }

    fn rates_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 0.5)
    }
}

/// SABR model parameters for volatility surface calibration.
///
/// The SABR (Stochastic Alpha Beta Rho) model is a stochastic volatility model
/// widely used for modeling implied volatility smiles in options markets.
///
/// Parameters:
///     alpha: Initial volatility level
///     nu: Volatility of volatility (vol-of-vol)
///     rho: Correlation between forward price and volatility
///     beta: CEV exponent (typically 0.0-1.0)
///
/// Examples:
///     >>> # Equity market standard (beta=1.0)
///     >>> params = SABRModelParams.equity_standard(0.2, 0.4, -0.3)
///     >>> params.beta
///     1.0
///
///     >>> # Interest rate market standard (beta=0.5)
///     >>> params = SABRModelParams.rates_standard(0.01, 0.2, 0.1)
///     >>> params.beta
///     0.5
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SABRModelParams",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PySABRModelParams {
    pub(crate) inner: SABRModelParamsData,
}

impl PySABRModelParams {
    pub(crate) fn new(inner: SABRModelParamsData) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySABRModelParams {
    #[new]
    #[pyo3(text_signature = "(alpha, nu, rho, beta)")]
    /// Create SABR model parameters with explicit values.
    ///
    /// Args:
    ///     alpha: Initial volatility level (must be positive)
    ///     nu: Volatility of volatility (typically 0.1 to 1.0)
    ///     rho: Correlation between forward and volatility (typically -0.9 to 0.9)
    ///     beta: CEV exponent (typically 0.0 for normal, 1.0 for lognormal, 0.5 for rates)
    ///
    /// Returns:
    ///     SABRModelParams: Configured SABR parameters
    ///
    /// Raises:
    ///     ValueError: If parameters are out of reasonable ranges
    fn ctor(alpha: f64, nu: f64, rho: f64, beta: f64) -> PyResult<Self> {
        if alpha <= 0.0 {
            return Err(PyValueError::new_err("alpha must be positive"));
        }
        if nu < 0.0 {
            return Err(PyValueError::new_err("nu must be non-negative"));
        }
        if !(-1.0..=1.0).contains(&rho) {
            return Err(PyValueError::new_err("rho must be in [-1, 1]"));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(PyValueError::new_err("beta must be in [0, 1]"));
        }
        Ok(Self::new(SABRModelParamsData::new(alpha, nu, rho, beta)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, alpha, nu, rho)")]
    /// Create SABR parameters with equity market standard (beta=1.0).
    ///
    /// Equity options typically use lognormal dynamics with beta=1.0.
    ///
    /// Args:
    ///     alpha: Initial volatility level
    ///     nu: Volatility of volatility
    ///     rho: Correlation parameter
    ///
    /// Returns:
    ///     SABRModelParams: Parameters with beta=1.0
    fn equity_standard(_cls: &Bound<'_, PyType>, alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(SABRModelParamsData::equity_standard(alpha, nu, rho))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, alpha, nu, rho)")]
    /// Create SABR parameters with interest rate market standard (beta=0.5).
    ///
    /// Interest rate options typically use beta=0.5 to capture the behavior
    /// of rates volatility at different strike levels.
    ///
    /// Args:
    ///     alpha: Initial volatility level
    ///     nu: Volatility of volatility
    ///     rho: Correlation parameter
    ///
    /// Returns:
    ///     SABRModelParams: Parameters with beta=0.5
    fn rates_standard(_cls: &Bound<'_, PyType>, alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(SABRModelParamsData::rates_standard(alpha, nu, rho))
    }

    /// Initial volatility level.
    ///
    /// Returns:
    ///     float: Alpha parameter
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }

    /// Volatility of volatility.
    ///
    /// Returns:
    ///     float: Nu parameter
    #[getter]
    fn nu(&self) -> f64 {
        self.inner.nu
    }

    /// Correlation between forward price and volatility.
    ///
    /// Returns:
    ///     float: Rho parameter in [-1, 1]
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }

    /// CEV exponent controlling volatility behavior across strikes.
    ///
    /// Returns:
    ///     float: Beta parameter in [0, 1]
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }

    fn __repr__(&self) -> String {
        format!(
            "SABRModelParams(alpha={:.6}, nu={:.6}, rho={:.6}, beta={:.2})",
            self.inner.alpha, self.inner.nu, self.inner.rho, self.inner.beta
        )
    }
}

/// Market data required for SABR model calibration.
///
/// Contains the market quotes and configuration needed to calibrate
/// SABR parameters (alpha, nu, rho) for a given expiry slice.
///
/// Examples:
///     >>> market_data = SABRMarketData(
///     ...     forward=100.0,
///     ...     time_to_expiry=1.0,
///     ...     strikes=[80.0, 90.0, 100.0, 110.0, 120.0],
///     ...     market_vols=[0.25, 0.22, 0.20, 0.22, 0.25],
///     ...     beta=1.0
///     ... )
///     >>> market_data.forward
///     100.0
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SABRMarketData",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PySABRMarketData {
    pub(crate) inner: SABRMarketData,
}

impl PySABRMarketData {
    pub(crate) fn new(inner: SABRMarketData) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySABRMarketData {
    #[new]
    #[pyo3(text_signature = "(forward, time_to_expiry, strikes, market_vols, beta)")]
    /// Create market data for SABR calibration.
    ///
    /// Args:
    ///     forward: Forward price of the underlying
    ///     time_to_expiry: Time to expiry in years
    ///     strikes: List of strike prices
    ///     market_vols: List of market implied volatilities (same length as strikes)
    ///     beta: Fixed beta parameter for calibration
    ///
    /// Returns:
    ///     SABRMarketData: Market data bundle for calibration
    ///
    /// Raises:
    ///     ValueError: If strikes and market_vols have different lengths,
    ///                 or if any values are invalid
    fn ctor(
        forward: f64,
        time_to_expiry: f64,
        strikes: Vec<f64>,
        market_vols: Vec<f64>,
        beta: f64,
    ) -> PyResult<Self> {
        if forward <= 0.0 {
            return Err(PyValueError::new_err("forward must be positive"));
        }
        if time_to_expiry <= 0.0 {
            return Err(PyValueError::new_err("time_to_expiry must be positive"));
        }
        if strikes.len() != market_vols.len() {
            return Err(PyValueError::new_err(
                "strikes and market_vols must have the same length",
            ));
        }
        if strikes.is_empty() {
            return Err(PyValueError::new_err("strikes cannot be empty"));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(PyValueError::new_err("beta must be in [0, 1]"));
        }

        Ok(Self::new(SABRMarketData {
            forward,
            time_to_expiry,
            strikes,
            market_vols,
            beta,
            shift: None, // Default to None, could be exposed in future if needed
        }))
    }

    /// Forward price of the underlying.
    ///
    /// Returns:
    ///     float: Forward price
    #[getter]
    fn forward(&self) -> f64 {
        self.inner.forward
    }

    /// Time to expiry in years.
    ///
    /// Returns:
    ///     float: Time to expiry
    #[getter]
    fn time_to_expiry(&self) -> f64 {
        self.inner.time_to_expiry
    }

    /// Strike prices for calibration.
    ///
    /// Returns:
    ///     list[float]: Strike prices
    #[getter]
    fn strikes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyList::new(py, &self.inner.strikes)?.into())
    }

    /// Market implied volatilities corresponding to strikes.
    ///
    /// Returns:
    ///     list[float]: Market volatilities
    #[getter]
    fn market_vols(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyList::new(py, &self.inner.market_vols)?.into())
    }

    /// Fixed beta parameter for calibration.
    ///
    /// Returns:
    ///     float: Beta parameter
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }

    fn __repr__(&self) -> String {
        format!(
            "SABRMarketData(forward={:.2}, time_to_expiry={:.2}, strikes={}, beta={:.2})",
            self.inner.forward,
            self.inner.time_to_expiry,
            self.inner.strikes.len(),
            self.inner.beta
        )
    }
}

/// Analytical derivatives provider for SABR calibration.
///
/// Provides exact gradients for SABR implied volatility with respect to
/// model parameters (alpha, nu, rho), significantly accelerating calibration
/// convergence compared to finite-difference approximations.
///
/// This is used internally by optimization algorithms when calibrating
/// SABR parameters to market data.
///
/// Examples:
///     >>> market_data = SABRMarketData(
///     ...     forward=100.0,
///     ...     time_to_expiry=1.0,
///     ...     strikes=[90.0, 100.0, 110.0],
///     ...     market_vols=[0.22, 0.20, 0.22],
///     ...     beta=1.0
///     ... )
///     >>> derivatives = SABRCalibrationDerivatives(market_data)
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SABRCalibrationDerivatives",
    frozen
)]
pub struct PySABRCalibrationDerivatives {
    #[allow(dead_code)]
    pub(crate) inner: SABRCalibrationDerivatives,
}

impl PySABRCalibrationDerivatives {
    pub(crate) fn new(inner: SABRCalibrationDerivatives) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySABRCalibrationDerivatives {
    #[new]
    #[pyo3(text_signature = "(market_data)")]
    /// Create a derivatives provider for SABR calibration.
    ///
    /// Args:
    ///     market_data: Market data containing forward, strikes, and volatilities
    ///
    /// Returns:
    ///     SABRCalibrationDerivatives: Provider for analytical derivatives
    fn ctor(market_data: &PySABRMarketData) -> Self {
        Self::new(SABRCalibrationDerivatives::new(market_data.inner.clone()))
    }

    fn __repr__(&self) -> String {
        "SABRCalibrationDerivatives(...)".to_string()
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySABRModelParams>()?;
    module.add_class::<PySABRMarketData>()?;
    module.add_class::<PySABRCalibrationDerivatives>()?;
    Ok(vec![
        "SABRModelParams",
        "SABRMarketData",
        "SABRCalibrationDerivatives",
    ])
}
