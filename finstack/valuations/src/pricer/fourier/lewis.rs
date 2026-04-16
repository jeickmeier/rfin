//! Lewis (2001) single-integral Fourier pricing method.
//!
//! Provides a Gauss-Legendre quadrature-based Fourier pricer using the
//! Lewis contour at Im(z) = -1/2. This method evaluates the characteristic
//! function at complex frequencies u - i/2 where u is real, avoiding the
//! need for separate P1/P2 integrals.
//!
//! # Current Limitations
//!
//! This implementation works best for near-ATM strikes. For deep ITM/OTM
//! the oscillatory nature of the integrand may require higher panel counts.
//! For production use with arbitrary strikes, prefer the COS method which
//! handles all moneyness levels with automatic truncation.
//!
//! # References
//!
//! - Lewis, A. L. (2001). "A Simple Option Formula for General
//!   Jump-Diffusion and Other Exponential Levy Processes."

use finstack_core::math::characteristic_function::CharacteristicFunction;
use num_complex::Complex64;
use std::f64::consts::PI;

/// Lewis (2001) single-integral pricing configuration.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LewisConfig {
    /// Upper integration limit (default: 500.0).
    pub u_max: f64,
    /// Number of Gauss-Legendre panels (default: 100).
    pub panels: usize,
}

impl Default for LewisConfig {
    fn default() -> Self {
        Self {
            u_max: 500.0,
            panels: 100,
        }
    }
}

/// Lewis (2001) single-integral Fourier pricer.
pub struct LewisPricer<'a> {
    cf: &'a dyn CharacteristicFunction,
    config: LewisConfig,
}

impl<'a> LewisPricer<'a> {
    /// Create a new Lewis pricer.
    pub fn new(cf: &'a dyn CharacteristicFunction, config: LewisConfig) -> Self {
        Self { cf, config }
    }

    /// Price a European call option.
    pub fn price_call(
        &self,
        spot: f64,
        strike: f64,
        r: f64,
        q: f64,
        t: f64,
    ) -> crate::pricer::PricingResult<f64> {
        let x = (spot / strike).ln();
        let df = (-r * t).exp();
        let integral = self.integrate(x, t)?;
        let call = (spot * (-q * t).exp() - strike * df / PI * integral).max(0.0);
        Ok(call)
    }

    /// Price a European put via put-call parity.
    pub fn price_put(
        &self,
        spot: f64,
        strike: f64,
        r: f64,
        q: f64,
        t: f64,
    ) -> crate::pricer::PricingResult<f64> {
        let call = self.price_call(spot, strike, r, q, t)?;
        let put = (call - spot * (-q * t).exp() + strike * (-r * t).exp()).max(0.0);
        Ok(put)
    }

    fn integrate(&self, x: f64, t: f64) -> crate::pricer::PricingResult<f64> {
        let panels = self.config.panels;
        let u_max = self.config.u_max;
        let h = u_max / panels as f64;
        let (nodes, weights) = gl_nodes_weights_16();
        let mut total = 0.0;

        for panel_idx in 0..panels {
            let lo = panel_idx as f64 * h;
            let half = 0.5 * h;
            let mid = lo + half;
            let mut panel_sum = 0.0;

            for (&node, &weight) in nodes.iter().zip(weights.iter()) {
                let v = mid + half * node;
                if v < 1e-12 {
                    continue;
                }
                let w = Complex64::new(v, -0.5);
                let phi = self.cf.cf(w, t);
                if !phi.is_finite() {
                    continue;
                }
                let exp_ivx = Complex64::new(0.0, v * x).exp();
                let val = (exp_ivx * phi).re / (v * v + 0.25);
                if val.is_finite() {
                    panel_sum += weight * val;
                }
            }
            total += half * panel_sum;
        }

        if !total.is_finite() {
            return Err(crate::pricer::PricingError::ModelFailure {
                message: "Lewis integral diverged".to_string(),
                context: crate::pricer::PricingErrorContext::default(),
            });
        }
        Ok(total)
    }
}

fn gl_nodes_weights_16() -> (&'static [f64], &'static [f64]) {
    static NODES: [f64; 16] = [
        -0.989_400_934_991_649_9, -0.944_575_023_073_232_6, -0.865_631_202_387_831_8,
        -0.755_404_408_355_003, -0.617_876_244_402_643_8, -0.458_016_777_657_227_37,
        -0.281_603_550_779_258_9, -0.095_012_509_837_637_44, 0.095_012_509_837_637_44,
        0.281_603_550_779_258_9, 0.458_016_777_657_227_37, 0.617_876_244_402_643_8,
        0.755_404_408_355_003, 0.865_631_202_387_831_8, 0.944_575_023_073_232_6,
        0.989_400_934_991_649_9,
    ];
    static WEIGHTS: [f64; 16] = [
        0.027_152_459_411_754_095, 0.062_253_523_938_647_894, 0.095_158_511_682_492_78,
        0.124_628_971_255_533_88, 0.149_595_988_816_576_73, 0.169_156_519_395_002_54,
        0.182_603_415_044_923_58, 0.189_450_610_455_068_5, 0.189_450_610_455_068_5,
        0.182_603_415_044_923_58, 0.169_156_519_395_002_54, 0.149_595_988_816_576_73,
        0.124_628_971_255_533_88, 0.095_158_511_682_492_78, 0.062_253_523_938_647_894,
        0.027_152_459_411_754_095,
    ];
    (&NODES, &WEIGHTS)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::math::characteristic_function::BlackScholesCf;

    fn bs_call_price(spot: f64, strike: f64, r: f64, q: f64, t: f64, sigma: f64) -> f64 {
        use finstack_core::math::special_functions::norm_cdf;
        let fwd = spot * ((r - q) * t).exp();
        let sqrt_t = t.sqrt();
        let d1 = ((fwd / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;
        (-r * t).exp() * (fwd * norm_cdf(d1) - strike * norm_cdf(d2))
    }

    #[test]
    fn lewis_matches_bs_call_atm() {
        let sigma = 0.2;
        let cf = BlackScholesCf { r: 0.05, q: 0.0, sigma };
        let pricer = LewisPricer::new(&cf, LewisConfig::default());
        let lewis = pricer.price_call(100.0, 100.0, 0.05, 0.0, 1.0).unwrap();
        let bs = bs_call_price(100.0, 100.0, 0.05, 0.0, 1.0, sigma);
        assert!(
            (lewis - bs).abs() < 5e-4,
            "Lewis={lewis:.8}, BS={bs:.8}, diff={}", (lewis - bs).abs()
        );
    }

    #[test]
    fn lewis_put_call_parity() {
        let sigma = 0.2;
        let cf = BlackScholesCf { r: 0.05, q: 0.02, sigma };
        let pricer = LewisPricer::new(&cf, LewisConfig::default());
        let (s, k, r, q, t) = (100.0, 105.0, 0.05, 0.02, 1.0);
        let call = pricer.price_call(s, k, r, q, t).unwrap();
        let put = pricer.price_put(s, k, r, q, t).unwrap();
        let parity = call - put - (s * (-q * t).exp() - k * (-r * t).exp());
        assert!(parity.abs() < 1e-6, "Put-call parity residual: {parity:.10}");
    }

    #[test]
    fn lewis_prices_nonnegative_and_bounded() {
        let sigma = 0.2;
        let cf = BlackScholesCf { r: 0.05, q: 0.0, sigma };
        let pricer = LewisPricer::new(&cf, LewisConfig::default());
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let call = pricer.price_call(100.0, strike, 0.05, 0.0, 1.0).unwrap();
            assert!(call >= 0.0, "K={strike}: call should be non-negative, got {call}");
            assert!(call < 100.0, "K={strike}: call should be < spot, got {call}");
        }
    }

    #[test]
    fn lewis_call_monotone_in_strike() {
        let cf = BlackScholesCf { r: 0.05, q: 0.0, sigma: 0.2 };
        let pricer = LewisPricer::new(&cf, LewisConfig::default());
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        let prices: Vec<f64> = strikes
            .iter()
            .map(|&k| pricer.price_call(100.0, k, 0.05, 0.0, 1.0).unwrap())
            .collect();
        for w in prices.windows(2) {
            assert!(w[0] >= w[1], "Call price should decrease with strike: {} >= {}", w[0], w[1]);
        }
    }

    #[test]
    fn lewis_vg_prices_reasonable() {
        use finstack_core::math::characteristic_function::VarianceGammaCf;
        let vg = VarianceGammaCf { r: 0.05, q: 0.0, sigma: 0.12, nu: 0.2, theta: -0.14 };
        let pricer = LewisPricer::new(&vg, LewisConfig::default());
        let call = pricer.price_call(100.0, 100.0, 0.05, 0.0, 1.0).unwrap();
        assert!(call > 0.0 && call < 100.0, "VG call: {call}");
    }

    #[test]
    fn lewis_merton_prices_reasonable() {
        use finstack_core::math::characteristic_function::MertonJumpCf;
        let merton = MertonJumpCf { r: 0.05, q: 0.0, sigma: 0.2, lambda: 1.0, mu_j: -0.05, sigma_j: 0.1 };
        let pricer = LewisPricer::new(&merton, LewisConfig::default());
        let call = pricer.price_call(100.0, 100.0, 0.05, 0.0, 1.0).unwrap();
        assert!(call > 0.0 && call < 100.0, "Merton call: {call}");
    }
}
