//! Closed-form analytic option primitives (Black-Scholes, Black-76, implied vol).
//!
//! Thin wasm-bindgen wrappers around the Rust closed-form formulas in
//! `finstack_valuations::instruments::models::closed_form`.
//!
//! All rates are continuously compounded decimals; `sigma` is annualized vol;
//! `t` is time to expiry in years. Greeks scale matches the Rust crate:
//! `vega` and both rho values are per 1% move, `theta` is per-day under the
//! `thetaDays` day-count (ACT/365 by default).

use crate::utils::to_js_err;
use finstack_valuations::instruments::models::closed_form::implied_vol::{
    black76_implied_vol, bs_implied_vol,
};
use finstack_valuations::instruments::models::closed_form::{
    arithmetic_asian_call_tw, arithmetic_asian_put_tw, bs_greeks, bs_price, down_in_call,
    down_out_call, fixed_strike_lookback_call, fixed_strike_lookback_put,
    floating_strike_lookback_call, floating_strike_lookback_put, geometric_asian_call,
    geometric_asian_put, quanto_call, quanto_put, up_in_call, up_out_call,
};
use finstack_valuations::instruments::OptionType;
use wasm_bindgen::prelude::*;

fn option_type(is_call: bool) -> OptionType {
    if is_call {
        OptionType::Call
    } else {
        OptionType::Put
    }
}

/// Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.
#[wasm_bindgen(js_name = bsPrice)]
pub fn bs_price_js(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
) -> f64 {
    bs_price(spot, strike, r, q, sigma, t, option_type(is_call))
}

/// Black-Scholes / Garman-Kohlhagen Greeks as a `{delta, gamma, vega, theta, rho, rhoQ}` object.
///
/// `thetaDays` is the day-count denominator for theta (default 365).
#[wasm_bindgen(js_name = bsGreeks)]
#[allow(clippy::too_many_arguments)]
pub fn bs_greeks_js(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
    theta_days: Option<f64>,
) -> Result<JsValue, JsValue> {
    let g = bs_greeks(
        spot,
        strike,
        r,
        q,
        sigma,
        t,
        option_type(is_call),
        theta_days.unwrap_or(365.0),
    );
    let value = serde_json::json!({
        "delta": g.delta,
        "gamma": g.gamma,
        "vega": g.vega,
        "theta": g.theta,
        "rho": g.rho_r,
        "rhoQ": g.rho_q,
    });
    serde_wasm_bindgen::to_value(&value).map_err(to_js_err)
}

/// Solve for Black-Scholes / Garman-Kohlhagen implied volatility.
#[wasm_bindgen(js_name = bsImpliedVol)]
pub fn bs_implied_vol_js(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    bs_implied_vol(spot, strike, r, q, t, option_type(is_call), price).map_err(to_js_err)
}

/// Solve for Black-76 (forward-based) implied volatility.
#[wasm_bindgen(js_name = black76ImpliedVol)]
pub fn black76_implied_vol_js(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    black76_implied_vol(forward, strike, df, t, option_type(is_call), price).map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// Closed-form exotics: barrier / asian / lookback / quanto
// ---------------------------------------------------------------------------

/// Reiner-Rubinstein continuous-monitoring barrier call price.
///
/// `direction` is `"up"` or `"down"`, `knock` is `"in"` or `"out"`.
#[wasm_bindgen(js_name = barrierCall)]
#[allow(clippy::too_many_arguments)]
pub fn barrier_call_js(
    spot: f64,
    strike: f64,
    barrier: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    direction: &str,
    knock: &str,
) -> Result<f64, JsValue> {
    Ok(match (direction, knock) {
        ("up", "in") => up_in_call(spot, strike, barrier, t, r, q, sigma),
        ("up", "out") => up_out_call(spot, strike, barrier, t, r, q, sigma),
        ("down", "in") => down_in_call(spot, strike, barrier, t, r, q, sigma),
        ("down", "out") => down_out_call(spot, strike, barrier, t, r, q, sigma),
        _ => {
            return Err(to_js_err(format!(
                "unknown barrier spec: direction='{direction}' knock='{knock}'"
            )));
        }
    })
}

/// Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option.
#[wasm_bindgen(js_name = asianOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn asian_option_price_js(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    num_fixings: usize,
    averaging: Option<String>,
    is_call: Option<bool>,
) -> Result<f64, JsValue> {
    let averaging = averaging.as_deref().unwrap_or("arithmetic");
    let is_call = is_call.unwrap_or(true);
    Ok(match (averaging, is_call) {
        ("arithmetic", true) => arithmetic_asian_call_tw(spot, strike, t, r, q, sigma, num_fixings),
        ("arithmetic", false) => arithmetic_asian_put_tw(spot, strike, t, r, q, sigma, num_fixings),
        ("geometric", true) => geometric_asian_call(spot, strike, t, r, q, sigma, num_fixings),
        ("geometric", false) => geometric_asian_put(spot, strike, t, r, q, sigma, num_fixings),
        _ => {
            return Err(to_js_err(format!(
                "unknown averaging '{averaging}'; expected 'arithmetic' or 'geometric'"
            )));
        }
    })
}

/// Conze-Viswanathan lookback option.
///
/// `strike_type` is `"fixed"` (default) or `"floating"`. For `"floating"`,
/// `strike` is ignored and `extremum` is the observed min/max to date.
#[wasm_bindgen(js_name = lookbackOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn lookback_option_price_js(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    extremum: f64,
    strike_type: Option<String>,
    is_call: Option<bool>,
) -> Result<f64, JsValue> {
    let strike_type = strike_type.as_deref().unwrap_or("fixed");
    let is_call = is_call.unwrap_or(true);
    Ok(match (strike_type, is_call) {
        ("fixed", true) => fixed_strike_lookback_call(spot, strike, t, r, q, sigma, extremum),
        ("fixed", false) => fixed_strike_lookback_put(spot, strike, t, r, q, sigma, extremum),
        ("floating", true) => floating_strike_lookback_call(spot, t, r, q, sigma, extremum),
        ("floating", false) => floating_strike_lookback_put(spot, t, r, q, sigma, extremum),
        _ => {
            return Err(to_js_err(format!(
                "unknown strike_type '{strike_type}'; expected 'fixed' or 'floating'"
            )));
        }
    })
}

/// Quanto option (FX-adjusted cross-currency) price in domestic currency.
#[wasm_bindgen(js_name = quantoOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn quanto_option_price_js(
    spot: f64,
    strike: f64,
    t: f64,
    rate_domestic: f64,
    rate_foreign: f64,
    div_yield: f64,
    vol_asset: f64,
    vol_fx: f64,
    correlation: f64,
    is_call: Option<bool>,
) -> f64 {
    if is_call.unwrap_or(true) {
        quanto_call(
            spot,
            strike,
            t,
            rate_domestic,
            rate_foreign,
            div_yield,
            vol_asset,
            vol_fx,
            correlation,
        )
    } else {
        quanto_put(
            spot,
            strike,
            t,
            rate_domestic,
            rate_foreign,
            div_yield,
            vol_asset,
            vol_fx,
            correlation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_price_call_atm_is_positive() {
        let p = bs_price_js(100.0, 100.0, 0.05, 0.02, 0.2, 1.0, true);
        assert!(p > 0.0);
    }

    #[test]
    fn bs_implied_vol_recovers_sigma() {
        let sigma = 0.25;
        let price = bs_price_js(100.0, 110.0, 0.03, 0.01, sigma, 0.75, true);
        let iv = bs_implied_vol_js(100.0, 110.0, 0.03, 0.01, 0.75, price, true)
            .expect("solver should converge");
        assert!((iv - sigma).abs() < 1e-6, "iv={iv} sigma={sigma}");
    }
}
