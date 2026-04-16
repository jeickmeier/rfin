//! Variance Gamma model characteristic function.

use super::{CharacteristicFunction, Cumulants};
use num_complex::Complex64;

/// Variance Gamma model characteristic function.
///
/// The VG process is a time-changed Brownian motion where the subordinator
/// is a Gamma process. Parameters: sigma (volatility), nu (variance rate
/// of Gamma subordinator), theta (drift of subordinated BM).
///
/// phi(u, t) = exp(iu * omega * t) * (1 - iu * theta * nu + 0.5 * sigma^2 * nu * u^2)^{-t/nu}
///
/// where omega = (1/nu) * ln(1 - theta * nu - 0.5 * sigma^2 * nu) is the
/// convexity correction ensuring E[S_T] = S_0 * exp((r-q)*T).
///
/// # References
///
/// - Madan, D. B., Carr, P. P. & Chang, E. C. (1998). "The Variance Gamma
///   Process and Option Pricing." *European Finance Review*, 2, 79-105.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct VarianceGammaCf {
    /// Risk-free rate.
    pub r: f64,
    /// Dividend yield.
    pub q: f64,
    /// Volatility of the subordinated Brownian motion.
    pub sigma: f64,
    /// Variance rate of the Gamma subordinator (nu > 0).
    pub nu: f64,
    /// Drift of the subordinated Brownian motion.
    pub theta: f64,
}

impl CharacteristicFunction for VarianceGammaCf {
    fn cf(&self, u: Complex64, t: f64) -> Complex64 {
        let iu = Complex64::i() * u;
        let omega = (1.0 / self.nu)
            * (1.0 - self.theta * self.nu - 0.5 * self.sigma * self.sigma * self.nu).ln();
        let base = Complex64::new(1.0, 0.0) - iu * self.theta * self.nu
            + 0.5 * self.sigma * self.sigma * self.nu * u * u;
        (iu * (self.r - self.q + omega) * t + (-t / self.nu) * base.ln()).exp()
    }

    fn cumulants(&self, t: f64) -> Cumulants {
        let s2 = self.sigma * self.sigma;
        let omega = (1.0 / self.nu) * (1.0 - self.theta * self.nu - 0.5 * s2 * self.nu).ln();
        Cumulants {
            c1: (self.r - self.q + omega) * t,
            c2: (s2 + self.nu * self.theta * self.theta) * t,
            c3: (2.0 * self.theta.powi(3) * self.nu * self.nu + 3.0 * s2 * self.theta * self.nu)
                * t,
            c4: (3.0 * s2 * s2 * self.nu
                + 12.0 * s2 * self.theta * self.theta * self.nu * self.nu
                + 6.0 * self.theta.powi(4) * self.nu.powi(3))
                * t,
        }
    }
}
