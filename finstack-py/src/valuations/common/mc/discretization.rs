//! Python bindings for Monte Carlo discretization scheme descriptors.
//!
//! These are lightweight Python-visible types that describe which discretization
//! scheme to use. The actual Rust trait-based discretization types are generic
//! and cannot be directly exposed as pyclass. These descriptors serve as
//! configuration objects that the path generator can interpret.

use pyo3::prelude::*;

/// Exact discretization for Geometric Brownian Motion.
///
/// Uses the analytical log-normal solution (no discretization error):
///
///   S_{t+dt} = S_t * exp((r - q - 0.5*sigma^2)*dt + sigma*sqrt(dt)*Z)
///
/// This is the recommended scheme for GBM processes.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "ExactGbmScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExactGbmScheme;

#[pymethods]
impl PyExactGbmScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "ExactGbmScheme()"
    }
}

/// Euler-Maruyama discretization (first-order explicit).
///
/// Generic scheme for any SDE:
///   X_{t+dt} = X_t + mu(t, X_t)*dt + sigma(t, X_t)*sqrt(dt)*Z
///
/// Properties:
///   - Weak order: O(dt)
///   - Strong order: O(sqrt(dt))
///
/// Use when no exact or specialized scheme is available.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "EulerMaruyamaScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEulerMaruyamaScheme;

#[pymethods]
impl PyEulerMaruyamaScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "EulerMaruyamaScheme()"
    }
}

/// Log-Euler discretization.
///
/// Euler scheme applied in log-space for processes with multiplicative noise.
/// More stable than standard Euler for GBM-like processes but less accurate
/// than the exact scheme.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "LogEulerScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyLogEulerScheme;

#[pymethods]
impl PyLogEulerScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "LogEulerScheme()"
    }
}

/// Milstein discretization (higher-order strong convergence).
///
/// Adds a correction term to Euler-Maruyama:
///   X_{t+dt} = X_t + mu*dt + sigma*sqrt(dt)*Z + 0.5*sigma*sigma'*(Z^2 - 1)*dt
///
/// Properties:
///   - Weak order: O(dt) (same as Euler)
///   - Strong order: O(dt) (better than Euler's O(sqrt(dt)))
///
/// Note: Only exact for processes with proportional volatility (GBM-like).
/// For CIR or OU processes, use dedicated schemes instead.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "MilsteinScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMilsteinScheme;

#[pymethods]
impl PyMilsteinScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "MilsteinScheme()"
    }
}

/// Log-Milstein discretization.
///
/// Milstein scheme applied in log-space. More stable for GBM-like processes.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "LogMilsteinScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyLogMilsteinScheme;

#[pymethods]
impl PyLogMilsteinScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "LogMilsteinScheme()"
    }
}

/// Quadratic-Exponential (QE) scheme for Heston stochastic volatility.
///
/// Andersen (2008) scheme that ensures positive variance while maintaining
/// good accuracy. Handles both the variance (CIR process) and spot price.
///
/// Args:
///     psi_c: Critical psi value (default 1.5). Controls the switch between
///            power/gamma and exponential/uniform mixture approximations.
///     use_exact_integrated_variance: If True, uses exact conditional expectation
///            for integrated variance. More accurate for high mean-reversion
///            or coarse time steps. Default: False (uses trapezoidal).
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "QeHestonScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyQeHestonScheme {
    psi_c: f64,
    use_exact_integrated_variance: bool,
}

#[pymethods]
impl PyQeHestonScheme {
    #[new]
    #[pyo3(signature = (psi_c=1.5, use_exact_integrated_variance=false))]
    fn new(psi_c: f64, use_exact_integrated_variance: bool) -> Self {
        Self {
            psi_c,
            use_exact_integrated_variance,
        }
    }

    /// Critical psi value for the QE switch.
    #[getter]
    fn psi_c(&self) -> f64 {
        self.psi_c
    }

    /// Whether exact integrated variance is used.
    #[getter]
    fn use_exact_integrated_variance(&self) -> bool {
        self.use_exact_integrated_variance
    }

    fn __repr__(&self) -> String {
        format!(
            "QeHestonScheme(psi_c={}, exact_iv={})",
            self.psi_c, self.use_exact_integrated_variance
        )
    }
}

/// Quadratic-Exponential (QE) scheme for CIR process.
///
/// Extracted from the Heston QE scheme, adapted for standalone CIR processes.
/// Ensures positive values while maintaining accuracy.
///
/// Args:
///     psi_c: Critical psi value (default 1.5)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "QeCirScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyQeCirScheme {
    psi_c: f64,
}

#[pymethods]
impl PyQeCirScheme {
    #[new]
    #[pyo3(signature = (psi_c=1.5))]
    fn new(psi_c: f64) -> Self {
        Self { psi_c }
    }

    #[getter]
    fn psi_c(&self) -> f64 {
        self.psi_c
    }

    fn __repr__(&self) -> String {
        format!("QeCirScheme(psi_c={})", self.psi_c)
    }
}

/// Exact discretization for Hull-White one-factor model.
///
/// Uses the analytical solution for the OU process:
///   r_{t+dt} = r_t * exp(-kappa*dt) + theta*(1 - exp(-kappa*dt))
///              + sigma * sqrt((1 - exp(-2*kappa*dt)) / (2*kappa)) * Z
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "ExactHullWhite1FScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExactHullWhite1FScheme;

#[pymethods]
impl PyExactHullWhite1FScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "ExactHullWhite1FScheme()"
    }
}

/// Jump-Euler discretization for jump-diffusion processes.
///
/// Combines Euler-Maruyama for the continuous part with Poisson arrival
/// and log-normal jumps for the jump component.
///
/// Suitable for Merton jump-diffusion and Bates models.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "JumpEulerScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyJumpEulerScheme;

#[pymethods]
impl PyJumpEulerScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "JumpEulerScheme()"
    }
}

/// Exact discretization for Schwartz-Smith two-factor model.
///
/// Uses the analytical solution for the coupled OU + ABM system
/// with correlation.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "ExactSchwartzSmithScheme",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExactSchwartzSmithScheme;

#[pymethods]
impl PyExactSchwartzSmithScheme {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "ExactSchwartzSmithScheme()"
    }
}
