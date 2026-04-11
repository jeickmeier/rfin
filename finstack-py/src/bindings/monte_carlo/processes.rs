//! Stochastic process bindings.
//!
//! These types hold parameters for Python construction and getter access.
//! The actual Rust process objects are constructed on-demand at pricing time.

#![allow(dead_code)]

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// GBM
// ---------------------------------------------------------------------------

/// Geometric Brownian Motion process parameters.
#[pyclass(name = "GbmProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyGbmProcess {
    pub(super) rate: f64,
    pub(super) div_yield: f64,
    pub(super) vol: f64,
}

#[pymethods]
impl PyGbmProcess {
    #[new]
    #[pyo3(text_signature = "(rate, div_yield, vol)")]
    fn new(rate: f64, div_yield: f64, vol: f64) -> Self {
        Self {
            rate,
            div_yield,
            vol,
        }
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.rate
    }
    #[getter]
    fn div_yield(&self) -> f64 {
        self.div_yield
    }
    #[getter]
    fn vol(&self) -> f64 {
        self.vol
    }

    fn __repr__(&self) -> String {
        format!(
            "GbmProcess(rate={}, div_yield={}, vol={})",
            self.rate, self.div_yield, self.vol,
        )
    }
}

// ---------------------------------------------------------------------------
// MultiGBM
// ---------------------------------------------------------------------------

/// Multi-asset GBM process with correlation.
#[pyclass(name = "MultiGbmProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyMultiGbmProcess {
    pub(super) rates: Vec<f64>,
    pub(super) div_yields: Vec<f64>,
    pub(super) vols: Vec<f64>,
    pub(super) correlation: Vec<f64>,
}

#[pymethods]
impl PyMultiGbmProcess {
    /// Create a multi-asset GBM process.
    ///
    /// ``correlation`` is a flat row-major correlation matrix of size ``n*n``.
    #[new]
    #[pyo3(text_signature = "(rates, div_yields, vols, correlation)")]
    fn new(rates: Vec<f64>, div_yields: Vec<f64>, vols: Vec<f64>, correlation: Vec<f64>) -> Self {
        Self {
            rates,
            div_yields,
            vols,
            correlation,
        }
    }

    /// Number of assets.
    #[getter]
    fn num_assets(&self) -> usize {
        self.rates.len()
    }

    fn __repr__(&self) -> String {
        format!("MultiGbmProcess(assets={})", self.rates.len())
    }
}

// ---------------------------------------------------------------------------
// Brownian
// ---------------------------------------------------------------------------

/// Arithmetic Brownian Motion process.
#[pyclass(name = "BrownianProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyBrownianProcess {
    pub(super) mu: f64,
    pub(super) sigma: f64,
}

#[pymethods]
impl PyBrownianProcess {
    #[new]
    #[pyo3(text_signature = "(mu, sigma)")]
    fn new(mu: f64, sigma: f64) -> Self {
        Self { mu, sigma }
    }

    #[getter]
    fn mu(&self) -> f64 {
        self.mu
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.sigma
    }

    fn __repr__(&self) -> String {
        format!("BrownianProcess(mu={}, sigma={})", self.mu, self.sigma)
    }
}

// ---------------------------------------------------------------------------
// Heston
// ---------------------------------------------------------------------------

/// Heston stochastic volatility model.
#[pyclass(name = "HestonProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyHestonProcess {
    pub(super) rate: f64,
    pub(super) div_yield: f64,
    pub(super) v0: f64,
    pub(super) kappa: f64,
    pub(super) theta: f64,
    pub(super) xi: f64,
    pub(super) rho: f64,
}

#[pymethods]
impl PyHestonProcess {
    /// Create a Heston stochastic volatility process.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Risk-free rate.
    /// div_yield : float
    ///     Dividend yield.
    /// v0 : float
    ///     Initial variance.
    /// kappa : float
    ///     Mean reversion speed.
    /// theta : float
    ///     Long-run variance.
    /// xi : float
    ///     Vol-of-vol.
    /// rho : float
    ///     Spot-vol correlation.
    #[new]
    #[pyo3(text_signature = "(rate, div_yield, v0, kappa, theta, xi, rho)")]
    fn new(rate: f64, div_yield: f64, v0: f64, kappa: f64, theta: f64, xi: f64, rho: f64) -> Self {
        Self {
            rate,
            div_yield,
            v0,
            kappa,
            theta,
            xi,
            rho,
        }
    }

    /// Whether the Feller condition (2*kappa*theta > xi^2) is satisfied.
    #[getter]
    fn satisfies_feller(&self) -> bool {
        2.0 * self.kappa * self.theta > self.xi * self.xi
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.rate
    }
    #[getter]
    fn div_yield(&self) -> f64 {
        self.div_yield
    }
    #[getter]
    fn v0(&self) -> f64 {
        self.v0
    }
    #[getter]
    fn kappa(&self) -> f64 {
        self.kappa
    }
    #[getter]
    fn theta(&self) -> f64 {
        self.theta
    }
    #[getter]
    fn xi(&self) -> f64 {
        self.xi
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.rho
    }

    fn __repr__(&self) -> String {
        format!(
            "HestonProcess(v0={:.4}, kappa={:.4}, theta={:.4}, xi={:.4}, rho={:.4})",
            self.v0, self.kappa, self.theta, self.xi, self.rho,
        )
    }
}

// ---------------------------------------------------------------------------
// CIR
// ---------------------------------------------------------------------------

/// Cox-Ingersoll-Ross process.
#[pyclass(name = "CirProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyCirProcess {
    pub(super) kappa: f64,
    pub(super) theta: f64,
    pub(super) sigma: f64,
    pub(super) x0: f64,
}

#[pymethods]
impl PyCirProcess {
    #[new]
    #[pyo3(text_signature = "(kappa, theta, sigma, x0)")]
    fn new(kappa: f64, theta: f64, sigma: f64, x0: f64) -> Self {
        Self {
            kappa,
            theta,
            sigma,
            x0,
        }
    }

    /// Whether the Feller condition is satisfied.
    #[getter]
    fn satisfies_feller(&self) -> bool {
        2.0 * self.kappa * self.theta > self.sigma * self.sigma
    }

    #[getter]
    fn kappa(&self) -> f64 {
        self.kappa
    }
    #[getter]
    fn theta(&self) -> f64 {
        self.theta
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.sigma
    }
    #[getter]
    fn x0(&self) -> f64 {
        self.x0
    }

    fn __repr__(&self) -> String {
        format!(
            "CirProcess(kappa={:.4}, theta={:.4}, sigma={:.4}, x0={:.4})",
            self.kappa, self.theta, self.sigma, self.x0,
        )
    }
}

// ---------------------------------------------------------------------------
// Merton Jump Diffusion
// ---------------------------------------------------------------------------

/// Merton jump-diffusion process.
#[pyclass(name = "MertonJumpProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyMertonJumpProcess {
    pub(super) rate: f64,
    pub(super) div_yield: f64,
    pub(super) sigma: f64,
    pub(super) jump_intensity: f64,
    pub(super) jump_mean: f64,
    pub(super) jump_vol: f64,
}

#[pymethods]
impl PyMertonJumpProcess {
    #[new]
    #[pyo3(text_signature = "(rate, div_yield, sigma, jump_intensity, jump_mean, jump_vol)")]
    fn new(
        rate: f64,
        div_yield: f64,
        sigma: f64,
        jump_intensity: f64,
        jump_mean: f64,
        jump_vol: f64,
    ) -> Self {
        Self {
            rate,
            div_yield,
            sigma,
            jump_intensity,
            jump_mean,
            jump_vol,
        }
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.rate
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.sigma
    }
    #[getter]
    fn jump_intensity(&self) -> f64 {
        self.jump_intensity
    }
    #[getter]
    fn jump_mean(&self) -> f64 {
        self.jump_mean
    }
    #[getter]
    fn jump_vol(&self) -> f64 {
        self.jump_vol
    }

    fn __repr__(&self) -> String {
        format!(
            "MertonJumpProcess(sigma={:.4}, lambda={:.2}, jm={:.4}, jv={:.4})",
            self.sigma, self.jump_intensity, self.jump_mean, self.jump_vol,
        )
    }
}

// ---------------------------------------------------------------------------
// Bates (Heston + Jumps)
// ---------------------------------------------------------------------------

/// Bates model (Heston + Merton jumps).
#[pyclass(name = "BatesProcess", module = "finstack.monte_carlo", frozen)]
pub struct PyBatesProcess {
    pub(super) rate: f64,
    pub(super) div_yield: f64,
    pub(super) v0: f64,
    pub(super) kappa: f64,
    pub(super) theta: f64,
    pub(super) xi: f64,
    pub(super) rho: f64,
    pub(super) jump_intensity: f64,
    pub(super) jump_mean: f64,
    pub(super) jump_vol: f64,
}

#[pymethods]
impl PyBatesProcess {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(rate, div_yield, v0, kappa, theta, xi, rho, jump_intensity, jump_mean, jump_vol)"
    )]
    fn new(
        rate: f64,
        div_yield: f64,
        v0: f64,
        kappa: f64,
        theta: f64,
        xi: f64,
        rho: f64,
        jump_intensity: f64,
        jump_mean: f64,
        jump_vol: f64,
    ) -> Self {
        Self {
            rate,
            div_yield,
            v0,
            kappa,
            theta,
            xi,
            rho,
            jump_intensity,
            jump_mean,
            jump_vol,
        }
    }

    #[getter]
    fn v0(&self) -> f64 {
        self.v0
    }
    #[getter]
    fn kappa(&self) -> f64 {
        self.kappa
    }
    #[getter]
    fn theta(&self) -> f64 {
        self.theta
    }
    #[getter]
    fn xi(&self) -> f64 {
        self.xi
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.rho
    }
    #[getter]
    fn jump_intensity(&self) -> f64 {
        self.jump_intensity
    }

    fn __repr__(&self) -> String {
        format!(
            "BatesProcess(v0={:.4}, kappa={:.4}, theta={:.4}, xi={:.4}, rho={:.4}, lam={:.2})",
            self.v0, self.kappa, self.theta, self.xi, self.rho, self.jump_intensity,
        )
    }
}

// ---------------------------------------------------------------------------
// Schwartz-Smith
// ---------------------------------------------------------------------------

/// Schwartz-Smith two-factor commodity model.
#[pyclass(name = "SchwartzSmithProcess", module = "finstack.monte_carlo", frozen)]
pub struct PySchwartzSmithProcess {
    pub(super) kappa: f64,
    pub(super) sigma_chi: f64,
    pub(super) sigma_xi: f64,
    pub(super) rho: f64,
    pub(super) mu_xi: f64,
    pub(super) lambda_chi: f64,
}

#[pymethods]
impl PySchwartzSmithProcess {
    #[new]
    #[pyo3(text_signature = "(kappa, sigma_chi, sigma_xi, rho, mu_xi, lambda_chi)")]
    fn new(
        kappa: f64,
        sigma_chi: f64,
        sigma_xi: f64,
        rho: f64,
        mu_xi: f64,
        lambda_chi: f64,
    ) -> Self {
        Self {
            kappa,
            sigma_chi,
            sigma_xi,
            rho,
            mu_xi,
            lambda_chi,
        }
    }

    #[getter]
    fn kappa(&self) -> f64 {
        self.kappa
    }
    #[getter]
    fn sigma_chi(&self) -> f64 {
        self.sigma_chi
    }
    #[getter]
    fn sigma_xi(&self) -> f64 {
        self.sigma_xi
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.rho
    }

    fn __repr__(&self) -> String {
        format!(
            "SchwartzSmithProcess(kappa={:.4}, sigma_chi={:.4}, sigma_xi={:.4}, rho={:.4})",
            self.kappa, self.sigma_chi, self.sigma_xi, self.rho,
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGbmProcess>()?;
    m.add_class::<PyMultiGbmProcess>()?;
    m.add_class::<PyBrownianProcess>()?;
    m.add_class::<PyHestonProcess>()?;
    m.add_class::<PyCirProcess>()?;
    m.add_class::<PyMertonJumpProcess>()?;
    m.add_class::<PyBatesProcess>()?;
    m.add_class::<PySchwartzSmithProcess>()?;
    Ok(())
}
