//! Black/Black–Scholes common math helpers shared across instruments.

use finstack_core::F;

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
