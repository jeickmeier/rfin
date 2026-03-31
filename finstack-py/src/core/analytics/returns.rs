//! Python bindings for standalone return computation functions.
//!
//! Thin wrappers around `finstack_analytics::returns` that accept and return
//! Python-native types (`list[float]`).

use finstack_analytics::returns;
use pyo3::prelude::*;

/// Clean a return series in place: replace infinities with NaN and strip
/// trailing NaN values.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Return series to clean. A *new* cleaned list is returned.
///
/// Returns
/// -------
/// list[float]
///     Cleaned return series with infinities replaced by NaN and trailing
///     NaN values removed.
#[pyfunction]
fn clean_returns(returns_vec: Vec<f64>) -> Vec<f64> {
    let mut r = returns_vec;
    returns::clean_returns(&mut r);
    r
}

/// Compute simple (percentage-change) returns from a price series.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Asset prices in chronological order.
///
/// Returns
/// -------
/// list[float]
///     Simple returns with the same length as ``prices``. The first element
///     is always ``0.0``.
#[pyfunction]
fn simple_returns(prices: Vec<f64>) -> Vec<f64> {
    returns::simple_returns(&prices)
}

/// Cumulative compounded returns: ``(1 + r).cumprod() - 1``.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// list[float]
///     Cumulative compounded return at each time step.
#[pyfunction]
fn comp_sum(returns_vec: Vec<f64>) -> Vec<f64> {
    returns::comp_sum(&returns_vec)
}

/// Total compounded return over the full series: ``prod(1 + r_i) - 1``.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Total compounded return.
#[pyfunction]
fn comp_total(returns_vec: Vec<f64>) -> f64 {
    returns::comp_total(&returns_vec)
}

/// Convert simple returns back to a price series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// base : float
///     Starting price level (e.g. ``100.0``).
///
/// Returns
/// -------
/// list[float]
///     Reconstructed price series of length ``len(returns) + 1``.
#[pyfunction]
fn convert_to_prices(returns_vec: Vec<f64>, base: f64) -> Vec<f64> {
    returns::convert_to_prices(&returns_vec, base)
}

/// Rebase a price series so the first value equals ``base``.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Price series to rebase.
/// base : float
///     Desired starting value (e.g. ``100.0``).
///
/// Returns
/// -------
/// list[float]
///     Rebased price series.
#[pyfunction]
fn rebase(prices: Vec<f64>, base: f64) -> Vec<f64> {
    returns::rebase(&prices, base)
}

/// Excess returns: portfolio returns minus risk-free returns.
///
/// When ``nperiods`` is provided, the risk-free rate is de-compounded to the
/// observation frequency before subtraction.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// rf : list[float]
///     Risk-free rate series, aligned with ``returns``.
/// nperiods : float, optional
///     Compounding periods per year. ``None`` uses ``rf`` directly.
///
/// Returns
/// -------
/// list[float]
///     Excess return series.
#[pyfunction]
#[pyo3(signature = (returns_vec, rf, nperiods=None))]
fn excess_returns(returns_vec: Vec<f64>, rf: Vec<f64>, nperiods: Option<f64>) -> Vec<f64> {
    returns::excess_returns(&returns_vec, &rf, nperiods)
}

/// Register standalone return functions into the given module and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(clean_returns, m)?)?;
    m.add_function(wrap_pyfunction!(simple_returns, m)?)?;
    m.add_function(wrap_pyfunction!(comp_sum, m)?)?;
    m.add_function(wrap_pyfunction!(comp_total, m)?)?;
    m.add_function(wrap_pyfunction!(convert_to_prices, m)?)?;
    m.add_function(wrap_pyfunction!(rebase, m)?)?;
    m.add_function(wrap_pyfunction!(excess_returns, m)?)?;
    Ok(vec![
        "clean_returns",
        "simple_returns",
        "comp_sum",
        "comp_total",
        "convert_to_prices",
        "rebase",
        "excess_returns",
    ])
}
