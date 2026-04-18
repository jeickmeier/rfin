use finstack_analytics as fa;
use pyo3::prelude::*;

/// Simple returns from prices.
#[pyfunction]
fn simple_returns(prices: Vec<f64>) -> Vec<f64> {
    fa::returns::simple_returns(&prices)
}

/// Replace NaN/Inf values in returns with zero (in-place semantics via copy).
#[pyfunction]
fn clean_returns(returns: Vec<f64>) -> Vec<f64> {
    let mut r = returns;
    fa::returns::clean_returns(&mut r);
    r
}

/// Excess returns over a risk-free series.
#[pyfunction]
#[pyo3(signature = (returns, rf, nperiods = None))]
fn excess_returns(returns: Vec<f64>, rf: Vec<f64>, nperiods: Option<f64>) -> Vec<f64> {
    fa::returns::excess_returns(&returns, &rf, nperiods)
}

/// Convert returns to prices.
#[pyfunction]
#[pyo3(signature = (returns, base = 100.0))]
fn convert_to_prices(returns: Vec<f64>, base: f64) -> Vec<f64> {
    fa::returns::convert_to_prices(&returns, base)
}

/// Rebase a price series to start at ``base``.
#[pyfunction]
#[pyo3(signature = (prices, base = 100.0))]
fn rebase(prices: Vec<f64>, base: f64) -> Vec<f64> {
    fa::returns::rebase(&prices, base)
}

/// Cumulative compounded returns.
#[pyfunction]
fn comp_sum(returns: Vec<f64>) -> Vec<f64> {
    fa::returns::comp_sum(&returns)
}

/// Total compounded return.
#[pyfunction]
fn comp_total(returns: Vec<f64>) -> f64 {
    fa::returns::comp_total(&returns)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(simple_returns, m)?)?;
    m.add_function(wrap_pyfunction!(clean_returns, m)?)?;
    m.add_function(wrap_pyfunction!(excess_returns, m)?)?;
    m.add_function(wrap_pyfunction!(convert_to_prices, m)?)?;
    m.add_function(wrap_pyfunction!(rebase, m)?)?;
    m.add_function(wrap_pyfunction!(comp_sum, m)?)?;
    m.add_function(wrap_pyfunction!(comp_total, m)?)?;
    Ok(())
}
