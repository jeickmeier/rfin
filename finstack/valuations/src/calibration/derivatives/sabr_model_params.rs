//! SABR model parameters used for calibration derivatives.

use finstack_core::F;

#[derive(Clone, Debug)]
pub struct SABRModelParams {
    pub alpha: F,
    pub nu: F,
    pub rho: F,
    pub beta: F,
}

impl SABRModelParams {
    pub fn new(alpha: F, nu: F, rho: F, beta: F) -> Self {
        Self { alpha, nu, rho, beta }
    }

    pub fn equity_standard(alpha: F, nu: F, rho: F) -> Self { Self::new(alpha, nu, rho, 1.0) }
    pub fn rates_standard(alpha: F, nu: F, rho: F) -> Self { Self::new(alpha, nu, rho, 0.5) }
}
