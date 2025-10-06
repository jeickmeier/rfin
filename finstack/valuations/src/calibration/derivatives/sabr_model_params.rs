//! SABR model parameters used for calibration derivatives.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SABRModelParams {
    pub alpha: f64,
    pub nu: f64,
    pub rho: f64,
    pub beta: f64,
}

impl SABRModelParams {
    pub fn new(alpha: f64, nu: f64, rho: f64, beta: f64) -> Self {
        Self {
            alpha,
            nu,
            rho,
            beta,
        }
    }

    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 1.0)
    }
    pub fn rates_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::new(alpha, nu, rho, 0.5)
    }
}
