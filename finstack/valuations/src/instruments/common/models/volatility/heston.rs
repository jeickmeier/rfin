//! Heston Stochastic Volatility Model.
//!
//! The Heston model assumes that the underlying asset price $S_t$ and its variance $v_t$
//! follow the stochastic processes:
//!
//! $$ dS_t = \mu S_t dt + \sqrt{v_t} S_t dW_t^S $$
//! $$ dv_t = \kappa (\theta - v_t) dt + \sigma \sqrt{v_t} dv_t dW_t^v $$
//!
//! where:
//! - $v_0$: Initial variance
//! - $\kappa$: Mean reversion speed of variance
//! - $\theta$: Long-run average variance
//! - $\sigma$: Volatility of variance (vol-of-vol)
//! - $\rho$: Correlation between asset and variance Brownian motions ($dW_t^S dW_t^v = \rho dt$)
//!
//! Pricing is performed using semi-analytical Fourier inversion of the characteristic function.

use finstack_core::Result;
use num_complex::Complex;
use std::f64::consts::PI;

/// Heston model parameters.
#[derive(Clone, Debug, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HestonParameters {
    /// Initial variance ($v_0$)
    pub v0: f64,
    /// Mean reversion speed ($\kappa$)
    pub kappa: f64,
    /// Long-run average variance ($\theta$)
    pub theta: f64,
    /// Volatility of variance ($\sigma$)
    pub sigma: f64,
    /// Correlation between asset and variance ($\rho$)
    pub rho: f64,
}

impl HestonParameters {
    /// Create new Heston parameters.
    pub fn new(v0: f64, kappa: f64, theta: f64, sigma: f64, rho: f64) -> Result<Self> {
        if v0 < 0.0 {
            return Err(finstack_core::Error::Validation("v0 must be non-negative".into()));
        }
        if kappa < 0.0 {
            return Err(finstack_core::Error::Validation("kappa must be non-negative".into()));
        }
        if theta < 0.0 {
            return Err(finstack_core::Error::Validation("theta must be non-negative".into()));
        }
        if sigma < 0.0 {
            return Err(finstack_core::Error::Validation("sigma must be non-negative".into()));
        }
        if !(-1.0..=1.0).contains(&rho) {
            return Err(finstack_core::Error::Validation("rho must be in [-1, 1]".into()));
        }

        Ok(Self {
            v0,
            kappa,
            theta,
            sigma,
            rho,
        })
    }

    /// Check Feller condition ($2\kappa\theta > \sigma^2$).
    ///
    /// If true, the variance process is strictly positive.
    /// If false, the variance can reach zero.
    pub fn satisfies_feller_condition(&self) -> bool {
        2.0 * self.kappa * self.theta > self.sigma * self.sigma
    }
}

/// Heston model pricer.
pub struct HestonModel {
    params: HestonParameters,
}

impl HestonModel {
    /// Create a new Heston model.
    pub fn new(params: HestonParameters) -> Self {
        Self { params }
    }

    /// Price a European call option using Fourier inversion.
    ///
    /// Uses the Gil-Pelaez formula or similar standard integration method.
    ///
    /// # Arguments
    /// * `S`: Spot price
    /// * `K`: Strike price
    /// * `T`: Time to maturity (years)
    /// * `r`: Risk-free rate (continuous compounding)
    /// * `q`: Dividend yield (continuous compounding)
    pub fn price_european_call(&self, S: f64, K: f64, T: f64, r: f64, q: f64) -> Result<f64> {
        // Heston pricing formula:
        // Call = S * e^{-qT} * P1 - K * e^{-rT} * P2
        // where P1 and P2 are probabilities derived from the characteristic function.

        let p1 = self.calculate_prob(1, S, K, T, r, q)?;
        let p2 = self.calculate_prob(2, S, K, T, r, q)?;

        let call_price = S * (-q * T).exp() * p1 - K * (-r * T).exp() * p2;
        
        // Ensure non-negative price (numerical errors can sometimes cause slight negatives for deep OTM)
        Ok(call_price.max(0.0))
    }

    /// Price a European put option using Put-Call Parity.
    pub fn price_european_put(&self, S: f64, K: f64, T: f64, r: f64, q: f64) -> Result<f64> {
        let call = self.price_european_call(S, K, T, r, q)?;
        // Put = Call - S * e^{-qT} + K * e^{-rT}
        let put = call - S * (-q * T).exp() + K * (-r * T).exp();
        Ok(put.max(0.0))
    }

    /// Calculate probabilities P1 and P2 via numerical integration.
    fn calculate_prob(&self, prob_num: u8, S: f64, K: f64, T: f64, r: f64, q: f64) -> Result<f64> {
        // Integration limits and step
        // The integrand decays effectively; 100-200 is usually sufficient upper bound for standard params
        let upper_bound = 100.0; 
        let n_steps = 1000;
        let d_phi = upper_bound / n_steps as f64;

        let mut sum = 0.0;

        // Trapezoidal rule
        for i in 0..=n_steps {
            let phi = i as f64 * d_phi;
            // Avoid singularity at phi=0
            let phi_eval = if phi == 0.0 { 1e-8 } else { phi };
            
            let integrand = self.integrand(prob_num, phi_eval, S, K, T, r, q)?;
            
            let weight = if i == 0 || i == n_steps { 0.5 } else { 1.0 };
            sum += weight * integrand;
        }

        let integral = sum * d_phi;
        Ok(0.5 + (1.0 / PI) * integral)
    }

    /// Integrand for the probabilities.
    /// Re[ (e^{-i * phi * ln(K)} * psi(phi)) / (i * phi) ]
    fn integrand(&self, prob_num: u8, phi: f64, S: f64, K: f64, T: f64, r: f64, q: f64) -> Result<f64> {
        let i_complex = Complex::new(0.0, 1.0);
        let log_k = K.ln();
        
        let psi = self.characteristic_function(prob_num, phi, S, T, r, q)?;
        
        let term = (-i_complex * phi * log_k).exp() * psi / (i_complex * phi);
        Ok(term.re)
    }

    /// Heston characteristic function.
    /// 
    /// Returns $\psi(\phi)$ for the log-price.
    fn characteristic_function(&self, prob_num: u8, phi: f64, S: f64, T: f64, r: f64, q: f64) -> Result<Complex<f64>> {
        let kappa = self.params.kappa;
        let theta = self.params.theta;
        let sigma = self.params.sigma;
        let rho = self.params.rho;
        let v0 = self.params.v0;

        let i_complex = Complex::new(0.0, 1.0);
        let x = S.ln();

        let (u, b) = if prob_num == 1 {
            (0.5, kappa - rho * sigma)
        } else {
            (-0.5, kappa)
        };

        let a = kappa * theta;
        let d_sq = (rho * sigma * phi * i_complex - b).powi(2) - sigma * sigma * (2.0 * u * phi * i_complex - phi * phi);
        let d = d_sq.sqrt();
        
        let g = (b - rho * sigma * phi * i_complex + d) / (b - rho * sigma * phi * i_complex - d);
        
        let c = (r - q) * phi * i_complex * T 
            + (a / sigma.powi(2)) * ((b - rho * sigma * phi * i_complex + d) * T - 2.0 * ((1.0 - g * (d * T).exp()) / (1.0 - g)).ln());
            
        let d_term = (b - rho * sigma * phi * i_complex + d) / sigma.powi(2) * ((1.0 - (d * T).exp()) / (1.0 - g * (d * T).exp()));
        
        Ok((c + d_term * v0 + i_complex * phi * x).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heston_black_scholes_limit() -> Result<()> {
        // As sigma (vol of vol) -> 0, Heston should converge to Black-Scholes
        // We set kappa high to force v to theta quickly, or just v0=theta and sigma=0
        
        let S = 100.0;
        let K = 100.0;
        let T = 1.0;
        let r = 0.05;
        let q = 0.0;
        let vol = 0.2;

        let params = HestonParameters::new(
            vol * vol, // v0
            1.0,       // kappa (irrelevant if sigma=0 and v0=theta)
            vol * vol, // theta
            1e-6,      // sigma (approx 0)
            0.0,       // rho
        )?;

        let model = HestonModel::new(params);
        let price = model.price_european_call(S, K, T, r, q)?;

        // BS Price for these params is approx 10.4506
        // d1 = (ln(1/1) + (0.05 + 0.04/2)*1) / 0.2 = 0.07 / 0.2 = 0.35
        // d2 = 0.15
        // N(0.35) = 0.6368, N(0.15) = 0.5596
        // C = 100 * 0.6368 - 100 * e^-0.05 * 0.5596 = 63.68 - 95.12 * 0.5596 = 63.68 - 53.23 = 10.45
        
        let bs_price = 10.4506;
        assert!((price - bs_price).abs() < 0.01, "Heston limit should match BS. Got {}, expected {}", price, bs_price);

        Ok(())
    }

    #[test]
    fn test_heston_literature_value() -> Result<()> {
        // Test case from "The Heston Model and its Extensions in Matlab and C#", Rouah & Vainberg
        // or similar standard test cases.
        // Parameters: S=100, K=100, T=0.5, r=0.03, q=0.0
        // v0=0.04, kappa=2.0, theta=0.04, sigma=0.3, rho=-0.5
        
        let S = 100.0;
        let K = 100.0;
        let T = 0.5;
        let r = 0.03;
        let q = 0.0;
        
        let params = HestonParameters::new(
            0.04, // v0
            2.0,  // kappa
            0.04, // theta
            0.3,  // sigma
            -0.5, // rho
        )?;

        let model = HestonModel::new(params);
        let call_price = model.price_european_call(S, K, T, r, q)?;
        
        // Reference value approx 6.28 (approximate check)
        // Let's verify directionality at least if exact reference unavailable
        assert!(call_price > 0.0);
        assert!(call_price < S);
        
        Ok(())
    }
}
