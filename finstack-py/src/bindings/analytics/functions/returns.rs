//! Python bindings for return-series transforms: simple returns, excess
//! returns, compounding, rebasing, and price reconstruction.
//!
//! All functions here are pure (no DataFrame dependency) and accept Python
//! lists / NumPy arrays of floats. Returns are simple decimal returns
//! (`0.01` = 1%) unless explicitly stated otherwise.

use finstack_analytics as fa;
use pyo3::prelude::*;

/// Compute simple returns from a price series.
///
/// Args:
///     prices: Sequence of period-end prices (e.g. close prices).
///
/// Returns:
///     A list of simple returns aligned to `prices[1..]`. The first element
///     is ``NaN`` to keep the output the same length as the input.
///     Non-positive or non-finite prices produce ``NaN``.
#[pyfunction]
fn simple_returns(prices: Vec<f64>) -> Vec<f64> {
    fa::returns::simple_returns(&prices)
}

/// Replace ``±inf`` with ``NaN`` and strip trailing ``NaN`` entries.
///
/// Args:
///     returns: Raw return series, possibly containing infinities or
///         trailing missing values.
///
/// Returns:
///     A new list with infinities replaced by ``NaN`` and trailing ``NaN``
///     values removed. Interior ``NaN`` values are preserved.
#[pyfunction]
fn clean_returns(returns: Vec<f64>) -> Vec<f64> {
    let mut r = returns;
    fa::returns::clean_returns(&mut r);
    r
}

/// Subtract a (possibly de-compounded) risk-free series from returns.
///
/// Args:
///     returns: Period simple returns.
///     rf: Risk-free series aligned with ``returns``.
///     nperiods: If provided, the risk-free rate is treated as **annual**
///         and de-compounded to the per-period rate by
///         ``(1 + rf) ** (1 / nperiods) - 1`` before subtraction. Use the
///         number of periods per year (e.g. ``252`` for daily data). If
///         ``None``, ``rf`` is treated as already on the same period
///         frequency as ``returns``. Non-finite or non-positive values of
///         ``nperiods`` propagate as ``NaN``.
///
/// Returns:
///     ``returns - rf`` element-wise.
#[pyfunction]
#[pyo3(signature = (returns, rf, nperiods = None))]
fn excess_returns(returns: Vec<f64>, rf: Vec<f64>, nperiods: Option<f64>) -> Vec<f64> {
    fa::returns::excess_returns(&returns, &rf, nperiods)
}

/// Reconstruct a price path from returns, starting at ``base``.
///
/// Args:
///     returns: Period simple returns.
///     base: Starting price for the reconstructed path. Defaults to
///         ``100.0``.
///
/// Returns:
///     A price series of length ``len(returns) + 1`` such that
///     ``prices[0] == base`` and successive prices compound the input
///     returns multiplicatively.
#[pyfunction]
#[pyo3(signature = (returns, base = 100.0))]
fn convert_to_prices(returns: Vec<f64>, base: f64) -> Vec<f64> {
    fa::returns::convert_to_prices(&returns, base)
}

/// Rebase a price series so that the first observation equals ``base``.
///
/// Args:
///     prices: Price series to rebase.
///     base: Target value for the first observation. Defaults to ``100.0``.
///
/// Returns:
///     A rebased price series of the same length as ``prices``. If
///     ``prices[0]`` is zero or non-finite the output contains ``NaN``.
#[pyfunction]
#[pyo3(signature = (prices, base = 100.0))]
fn rebase(prices: Vec<f64>, base: f64) -> Vec<f64> {
    fa::returns::rebase(&prices, base)
}

/// Cumulative compounded returns.
///
/// Args:
///     returns: Period simple returns.
///
/// Returns:
///     A series of the same length where element ``i`` equals
///     ``prod(1 + returns[..=i]) - 1``. Accumulates in log space using
///     compensated summation for numerical stability over long series.
#[pyfunction]
fn comp_sum(returns: Vec<f64>) -> Vec<f64> {
    fa::returns::comp_sum(&returns)
}

/// Total compounded return of a series.
///
/// Args:
///     returns: Period simple returns.
///
/// Returns:
///     ``prod(1 + returns) - 1`` as a single scalar. Returns ``0.0`` for
///     an empty input.
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
