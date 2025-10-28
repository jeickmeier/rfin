//! Black-Scholes analytical Greeks for validation.
//!
//! Provides closed-form formulas for option sensitivities (Greeks)
//! used to validate Monte Carlo Greek computations.

use finstack_core::math::special_functions::{norm_cdf, norm_pdf};

/// Compute d1 parameter for Black-Scholes formula.
fn bs_d1(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 || vol <= 0.0 {
        return 0.0;
    }
    ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt())
}

/// Compute d2 parameter for Black-Scholes formula.
fn bs_d2(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    bs_d1(spot, strike, time, rate, div_yield, vol) - vol * time.sqrt()
}

/// Black-Scholes call delta.
///
/// Δ_call = exp(-qT) * N(d1)
pub fn bs_call_delta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return if spot > strike { 1.0 } else { 0.0 };
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    (-div_yield * time).exp() * norm_cdf(d1)
}

/// Black-Scholes put delta.
///
/// Δ_put = -exp(-qT) * N(-d1)
pub fn bs_put_delta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return if spot < strike { -1.0 } else { 0.0 };
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    -(-div_yield * time).exp() * norm_cdf(-d1)
}

/// Black-Scholes gamma (same for call and put).
///
/// Γ = exp(-qT) * φ(d1) / (S * σ * √T)
pub fn bs_gamma(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 || spot <= 0.0 || vol <= 0.0 {
        return 0.0;
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    (-div_yield * time).exp() * norm_pdf(d1) / (spot * vol * time.sqrt())
}

/// Black-Scholes vega (same for call and put).
///
/// ν = S * exp(-qT) * √T * φ(d1)
pub fn bs_vega(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    spot * (-div_yield * time).exp() * time.sqrt() * norm_pdf(d1)
}

/// Black-Scholes call theta.
///
/// Θ_call = -S * φ(d1) * σ / (2√T) * exp(-qT) - r*K*exp(-rT)*N(d2) + q*S*exp(-qT)*N(d1)
pub fn bs_call_theta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    
    let term1 = -spot * norm_pdf(d1) * vol * (-div_yield * time).exp() / (2.0 * time.sqrt());
    let term2 = -rate * strike * (-rate * time).exp() * norm_cdf(d2);
    let term3 = div_yield * spot * (-div_yield * time).exp() * norm_cdf(d1);
    
    term1 + term2 + term3
}

/// Black-Scholes put theta.
///
/// Θ_put = -S * φ(d1) * σ / (2√T) * exp(-qT) + r*K*exp(-rT)*N(-d2) - q*S*exp(-qT)*N(-d1)
pub fn bs_put_theta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    
    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    
    let term1 = -spot * norm_pdf(d1) * vol * (-div_yield * time).exp() / (2.0 * time.sqrt());
    let term2 = rate * strike * (-rate * time).exp() * norm_cdf(-d2);
    let term3 = -div_yield * spot * (-div_yield * time).exp() * norm_cdf(-d1);
    
    term1 + term2 + term3
}

/// Black-Scholes call rho.
///
/// ρ_call = K * T * exp(-rT) * N(d2)
pub fn bs_call_rho(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    strike * time * (-rate * time).exp() * norm_cdf(d2)
}

/// Black-Scholes put rho.
///
/// ρ_put = -K * T * exp(-rT) * N(-d2)
pub fn bs_put_rho(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    -strike * time * (-rate * time).exp() * norm_cdf(-d2)
}

/// Convenience wrapper for all call Greeks.
pub struct CallGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

/// Convenience wrapper for all put Greeks.
pub struct PutGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

/// Compute all call Greeks at once.
pub fn bs_call_greeks(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> CallGreeks {
    CallGreeks {
        delta: bs_call_delta(spot, strike, time, rate, div_yield, vol),
        gamma: bs_gamma(spot, strike, time, rate, div_yield, vol),
        vega: bs_vega(spot, strike, time, rate, div_yield, vol),
        theta: bs_call_theta(spot, strike, time, rate, div_yield, vol),
        rho: bs_call_rho(spot, strike, time, rate, div_yield, vol),
    }
}

/// Compute all put Greeks at once.
pub fn bs_put_greeks(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> PutGreeks {
    PutGreeks {
        delta: bs_put_delta(spot, strike, time, rate, div_yield, vol),
        gamma: bs_gamma(spot, strike, time, rate, div_yield, vol),
        vega: bs_vega(spot, strike, time, rate, div_yield, vol),
        theta: bs_put_theta(spot, strike, time, rate, div_yield, vol),
        rho: bs_put_rho(spot, strike, time, rate, div_yield, vol),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_delta_atm() {
        let delta = bs_call_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // ATM delta should be around 0.5-0.6
        assert!(delta > 0.4 && delta < 0.7);
    }

    #[test]
    fn test_put_delta_atm() {
        let delta = bs_put_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // ATM put delta should be negative, around -0.4 to -0.5
        assert!(delta < 0.0 && delta > -0.7);
    }

    #[test]
    fn test_gamma_positive() {
        let gamma = bs_gamma(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // Gamma should always be positive
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_vega_positive() {
        let vega = bs_vega(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // Vega should always be positive
        assert!(vega > 0.0);
    }

    #[test]
    fn test_put_call_delta_parity() {
        let call_delta = bs_call_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        let put_delta = bs_put_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        
        // Delta parity: Δ_call - Δ_put = exp(-qT)
        let lhs = call_delta - put_delta;
        let rhs = (-0.02_f64 * 1.0).exp();
        
        assert!((lhs - rhs).abs() < 0.001, "Delta parity failed: {} vs {}", lhs, rhs);
    }
}

