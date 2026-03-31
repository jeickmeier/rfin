use crate::core::error::js_error;
use wasm_bindgen::prelude::*;

/// Black-76 call price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Call price
#[wasm_bindgen(js_name = blackCall)]
pub fn black_call_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_call(fwd, strike, sigma, t)
}

/// Black-76 put price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Put price
#[wasm_bindgen(js_name = blackPut)]
pub fn black_put_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_put(fwd, strike, sigma, t)
}

/// Black-76 vega (sensitivity to volatility).
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Vega
#[wasm_bindgen(js_name = blackVega)]
pub fn black_vega_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_vega(fwd, strike, sigma, t)
}

/// Black-76 call delta.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Call delta
#[wasm_bindgen(js_name = blackDeltaCall)]
pub fn black_delta_call_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_delta_call(fwd, strike, sigma, t)
}

/// Black-76 put delta.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Put delta
#[wasm_bindgen(js_name = blackDeltaPut)]
pub fn black_delta_put_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_delta_put(fwd, strike, sigma, t)
}

/// Black-76 gamma.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Gamma
#[wasm_bindgen(js_name = blackGamma)]
pub fn black_gamma_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::black_gamma(fwd, strike, sigma, t)
}

/// Bachelier (normal model) call price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Normal volatility (absolute, e.g. 50 bps)
/// @param {number} t - Time to expiry in years
/// @returns {number} Call price
#[wasm_bindgen(js_name = bachelierCall)]
pub fn bachelier_call_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::bachelier_call(fwd, strike, sigma, t)
}

/// Bachelier (normal model) put price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Normal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Put price
#[wasm_bindgen(js_name = bachelierPut)]
pub fn bachelier_put_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::bachelier_put(fwd, strike, sigma, t)
}

/// Bachelier (normal model) vega.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Normal volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Vega
#[wasm_bindgen(js_name = bachelierVega)]
pub fn bachelier_vega_js(fwd: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    finstack_core::math::volatility::bachelier_vega(fwd, strike, sigma, t)
}

/// Shifted Black call price for low/negative rate environments.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @param {number} shift - Additive shift (e.g. 0.03 for 3%)
/// @returns {number} Call price
#[wasm_bindgen(js_name = blackShiftedCall)]
pub fn black_shifted_call_js(fwd: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    finstack_core::math::volatility::black_shifted_call(fwd, strike, sigma, t, shift)
}

/// Shifted Black put price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @param {number} shift - Additive shift
/// @returns {number} Put price
#[wasm_bindgen(js_name = blackShiftedPut)]
pub fn black_shifted_put_js(fwd: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    finstack_core::math::volatility::black_shifted_put(fwd, strike, sigma, t, shift)
}

/// Shifted Black vega.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} sigma - Lognormal volatility
/// @param {number} t - Time to expiry in years
/// @param {number} shift - Additive shift
/// @returns {number} Vega
#[wasm_bindgen(js_name = blackShiftedVega)]
pub fn black_shifted_vega_js(fwd: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    finstack_core::math::volatility::black_shifted_vega(fwd, strike, sigma, t, shift)
}

/// Black-Scholes-Merton spot call price with continuous carry.
///
/// @param {number} spot - Spot price
/// @param {number} strike - Strike price
/// @param {number} r - Risk-free rate
/// @param {number} dividendYield - Continuous dividend yield
/// @param {number} sigma - Volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Call price
#[wasm_bindgen(js_name = blackScholesSpotCall)]
pub fn black_scholes_spot_call_js(
    spot: f64,
    strike: f64,
    r: f64,
    dividend_yield: f64,
    sigma: f64,
    t: f64,
) -> f64 {
    finstack_core::math::volatility::black_scholes_spot_call(
        spot,
        strike,
        r,
        dividend_yield,
        sigma,
        t,
    )
}

/// Black-Scholes-Merton spot put price with continuous carry.
///
/// @param {number} spot - Spot price
/// @param {number} strike - Strike price
/// @param {number} r - Risk-free rate
/// @param {number} dividendYield - Continuous dividend yield
/// @param {number} sigma - Volatility
/// @param {number} t - Time to expiry in years
/// @returns {number} Put price
#[wasm_bindgen(js_name = blackScholesSpotPut)]
pub fn black_scholes_spot_put_js(
    spot: f64,
    strike: f64,
    r: f64,
    dividend_yield: f64,
    sigma: f64,
    t: f64,
) -> f64 {
    finstack_core::math::volatility::black_scholes_spot_put(
        spot,
        strike,
        r,
        dividend_yield,
        sigma,
        t,
    )
}

/// Implied Black-76 volatility from option price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} t - Time to expiry in years
/// @param {number} price - Option market price
/// @param {boolean} isCall - True for call, false for put
/// @returns {number} Implied lognormal volatility
#[wasm_bindgen(js_name = impliedVolBlack)]
pub fn implied_vol_black_js(
    fwd: f64,
    strike: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    finstack_core::math::volatility::implied_vol_black(fwd, strike, t, price, is_call)
        .map_err(|e| js_error(e.to_string()))
}

/// Implied Bachelier (normal) volatility from option price.
///
/// @param {number} fwd - Forward price
/// @param {number} strike - Strike price
/// @param {number} t - Time to expiry in years
/// @param {number} price - Option market price
/// @param {boolean} isCall - True for call, false for put
/// @returns {number} Implied normal volatility
#[wasm_bindgen(js_name = impliedVolBachelier)]
pub fn implied_vol_bachelier_js(
    fwd: f64,
    strike: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    finstack_core::math::volatility::implied_vol_bachelier(fwd, strike, t, price, is_call)
        .map_err(|e| js_error(e.to_string()))
}

/// Geometric-average Asian call price (Kemna-Vorst with discrete-monitoring adjustment).
///
/// @param {number} spot - Spot price
/// @param {number} strike - Strike price
/// @param {number} t - Time to expiry in years
/// @param {number} r - Risk-free rate
/// @param {number} divYield - Continuous dividend yield
/// @param {number} vol - Lognormal volatility
/// @param {number} numFixings - Number of discrete fixing dates
/// @returns {number} Call price
#[wasm_bindgen(js_name = geometricAsianCall)]
pub fn geometric_asian_call_js(
    spot: f64,
    strike: f64,
    t: f64,
    r: f64,
    div_yield: f64,
    vol: f64,
    num_fixings: usize,
) -> f64 {
    finstack_core::math::volatility::geometric_asian_call(
        spot,
        strike,
        t,
        r,
        div_yield,
        vol,
        num_fixings,
    )
}
