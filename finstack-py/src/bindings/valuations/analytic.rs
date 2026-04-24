//! Closed-form analytic option primitives (Black-Scholes, Black-76, implied vol).
//!
//! Thin wrappers around `finstack_valuations::instruments::models::closed_form`
//! that expose the per-unit pricing and Greek formulas to Python without
//! requiring a full `MarketContext` / `Instrument` round trip.
//!
//! Conventions mirror the underlying Rust crate:
//!
//! - `r`, `q` are continuously-compounded annualized rates (decimal).
//! - `sigma` is annualized volatility (decimal).
//! - `t` is time to expiry in years.
//! - Greeks use the canonical Rust scaling: `vega` and `rho_*` are per-1% move,
//!   `theta` is per day under ACT/365 (use 252 day-count via `theta_days` if you
//!   want a business-day convention).

use crate::errors::display_to_py;
use finstack_valuations::instruments::models::closed_form::implied_vol::{
    black76_implied_vol, bs_implied_vol,
};
use finstack_valuations::instruments::models::closed_form::{bs_greeks, bs_price, BsGreeks};
use finstack_valuations::instruments::OptionType;
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn option_type(is_call: bool) -> OptionType {
    if is_call {
        OptionType::Call
    } else {
        OptionType::Put
    }
}

// ---------------------------------------------------------------------------
// bs_price
// ---------------------------------------------------------------------------

/// Black-Scholes / Garman-Kohlhagen per-unit price of a European option.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Present-value option price (per unit; multiply by contract size to scale).
#[pyfunction(name = "bs_price")]
#[pyo3(signature = (spot, strike, r, q, sigma, t, is_call))]
fn bs_price_wrapper(
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

// ---------------------------------------------------------------------------
// bs_greeks
// ---------------------------------------------------------------------------

/// Black-Scholes / Garman-Kohlhagen Greeks for a European option.
///
/// Returns a dict with ``delta``, ``gamma``, ``vega``, ``theta``, ``rho`` (=rho_r),
/// and ``rho_q``. ``vega`` and both rho values are per 1% move; ``theta`` is
/// per day using the `theta_days` day-count (ACT/365 by default).
///
/// Parameters
/// ----------
/// spot, strike, r, q, sigma, t, is_call
///     Same as :func:`bs_price`.
/// theta_days : float, optional
///     Day-count denominator for per-day theta (default ``365.0``). Pass
///     ``252.0`` for business-day-scaled theta, ``360.0`` for ACT/360.
///
/// Returns
/// -------
/// dict
///     ``{"delta": ..., "gamma": ..., "vega": ..., "theta": ..., "rho": ..., "rho_q": ...}``.
#[pyfunction(name = "bs_greeks")]
#[pyo3(signature = (spot, strike, r, q, sigma, t, is_call, theta_days=365.0))]
#[allow(clippy::too_many_arguments)]
fn bs_greeks_wrapper<'py>(
    py: Python<'py>,
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
    theta_days: f64,
) -> PyResult<Bound<'py, PyDict>> {
    let greeks: BsGreeks = bs_greeks(
        spot,
        strike,
        r,
        q,
        sigma,
        t,
        option_type(is_call),
        theta_days,
    );
    let out = PyDict::new(py);
    out.set_item("delta", greeks.delta)?;
    out.set_item("gamma", greeks.gamma)?;
    out.set_item("vega", greeks.vega)?;
    out.set_item("theta", greeks.theta)?;
    out.set_item("rho", greeks.rho_r)?;
    out.set_item("rho_q", greeks.rho_q)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// bs_implied_vol
// ---------------------------------------------------------------------------

/// Solve for Black-Scholes / Garman-Kohlhagen implied volatility.
///
/// Uses a Newton-in-vega hybrid with bisection fallback. Returns ``0.0`` when
/// ``t <= 0`` (expired — volatility is undefined); raises on non-finite inputs
/// or target prices outside the no-arbitrage bracket.
///
/// Parameters
/// ----------
/// spot, strike, r, q, t, is_call
///     Same as :func:`bs_price`.
/// price : float
///     Target per-unit option price.
///
/// Returns
/// -------
/// float
///     Implied volatility (annualized, decimal).
#[pyfunction(name = "bs_implied_vol")]
#[pyo3(signature = (spot, strike, r, q, t, price, is_call))]
fn bs_implied_vol_wrapper(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> PyResult<f64> {
    bs_implied_vol(spot, strike, r, q, t, option_type(is_call), price).map_err(display_to_py)
}

// ---------------------------------------------------------------------------
// black76_implied_vol
// ---------------------------------------------------------------------------

/// Solve for Black-76 (forward-based) implied volatility.
///
/// Takes a forward price, strike, discount factor, time to expiry, and target
/// price; returns the lognormal implied vol consistent with the Black-76
/// pricing formula.
///
/// Parameters
/// ----------
/// forward : float
///     Forward price `F`.
/// strike : float
///     Strike `K`.
/// df : float
///     Discount factor from expiry to settlement (``exp(-r * t)`` for
///     continuously-compounded rate ``r``).
/// t : float
///     Time to expiry in years.
/// price : float
///     Target per-unit option price.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Implied volatility (annualized, decimal).
#[pyfunction(name = "black76_implied_vol")]
#[pyo3(signature = (forward, strike, df, t, price, is_call))]
fn black76_implied_vol_wrapper(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> PyResult<f64> {
    black76_implied_vol(forward, strike, df, t, option_type(is_call), price).map_err(display_to_py)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register the analytic option primitives on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bs_price_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(bs_greeks_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(bs_implied_vol_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(black76_implied_vol_wrapper, m)?)?;
    Ok(())
}
