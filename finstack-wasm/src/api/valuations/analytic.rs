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
use finstack_valuations::instruments::models::closed_form::{bs_greeks, bs_price};
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
