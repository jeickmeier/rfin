//! Geometric Brownian Motion (GBM) process for equity and FX simulation.
//!
//! Implements the standard stochastic differential equation for asset prices
//! under the risk-neutral measure. This is the fundamental process underlying
//! the Black-Scholes-Merton framework.
//!
//! # Stochastic Differential Equation
//!
//! Under the risk-neutral measure ℚ:
//!
//! ```text
//! dS_t = (r - q) S_t dt + σ S_t dW_t
//! ```
//!
//! where:
//! - **S_t**: Asset price at time t
//! - **r**: Risk-free interest rate (continuously compounded)
//! - **q**: Dividend yield (equity) or foreign rate (FX)
//! - **σ**: Volatility (constant in standard GBM)
//! - **W_t**: Standard Brownian motion under ℚ
//!
//! # Exact Simulation
//!
//! GBM admits exact simulation without discretization bias. The solution is:
//!
//! ```text
//! S_{t+Δt} = S_t · exp[(r - q - σ²/2)Δt + σ√Δt · Z]
//! ```
//!
//! where Z ~ N(0,1). This is **unbiased** regardless of step size Δt.
//!
//! # Applications
//!
//! - **Equity options**: Standard model for single-stock options
//! - **FX options**: Garman-Kohlhagen model (GBM with two rates)
//! - **Index options**: Assuming constant dividend yield
//! - **Commodity options**: With convenience yield q
//!
//! # Limitations
//!
//! GBM assumes:
//! - Constant volatility (no smile/skew)
//! - Lognormal returns (no jumps)
//! - Continuous trading
//! - No transaction costs
//!
//! For more realistic models, see:
//! - [`HestonProcess`](super::heston::HestonProcess) for stochastic volatility
//! - [`BatesProcess`](super::bates::BatesProcess) for jumps + stochastic vol
//! - Local volatility models for calibrated smiles
//!
//! # References
//!
//! - Black, F., & Scholes, M. (1973). "The Pricing of Options and Corporate
//!   Liabilities." *Journal of Political Economy*, 81(3), 637-654.
//!
//! - Merton, R. C. (1973). "Theory of Rational Option Pricing."
//!   *Bell Journal of Economics and Management Science*, 4(1), 141-183.
//!
//! - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*.
//!   Springer. Section 3.1: Generating Sample Paths. pp. 97-103.
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::common::mc::process::gbm::{GbmProcess, GbmParams};
//!
//! let params = GbmParams::new(
//!     0.05,  // r = 5% risk-free rate
//!     0.02,  // q = 2% dividend yield
//!     0.20,  // σ = 20% volatility
//! );
//!
//! let gbm = GbmProcess::new(params);
//!
//! // Use in MC engine for path generation
//! // let paths = engine.generate_paths(&gbm, spot, time_grid)?;
//! ```

use super::super::paths::ProcessParams;
use super::super::traits::StochasticProcess;
use super::metadata::ProcessMetadata;

/// Geometric Brownian Motion parameters.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GbmParams {
    /// Risk-free rate (annual)
    pub r: f64,
    /// Dividend/foreign rate (annual)
    pub q: f64,
    /// Volatility (annual)
    pub sigma: f64,
}

impl GbmParams {
    /// Create new GBM parameters.
    pub fn new(r: f64, q: f64, sigma: f64) -> Self {
        Self { r, q, sigma }
    }

    /// Create from market-implied parameters.
    ///
    /// # Arguments
    ///
    /// * `r` - Risk-free rate
    /// * `q` - Dividend yield
    /// * `sigma` - Implied volatility
    pub fn from_market(r: f64, q: f64, sigma: f64) -> Self {
        Self::new(r, q, sigma)
    }
}

/// Single-factor GBM process (1D).
///
/// State: S (spot price)
/// Factor: 1 Brownian motion
#[derive(Clone, Debug)]
pub struct GbmProcess {
    params: GbmParams,
}

impl GbmProcess {
    /// Create a new GBM process.
    pub fn new(params: GbmParams) -> Self {
        Self { params }
    }

    /// Create with explicit parameters.
    pub fn with_params(r: f64, q: f64, sigma: f64) -> Self {
        Self::new(GbmParams::new(r, q, sigma))
    }

    /// Get parameters.
    pub fn params(&self) -> &GbmParams {
        &self.params
    }

    /// Risk-neutral drift rate.
    pub fn drift_rate(&self) -> f64 {
        self.params.r - self.params.q
    }

    /// Volatility.
    pub fn volatility(&self) -> f64 {
        self.params.sigma
    }
}

impl StochasticProcess for GbmProcess {
    fn dim(&self) -> usize {
        1
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        // μ(S) = (r - q) S
        out[0] = self.drift_rate() * x[0];
    }

    fn diffusion(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        // σ(S) = σ S
        out[0] = self.params.sigma * x[0];
    }

    fn is_diagonal(&self) -> bool {
        true
    }
}

impl ProcessMetadata for GbmProcess {
    fn metadata(&self) -> ProcessParams {
        let mut params = ProcessParams::new("GBM");
        params.add_param("r", self.params.r);
        params.add_param("q", self.params.q);
        params.add_param("sigma", self.params.sigma);
        params.with_factors(vec!["spot".to_string()])
    }
}

/// Multi-factor GBM (for correlated assets).
///
/// State: [S_1, S_2, ..., S_n]
/// Factors: n correlated Brownian motions
///
/// Each asset i follows:
/// ```text
/// dS_i = (r - q_i) S_i dt + σ_i S_i dW_i
/// ```
///
/// where W_i are correlated via a correlation matrix ρ.
#[derive(Clone, Debug)]
pub struct MultiGbmProcess {
    /// Parameters for each asset
    params: Vec<GbmParams>,
    /// Correlation matrix (n x n, row-major)
    correlation: Option<Vec<f64>>,
}

impl MultiGbmProcess {
    /// Create a multi-factor GBM process.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for each asset
    /// * `correlation` - Optional correlation matrix (if None, assumes independence)
    pub fn new(params: Vec<GbmParams>, correlation: Option<Vec<f64>>) -> Self {
        if let Some(ref corr) = correlation {
            let n = params.len();
            assert_eq!(corr.len(), n * n, "Correlation matrix must be n x n");
        }
        Self {
            params,
            correlation,
        }
    }

    /// Number of assets.
    pub fn num_assets(&self) -> usize {
        self.params.len()
    }

    /// Get asset parameters.
    pub fn asset_params(&self, i: usize) -> &GbmParams {
        &self.params[i]
    }

    /// Get correlation matrix (if set).
    pub fn correlation(&self) -> Option<&[f64]> {
        self.correlation.as_deref()
    }
}

impl StochasticProcess for MultiGbmProcess {
    fn dim(&self) -> usize {
        self.params.len()
    }

    fn num_factors(&self) -> usize {
        self.params.len()
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        for (i, params) in self.params.iter().enumerate() {
            out[i] = (params.r - params.q) * x[i];
        }
    }

    fn diffusion(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        for (i, params) in self.params.iter().enumerate() {
            out[i] = params.sigma * x[i];
        }
    }

    fn is_diagonal(&self) -> bool {
        self.correlation.is_none()
    }
}

impl ProcessMetadata for MultiGbmProcess {
    fn metadata(&self) -> ProcessParams {
        let mut params = ProcessParams::new("MultiGBM");

        // Add per-asset parameters
        for (i, asset_params) in self.params.iter().enumerate() {
            params.add_param(format!("r_{}", i), asset_params.r);
            params.add_param(format!("q_{}", i), asset_params.q);
            params.add_param(format!("sigma_{}", i), asset_params.sigma);
        }

        // Add correlation matrix if present
        let params = if let Some(ref corr) = self.correlation {
            params.with_correlation(corr.clone())
        } else {
            params
        };

        // Add factor names
        let factor_names: Vec<String> = (0..self.params.len())
            .map(|i| format!("spot_{}", i))
            .collect();
        params.with_factors(factor_names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbm_creation() {
        let params = GbmParams::new(0.05, 0.02, 0.2);
        let gbm = GbmProcess::new(params);

        assert_eq!(gbm.dim(), 1);
        assert!((gbm.drift_rate() - 0.03).abs() < 1e-10); // r - q
        assert!((gbm.volatility() - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_gbm_drift_diffusion() {
        let gbm = GbmProcess::with_params(0.05, 0.02, 0.2);
        let x = [100.0];
        let mut drift = [0.0];
        let mut diffusion = [0.0];

        gbm.drift(0.0, &x, &mut drift);
        gbm.diffusion(0.0, &x, &mut diffusion);

        assert!((drift[0] - 3.0).abs() < 1e-10); // 0.03 * 100
        assert!((diffusion[0] - 20.0).abs() < 1e-10); // 0.2 * 100
    }

    #[test]
    fn test_multi_gbm() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let multi_gbm = MultiGbmProcess::new(params, None);

        assert_eq!(multi_gbm.dim(), 2);
        assert_eq!(multi_gbm.num_assets(), 2);
        assert!(multi_gbm.is_diagonal());
    }

    #[test]
    fn test_multi_gbm_with_correlation() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let multi_gbm = MultiGbmProcess::new(params, Some(corr));

        assert!(!multi_gbm.is_diagonal());
        assert!(multi_gbm.correlation().is_some());
    }
}
