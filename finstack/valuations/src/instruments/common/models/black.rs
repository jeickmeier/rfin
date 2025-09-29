//! Black/Black–Scholes common math helpers shared across instruments.



/// Calculate d1 for Black–Scholes
#[inline]
pub fn d1(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return 0.0;
    }
    ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt())
}

/// Calculate d2 for Black–Scholes
#[inline]
pub fn d2(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    d1(spot, strike, r, sigma, t, q) - sigma * t.sqrt()
}
