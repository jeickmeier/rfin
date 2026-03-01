//! Python bindings for stochastic process types.
//!
//! Exposes all stochastic processes from the Rust MC infrastructure:
//! GBM, Heston, CIR, Hull-White, Merton Jump-Diffusion, Bates,
//! Ornstein-Uhlenbeck, Schwartz-Smith, and Brownian motion.

use pyo3::prelude::*;

// ============================================================================
// GBM Process
// ============================================================================

/// Parameters for Geometric Brownian Motion.
///
/// SDE: dS_t = (r - q) S_t dt + sigma S_t dW_t
///
/// Args:
///     r: Risk-free rate (annual)
///     q: Dividend/foreign rate (annual)
///     sigma: Volatility (annual)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "GbmParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyGbmParams {
    pub(crate) inner: finstack_valuations::instruments::common::mc::process::gbm::GbmParams,
}

#[pymethods]
impl PyGbmParams {
    #[new]
    fn new(r: f64, q: f64, sigma: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::gbm::GbmParams;
        Self {
            inner: GbmParams::new(r, q, sigma),
        }
    }

    /// Risk-free rate.
    #[getter]
    fn r(&self) -> f64 {
        self.inner.r
    }

    /// Dividend/foreign rate.
    #[getter]
    fn q(&self) -> f64 {
        self.inner.q
    }

    /// Volatility.
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    fn __repr__(&self) -> String {
        format!(
            "GbmParams(r={}, q={}, sigma={})",
            self.inner.r, self.inner.q, self.inner.sigma
        )
    }
}

// ============================================================================
// Heston Process
// ============================================================================

/// Parameters for the Heston stochastic volatility model.
///
/// SDE:
///   dS_t = (r - q) S_t dt + sqrt(v_t) S_t dW1_t
///   dv_t = kappa (theta - v_t) dt + sigma_v sqrt(v_t) dW2_t
///   Corr(dW1, dW2) = rho
///
/// Args:
///     r: Risk-free rate
///     q: Dividend yield
///     kappa: Mean reversion speed (> 0)
///     theta: Long-term variance (> 0)
///     sigma_v: Vol-of-vol (> 0)
///     rho: Correlation between asset and variance in [-1, 1]
///     v0: Initial variance (> 0)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "HestonParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHestonParams {
    pub(crate) inner: finstack_valuations::instruments::common::mc::process::heston::HestonParams,
}

#[pymethods]
impl PyHestonParams {
    #[new]
    fn new(r: f64, q: f64, kappa: f64, theta: f64, sigma_v: f64, rho: f64, v0: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::heston::HestonParams;
        Self {
            inner: HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0),
        }
    }

    #[getter]
    fn r(&self) -> f64 {
        self.inner.r
    }
    #[getter]
    fn q(&self) -> f64 {
        self.inner.q
    }
    #[getter]
    fn kappa(&self) -> f64 {
        self.inner.kappa
    }
    #[getter]
    fn theta(&self) -> f64 {
        self.inner.theta
    }
    #[getter]
    fn sigma_v(&self) -> f64 {
        self.inner.sigma_v
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }
    #[getter]
    fn v0(&self) -> f64 {
        self.inner.v0
    }

    /// Check if the Feller condition (2*kappa*theta >= sigma_v^2) is satisfied.
    ///
    /// When satisfied, the variance process stays strictly positive.
    fn satisfies_feller(&self) -> bool {
        self.inner.satisfies_feller()
    }

    fn __repr__(&self) -> String {
        format!(
            "HestonParams(r={}, q={}, kappa={}, theta={}, sigma_v={}, rho={}, v0={}, feller={})",
            self.inner.r,
            self.inner.q,
            self.inner.kappa,
            self.inner.theta,
            self.inner.sigma_v,
            self.inner.rho,
            self.inner.v0,
            self.inner.satisfies_feller()
        )
    }
}

// ============================================================================
// CIR Process
// ============================================================================

/// Parameters for the CIR (Cox-Ingersoll-Ross) square-root diffusion.
///
/// SDE: dv_t = kappa (theta - v_t) dt + sigma sqrt(v_t) dW_t
///
/// Used for modeling short rates, stochastic volatility, and credit intensities.
///
/// Args:
///     kappa: Mean reversion speed (> 0)
///     theta: Long-term mean (>= 0)
///     sigma: Volatility of volatility (> 0)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "CirParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCirParams {
    pub(crate) inner: finstack_valuations::instruments::common::mc::process::cir::CirParams,
}

#[pymethods]
impl PyCirParams {
    #[new]
    fn new(kappa: f64, theta: f64, sigma: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::cir::CirParams;
        Self {
            inner: CirParams::new(kappa, theta, sigma),
        }
    }

    #[getter]
    fn kappa(&self) -> f64 {
        self.inner.kappa
    }
    #[getter]
    fn theta(&self) -> f64 {
        self.inner.theta
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    /// Check if Feller condition (2*kappa*theta >= sigma^2) is satisfied.
    fn satisfies_feller(&self) -> bool {
        self.inner.satisfies_feller()
    }

    fn __repr__(&self) -> String {
        format!(
            "CirParams(kappa={}, theta={}, sigma={}, feller={})",
            self.inner.kappa,
            self.inner.theta,
            self.inner.sigma,
            self.inner.satisfies_feller()
        )
    }
}

// ============================================================================
// Hull-White 1F Process
// ============================================================================

/// Parameters for the Hull-White one-factor short rate model.
///
/// SDE: dr_t = kappa [theta(t) - r_t] dt + sigma dW_t
///
/// Supports both constant theta (Vasicek model) and time-dependent theta(t)
/// for fitting the initial yield curve.
///
/// Args:
///     kappa: Mean reversion speed
///     sigma: Instantaneous volatility
///     theta: Constant mean reversion level (for Vasicek/constant theta)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "HullWhite1FParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHullWhite1FParams {
    pub(crate) inner: finstack_valuations::instruments::common::mc::process::ou::HullWhite1FParams,
}

#[pymethods]
impl PyHullWhite1FParams {
    /// Create with constant theta (Vasicek model).
    #[new]
    fn new(kappa: f64, sigma: f64, theta: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::ou::HullWhite1FParams;
        Self {
            inner: HullWhite1FParams::new(kappa, sigma, theta),
        }
    }

    /// Create with time-dependent theta(t).
    ///
    /// Args:
    ///     kappa: Mean reversion speed
    ///     sigma: Volatility
    ///     theta_curve: List of theta values (piecewise constant)
    ///     theta_times: List of time breakpoints (must be sorted)
    #[staticmethod]
    fn with_time_dependent_theta(
        kappa: f64,
        sigma: f64,
        theta_curve: Vec<f64>,
        theta_times: Vec<f64>,
    ) -> Self {
        use finstack_valuations::instruments::common::mc::process::ou::HullWhite1FParams;
        Self {
            inner: HullWhite1FParams::with_time_dependent_theta(
                kappa,
                sigma,
                theta_curve,
                theta_times,
            ),
        }
    }

    #[getter]
    fn kappa(&self) -> f64 {
        self.inner.kappa
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    /// Get theta(t) at a given time.
    fn theta_at_time(&self, t: f64) -> f64 {
        self.inner.theta_at_time(t)
    }

    fn __repr__(&self) -> String {
        format!(
            "HullWhite1FParams(kappa={}, sigma={}, theta_points={})",
            self.inner.kappa,
            self.inner.sigma,
            self.inner.theta_curve.len()
        )
    }
}

// ============================================================================
// Merton Jump-Diffusion Process
// ============================================================================

/// Parameters for the Merton jump-diffusion model.
///
/// SDE: dS_t/S_t = (r - q - lambda*k) dt + sigma dW_t + (J-1) dN_t
///
/// where:
///   lambda = jump intensity (average jumps per year)
///   J ~ LogNormal(mu_j, sigma_j^2)
///   k = E[J-1] = exp(mu_j + sigma_j^2/2) - 1
///
/// Args:
///     r: Risk-free rate
///     q: Dividend yield
///     sigma: Continuous diffusion volatility
///     lambda_: Jump intensity (jumps per year)
///     mu_j: Mean of log-jump size
///     sigma_j: Std dev of log-jump size
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "MertonJumpParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMertonJumpParams {
    pub(crate) inner:
        finstack_valuations::instruments::common::mc::process::jump_diffusion::MertonJumpParams,
}

#[pymethods]
impl PyMertonJumpParams {
    #[new]
    fn new(r: f64, q: f64, sigma: f64, lambda_: f64, mu_j: f64, sigma_j: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::jump_diffusion::MertonJumpParams;
        Self {
            inner: MertonJumpParams::new(r, q, sigma, lambda_, mu_j, sigma_j),
        }
    }

    #[getter]
    fn r(&self) -> f64 {
        self.inner.gbm.r
    }
    #[getter]
    fn q(&self) -> f64 {
        self.inner.gbm.q
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.gbm.sigma
    }
    #[getter]
    fn lambda_(&self) -> f64 {
        self.inner.lambda
    }
    #[getter]
    fn mu_j(&self) -> f64 {
        self.inner.mu_j
    }
    #[getter]
    fn sigma_j(&self) -> f64 {
        self.inner.sigma_j
    }

    /// Compute jump compensation term k = E[J - 1].
    fn jump_compensation(&self) -> f64 {
        self.inner.jump_compensation()
    }

    /// Compensated drift rate for risk-neutral measure (r - q - lambda*k).
    fn compensated_drift(&self) -> f64 {
        self.inner.compensated_drift()
    }

    fn __repr__(&self) -> String {
        format!(
            "MertonJumpParams(r={}, q={}, sigma={}, lambda={}, mu_j={}, sigma_j={})",
            self.inner.gbm.r,
            self.inner.gbm.q,
            self.inner.gbm.sigma,
            self.inner.lambda,
            self.inner.mu_j,
            self.inner.sigma_j
        )
    }
}

// ============================================================================
// Schwartz-Smith Process
// ============================================================================

/// Parameters for the Schwartz-Smith two-factor commodity model.
///
/// SDE:
///   dX_t = -kappa_x X_t dt + sigma_x dW_X   (short-term, mean-reverting)
///   dY_t = mu_y dt + sigma_y dW_Y            (long-term trend)
///   S_t = exp(X_t + Y_t)                     (spot price)
///   Corr(dW_X, dW_Y) = rho
///
/// Args:
///     kappa_x: Mean reversion speed for short-term deviation (> 0)
///     sigma_x: Short-term volatility (> 0)
///     mu_y: Long-term drift
///     sigma_y: Long-term volatility (> 0)
///     rho: Correlation between X and Y in [-1, 1]
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "SchwartzSmithParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySchwartzSmithParams {
    pub(crate) inner:
        finstack_valuations::instruments::common::mc::process::schwartz_smith::SchwartzSmithParams,
}

#[pymethods]
impl PySchwartzSmithParams {
    #[new]
    fn new(kappa_x: f64, sigma_x: f64, mu_y: f64, sigma_y: f64, rho: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::schwartz_smith::SchwartzSmithParams;
        Self {
            inner: SchwartzSmithParams::new(kappa_x, sigma_x, mu_y, sigma_y, rho),
        }
    }

    #[getter]
    fn kappa_x(&self) -> f64 {
        self.inner.kappa_x
    }
    #[getter]
    fn sigma_x(&self) -> f64 {
        self.inner.sigma_x
    }
    #[getter]
    fn mu_y(&self) -> f64 {
        self.inner.mu_y
    }
    #[getter]
    fn sigma_y(&self) -> f64 {
        self.inner.sigma_y
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }

    fn __repr__(&self) -> String {
        format!(
            "SchwartzSmithParams(kappa_x={}, sigma_x={}, mu_y={}, sigma_y={}, rho={})",
            self.inner.kappa_x,
            self.inner.sigma_x,
            self.inner.mu_y,
            self.inner.sigma_y,
            self.inner.rho
        )
    }
}

// ============================================================================
// Brownian Motion
// ============================================================================

/// Parameters for 1D Brownian motion with drift.
///
/// SDE: dX_t = mu dt + sigma dW_t
///
/// Args:
///     mu: Constant drift
///     sigma: Constant diffusion
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "BrownianParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBrownianParams {
    pub(crate) inner:
        finstack_valuations::instruments::common::mc::process::brownian::BrownianParams,
}

#[pymethods]
impl PyBrownianParams {
    #[new]
    fn new(mu: f64, sigma: f64) -> Self {
        use finstack_valuations::instruments::common::mc::process::brownian::BrownianParams;
        Self {
            inner: BrownianParams::new(mu, sigma),
        }
    }

    #[getter]
    fn mu(&self) -> f64 {
        self.inner.mu
    }
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    fn __repr__(&self) -> String {
        format!(
            "BrownianParams(mu={}, sigma={})",
            self.inner.mu, self.inner.sigma
        )
    }
}

// ============================================================================
// Multi-dimensional OU
// ============================================================================

/// Parameters for multi-dimensional Ornstein-Uhlenbeck process.
///
/// SDE (component i): dX_i = kappa_i (theta_i - X_i) dt + sigma_i dW_i
/// with optional correlation across the driving Brownian motions.
///
/// Args:
///     kappas: Mean reversion speeds (> 0)
///     thetas: Long-run means
///     sigmas: Volatilities (>= 0)
///     correlation: Optional correlation matrix (n x n, row-major flat list)
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "MultiOuParams",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMultiOuParams {
    pub(crate) inner:
        finstack_valuations::instruments::common::mc::process::multi_ou::MultiOuParams,
}

#[pymethods]
impl PyMultiOuParams {
    #[new]
    #[pyo3(signature = (kappas, thetas, sigmas, correlation=None))]
    fn new(
        kappas: Vec<f64>,
        thetas: Vec<f64>,
        sigmas: Vec<f64>,
        correlation: Option<Vec<f64>>,
    ) -> Self {
        use finstack_valuations::instruments::common::mc::process::multi_ou::MultiOuParams;
        Self {
            inner: MultiOuParams::new(kappas, thetas, sigmas, correlation),
        }
    }

    #[getter]
    fn kappas(&self) -> Vec<f64> {
        self.inner.kappas.clone()
    }
    #[getter]
    fn thetas(&self) -> Vec<f64> {
        self.inner.thetas.clone()
    }
    #[getter]
    fn sigmas(&self) -> Vec<f64> {
        self.inner.sigmas.clone()
    }
    #[getter]
    fn correlation(&self) -> Option<Vec<f64>> {
        self.inner.correlation.clone()
    }

    /// Get the number of dimensions.
    fn dim(&self) -> usize {
        self.inner.kappas.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiOuParams(dim={}, correlated={})",
            self.inner.kappas.len(),
            self.inner.correlation.is_some()
        )
    }
}
