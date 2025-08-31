//! Black/Black–Scholes common math helpers shared across instruments.

use finstack_core::F;
use std::f64::consts::PI;

/// Cumulative standard normal distribution function
#[inline]
pub fn norm_cdf(x: F) -> F {
    0.5 * (1.0 + erf(x / (2.0_f64).sqrt()))
}

/// Standard normal probability density function
#[inline]
pub fn norm_pdf(x: F) -> F {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// Error function approximation (Abramowitz and Stegun)
#[inline]
fn erf(x: F) -> F {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let y = 1.0 - ((((a5 * t5 + a4 * t4) + a3 * t3) + a2 * t2) + a1 * t) * (-x * x).exp();

    sign * y
}

/// Calculate d1 for Black–Scholes
#[inline]
pub fn d1(spot: F, strike: F, r: F, sigma: F, t: F, q: F) -> F {
    if t <= 0.0 || sigma <= 0.0 {
        return 0.0;
    }
    ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt())
}

/// Calculate d2 for Black–Scholes
#[inline]
pub fn d2(spot: F, strike: F, r: F, sigma: F, t: F, q: F) -> F {
    d1(spot, strike, r, sigma, t, q) - sigma * t.sqrt()
}


