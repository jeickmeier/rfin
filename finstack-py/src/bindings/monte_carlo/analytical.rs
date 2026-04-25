//! Analytical closed-form pricing formulas.
//!
//! # Argument-order note
//!
//! The Python signature is `(spot, strike, rate, div_yield, vol, expiry)`,
//! which matches the typical Python caller mental model (the rate-and-yield
//! pair sits next to the volatility).
//!
//! The underlying Rust function in `finstack_monte_carlo::variance_reduction::
//! control_variate` orders its arguments as
//! `(spot, strike, time_to_maturity, rate, dividend_yield, volatility)` — i.e.
//! `expiry` is in third position, not last. The wrappers below intentionally
//! re-order at the boundary; the comment marks the swap so a future maintainer
//! does not "fix" the apparent inconsistency.

use pyo3::prelude::*;

/// Black-Scholes call price.
///
/// Argument order is `(spot, strike, rate, div_yield, vol, expiry)`. Internally
/// re-ordered to the Rust crate's `(spot, strike, expiry, rate, q, vol)` layout.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry))]
fn black_scholes_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    // Rust crate order: (spot, strike, time_to_maturity, rate, dividend_yield, volatility).
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_call(
        spot, strike, expiry, rate, div_yield, vol,
    )
}

/// Black-Scholes put price.
///
/// Argument order is `(spot, strike, rate, div_yield, vol, expiry)`. Internally
/// re-ordered to the Rust crate's `(spot, strike, expiry, rate, q, vol)` layout.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry))]
fn black_scholes_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    // Rust crate order: (spot, strike, time_to_maturity, rate, dividend_yield, volatility).
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_put(
        spot, strike, expiry, rate, div_yield, vol,
    )
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_scholes_call, m)?)?;
    m.add_function(wrap_pyfunction!(black_scholes_put, m)?)?;
    Ok(())
}
