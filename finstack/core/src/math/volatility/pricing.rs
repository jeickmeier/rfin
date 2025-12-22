use crate::math::{norm_cdf, norm_pdf};

/// Bachelier (normal) call price with unit annuity.
///
/// Computes the price of a call option under the Bachelier model assuming a unit annuity (PV01=1).
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility
/// * `t` - Time to expiry
pub fn bachelier_price(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    if sigma_n <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma_n * t.sqrt();
    if st <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let d = (forward - strike) / st;
    (forward - strike) * norm_cdf(d) + st * norm_pdf(d)
}

/// Black (lognormal) call price with unit annuity.
///
/// Computes the price of a call option under the Black model assuming a unit annuity (PV01=1).
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry
pub fn black_price(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    if sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    let d2 = d1 - st;
    forward * norm_cdf(d1) - strike * norm_cdf(d2)
}

/// Black with shift (for shifted lognormal) call price with unit annuity.
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry
/// * `shift` - Shift amount
pub fn black_shifted_price(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_price(forward + shift, strike + shift, sigma, t)
}

