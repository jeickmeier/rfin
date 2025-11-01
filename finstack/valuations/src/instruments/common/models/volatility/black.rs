//! Black/Black–Scholes common math helpers shared across instruments.

/// Calculate d1 for Black–Scholes (general form with dividends/carry)
///
/// Full Black-Scholes-Merton d1 calculation:
/// d1 = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
///
/// # Parameters
/// - `spot`: Spot price (or forward for Black76 when r=q=0)
/// - `strike`: Strike price
/// - `r`: Risk-free rate (or 0 for Black76)
/// - `sigma`: Volatility
/// - `t`: Time to expiry in years
/// - `q`: Dividend yield / cost of carry (or 0 for Black76)
#[inline]
pub fn d1(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return 0.0;
    }
    ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt())
}

/// Calculate d2 for Black–Scholes (general form with dividends/carry)
///
/// d2 = d1 - σ√T
#[inline]
pub fn d2(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    d1(spot, strike, r, sigma, t, q) - sigma * t.sqrt()
}

/// Calculate d1 for Black76 model (forward-based, no drift)
///
/// Black76 d1 calculation for rates/commodities:
/// d1 = [ln(F/K) + σ²T/2] / (σ√T)
///
/// This is equivalent to Black-Scholes with r=q=0.
/// Used for swaptions, caps/floors, and commodity options.
///
/// # Parameters
/// - `forward`: Forward price/rate
/// - `strike`: Strike price/rate
/// - `sigma`: Volatility
/// - `t`: Time to expiry in years
#[inline]
pub fn d1_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return 0.0;
    }
    let variance = sigma * sigma * t;
    ((forward / strike).ln() + 0.5 * variance) / variance.sqrt()
}

/// Calculate d2 for Black76 model
///
/// d2 = d1 - σ√T
#[inline]
pub fn d2_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    d1_black76(forward, strike, sigma, t) - sigma * t.sqrt()
}
