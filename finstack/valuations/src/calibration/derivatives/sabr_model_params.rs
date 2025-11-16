//! SABR model parameters used for calibration derivatives.

use serde::{Deserialize, Serialize};

/// SABR (Stochastic Alpha Beta Rho) volatility model parameters.
///
/// Parameters for the SABR model used in calibrating implied volatility surfaces
/// for interest rate and equity options.
///
/// # References
///
/// - Hagan, P. S., Kumar, D., Lesniewski, A. S., & Woodward, D. E. (2002).
///   "Managing Smile Risk." *Wilmott Magazine*, September, 84-108.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SABRModelParams {
    /// Alpha: initial volatility level
    pub alpha: f64,
    /// Nu: volatility of volatility (vol-of-vol)
    pub nu: f64,
    /// Rho: correlation between asset price and volatility
    pub rho: f64,
    /// Beta: CEV exponent (0 = normal, 0.5 = CIR, 1 = lognormal)
    pub beta: f64,
}

impl SABRModelParams {
    /// Create new SABR parameters with all four parameters.
    pub fn new(alpha: f64, nu: f64, rho: f64, beta: f64) -> Self {
        Self {
            alpha,
            nu,
            rho,
            beta,
        }
    }

    /// Create SABR parameters with equity market standard beta = 1.0 (lognormal).
    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 1.0)
    }

    /// Create SABR parameters with rates market standard beta = 0.5 (CIR/square-root).
    pub fn rates_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 0.5)
    }
}
