//! Analytical closed-form pricing formulas.

use pyo3::prelude::*;

/// Black-Scholes call price.
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
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_call(
        spot, strike, rate, div_yield, vol, expiry,
    )
}

/// Black-Scholes put price.
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
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_put(
        spot, strike, rate, div_yield, vol, expiry,
    )
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_scholes_call, m)?)?;
    m.add_function(wrap_pyfunction!(black_scholes_put, m)?)?;
    Ok(())
}
