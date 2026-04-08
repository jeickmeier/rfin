//! Python bindings for stochastic volatility model parameters.
//!
//! Exposes Heston, SABR, and SVI model types from `finstack_core::math::volatility`
//! as frozen `#[pyclass]` structs with constructors, getters, and pricing/vol methods.

use finstack_core::math::volatility::heston::HestonParams;
use finstack_core::math::volatility::sabr::SabrParams;
use finstack_core::math::volatility::svi::SviParams;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

// ============================================================================
// HestonParams
// ============================================================================

/// Heston stochastic volatility model parameters.
///
/// The Heston model describes joint dynamics of an asset price and its
/// instantaneous variance using five parameters.
///
/// Parameters
/// ----------
/// v0 : float
///     Initial variance (must be > 0).
/// kappa : float
///     Mean reversion speed (must be > 0).
/// theta : float
///     Long-run variance level (must be > 0).
/// sigma : float
///     Vol-of-vol (must be > 0).
/// rho : float
///     Correlation between spot and variance, in (-1, 1).
///
/// Raises
/// ------
/// ValueError
///     If any parameter is out of range.
#[pyclass(
    name = "HestonParams",
    module = "finstack.core.volatility_models",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyHestonParams {
    pub(crate) inner: HestonParams,
}

#[pymethods]
impl PyHestonParams {
    #[new]
    #[pyo3(text_signature = "(v0, kappa, theta, sigma, rho)")]
    fn new(v0: f64, kappa: f64, theta: f64, sigma: f64, rho: f64) -> PyResult<Self> {
        HestonParams::new(v0, kappa, theta, sigma, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Initial variance (v0).
    #[getter]
    fn v0(&self) -> f64 {
        self.inner.v0
    }

    /// Mean reversion speed (kappa).
    #[getter]
    fn kappa(&self) -> f64 {
        self.inner.kappa
    }

    /// Long-run variance level (theta).
    #[getter]
    fn theta(&self) -> f64 {
        self.inner.theta
    }

    /// Vol-of-vol (sigma).
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    /// Correlation (rho).
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }

    /// Check whether the Feller condition (2*kappa*theta > sigma^2) is satisfied.
    ///
    /// When satisfied, the variance process is strictly positive almost surely.
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if the Feller condition holds.
    #[pyo3(text_signature = "(self)")]
    fn satisfies_feller_condition(&self) -> bool {
        self.inner.satisfies_feller_condition()
    }

    /// Price a European option using Fourier integration.
    ///
    /// Parameters
    /// ----------
    /// spot : float
    ///     Current spot price.
    /// strike : float
    ///     Strike price.
    /// r : float
    ///     Risk-free rate (continuous compounding).
    /// q : float
    ///     Dividend yield (continuous compounding).
    /// t : float
    ///     Time to expiry in years.
    /// is_call : bool
    ///     True for call, False for put.
    ///
    /// Returns
    /// -------
    /// float
    ///     Option price (non-negative).
    #[pyo3(text_signature = "(self, spot, strike, r, q, t, is_call)")]
    fn price_european(&self, spot: f64, strike: f64, r: f64, q: f64, t: f64, is_call: bool) -> f64 {
        self.inner.price_european(spot, strike, r, q, t, is_call)
    }

    fn __repr__(&self) -> String {
        format!(
            "HestonParams(v0={}, kappa={}, theta={}, sigma={}, rho={})",
            self.inner.v0, self.inner.kappa, self.inner.theta, self.inner.sigma, self.inner.rho,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Deconstruct into tuple (used by pickle).
    fn __getnewargs__(&self) -> (f64, f64, f64, f64, f64) {
        (
            self.inner.v0,
            self.inner.kappa,
            self.inner.theta,
            self.inner.sigma,
            self.inner.rho,
        )
    }
}

// ============================================================================
// SabrParams
// ============================================================================

/// SABR stochastic volatility model parameters.
///
/// The SABR model is the market standard for swaption and cap/floor
/// volatility smile modeling.
///
/// Parameters
/// ----------
/// alpha : float
///     Initial volatility level (must be > 0).
/// beta : float
///     CEV exponent, in [0, 1].
/// rho : float
///     Correlation, in (-1, 1).
/// nu : float
///     Vol-of-vol (must be > 0).
///
/// Raises
/// ------
/// ValueError
///     If any parameter is out of range.
#[pyclass(
    name = "SabrParams",
    module = "finstack.core.volatility_models",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySabrParams {
    pub(crate) inner: SabrParams,
}

#[pymethods]
impl PySabrParams {
    #[new]
    #[pyo3(text_signature = "(alpha, beta, rho, nu)")]
    fn new(alpha: f64, beta: f64, rho: f64, nu: f64) -> PyResult<Self> {
        SabrParams::new(alpha, beta, rho, nu)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Initial volatility level (alpha).
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }

    /// CEV exponent (beta).
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }

    /// Correlation (rho).
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }

    /// Vol-of-vol (nu).
    #[getter]
    fn nu(&self) -> f64 {
        self.inner.nu
    }

    /// Lognormal (Black-76) implied volatility using Hagan's approximation.
    ///
    /// Parameters
    /// ----------
    /// f : float
    ///     Forward rate.
    /// k : float
    ///     Strike rate.
    /// t : float
    ///     Time to expiry in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Black-76 implied volatility.
    #[pyo3(text_signature = "(self, f, k, t)")]
    fn implied_vol_lognormal(&self, f: f64, k: f64, t: f64) -> f64 {
        self.inner.implied_vol_lognormal(f, k, t)
    }

    /// Normal (Bachelier) implied volatility using Hagan's approximation.
    ///
    /// Parameters
    /// ----------
    /// f : float
    ///     Forward rate (may be negative).
    /// k : float
    ///     Strike rate (may be negative).
    /// t : float
    ///     Time to expiry in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Normal/Bachelier implied volatility.
    #[pyo3(text_signature = "(self, f, k, t)")]
    fn implied_vol_normal(&self, f: f64, k: f64, t: f64) -> f64 {
        self.inner.implied_vol_normal(f, k, t)
    }

    fn __repr__(&self) -> String {
        format!(
            "SabrParams(alpha={}, beta={}, rho={}, nu={})",
            self.inner.alpha, self.inner.beta, self.inner.rho, self.inner.nu,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Deconstruct into tuple (used by pickle).
    fn __getnewargs__(&self) -> (f64, f64, f64, f64) {
        (
            self.inner.alpha,
            self.inner.beta,
            self.inner.rho,
            self.inner.nu,
        )
    }
}

// ============================================================================
// SviParams
// ============================================================================

/// SVI (Stochastic Volatility Inspired) raw parameterization.
///
/// Represents one slice of the volatility surface at a fixed expiry using
/// five parameters that control the shape of the smile.
///
/// Parameters
/// ----------
/// a : float
///     Overall variance level.
/// b : float
///     Slope of the wings (must be >= 0).
/// rho : float
///     Rotation / asymmetry, in (-1, 1).
/// m : float
///     Translation (shift of minimum variance point).
/// sigma : float
///     Smoothing parameter (must be > 0).
///
/// Raises
/// ------
/// ValueError
///     If parameters violate no-arbitrage conditions.
#[pyclass(
    name = "SviParams",
    module = "finstack.core.volatility_models",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySviParams {
    pub(crate) inner: SviParams,
}

#[pymethods]
impl PySviParams {
    #[new]
    #[pyo3(text_signature = "(a, b, rho, m, sigma)")]
    fn new(a: f64, b: f64, rho: f64, m: f64, sigma: f64) -> PyResult<Self> {
        let params = SviParams {
            a,
            b,
            rho,
            m,
            sigma,
        };
        params
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: params })
    }

    /// Overall variance level (a).
    #[getter]
    fn a(&self) -> f64 {
        self.inner.a
    }

    /// Slope of the wings (b).
    #[getter]
    fn b(&self) -> f64 {
        self.inner.b
    }

    /// Rotation / asymmetry (rho).
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }

    /// Translation (m).
    #[getter]
    fn m(&self) -> f64 {
        self.inner.m
    }

    /// Smoothing parameter (sigma).
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    /// Compute the total implied variance w(k) at log-moneyness k.
    ///
    /// Parameters
    /// ----------
    /// k : float
    ///     Log-moneyness, ln(K/F).
    ///
    /// Returns
    /// -------
    /// float
    ///     Total implied variance w(k) = sigma^2 * T.
    #[pyo3(text_signature = "(self, k)")]
    fn total_variance(&self, k: f64) -> f64 {
        self.inner.total_variance(k)
    }

    /// Compute Black-Scholes implied volatility from SVI total variance.
    ///
    /// Parameters
    /// ----------
    /// k : float
    ///     Log-moneyness, ln(K/F).
    /// t : float
    ///     Time to expiry in years (must be > 0).
    ///
    /// Returns
    /// -------
    /// float
    ///     Implied volatility. Returns NaN if t <= 0 or total variance is negative.
    #[pyo3(text_signature = "(self, k, t)")]
    fn implied_vol(&self, k: f64, t: f64) -> f64 {
        self.inner.implied_vol(k, t)
    }

    /// Validate SVI parameters against no-arbitrage constraints.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If parameters violate no-arbitrage conditions.
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SviParams(a={}, b={}, rho={}, m={}, sigma={})",
            self.inner.a, self.inner.b, self.inner.rho, self.inner.m, self.inner.sigma,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Deconstruct into tuple (used by pickle).
    fn __getnewargs__(&self) -> (f64, f64, f64, f64, f64) {
        (
            self.inner.a,
            self.inner.b,
            self.inner.rho,
            self.inner.m,
            self.inner.sigma,
        )
    }
}

// ============================================================================
// Module registration
// ============================================================================

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "volatility_models")?;
    module.setattr(
        "__doc__",
        concat!(
            "Stochastic volatility model parameter types.\n\n",
            "Provides:\n",
            "- HestonParams: Heston (1993) stochastic volatility model\n",
            "- SabrParams: SABR (Hagan 2002) stochastic alpha-beta-rho model\n",
            "- SviParams: SVI (Gatheral 2004) implied variance parameterization\n",
        ),
    )?;

    module.add_class::<PyHestonParams>()?;
    module.add_class::<PySabrParams>()?;
    module.add_class::<PySviParams>()?;

    let exports = ["HestonParams", "SabrParams", "SviParams"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
