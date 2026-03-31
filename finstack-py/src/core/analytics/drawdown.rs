//! Python bindings for standalone drawdown functions.
//!
//! Thin wrappers around `finstack_analytics::drawdown` that operate on
//! plain `list[float]` slices without requiring a `Performance` object.

use finstack_analytics::drawdown;
use pyo3::prelude::*;

/// Compute a drawdown series from simple returns.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// list[float]
///     Drawdown depth at each time step (values <= 0).
#[pyfunction]
fn to_drawdown_series(returns: Vec<f64>) -> Vec<f64> {
    drawdown::to_drawdown_series(&returns)
}

/// Maximum drawdown depth from a pre-computed drawdown series.
///
/// Parameters
/// ----------
/// drawdown : list[float]
///     Pre-computed drawdown series (values <= 0).
///
/// Returns
/// -------
/// float
///     Most negative value in the series, or ``0.0`` if empty.
#[pyfunction]
fn max_drawdown(drawdown_series: Vec<f64>) -> f64 {
    drawdown::max_drawdown(&drawdown_series)
}

/// Maximum drawdown computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Maximum drawdown depth.
#[pyfunction]
fn max_drawdown_from_returns(returns: Vec<f64>) -> f64 {
    drawdown::max_drawdown_from_returns(&returns)
}

/// Average drawdown depth across all periods.
///
/// Parameters
/// ----------
/// drawdown : list[float]
///     Drawdown series (values <= 0).
///
/// Returns
/// -------
/// float
///     Arithmetic mean of the drawdown series.
#[pyfunction]
fn average_drawdown(drawdown_series: Vec<f64>) -> f64 {
    drawdown::average_drawdown(&drawdown_series)
}

/// Calmar ratio: CAGR / |max drawdown|.
///
/// Parameters
/// ----------
/// cagr_val : float
///     Compound annual growth rate.
/// max_dd : float
///     Maximum drawdown (negative, e.g. ``-0.25``).
///
/// Returns
/// -------
/// float
///     Calmar ratio, or ``0.0`` if ``max_dd`` is zero.
#[pyfunction]
fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    drawdown::calmar(cagr_val, max_dd)
}

/// Calmar ratio computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// ann_factor : float
///     Annualization factor (e.g. ``252.0`` for daily).
///
/// Returns
/// -------
/// float
///     Calmar ratio.
#[pyfunction]
fn calmar_from_returns(returns: Vec<f64>, ann_factor: f64) -> f64 {
    drawdown::calmar_from_returns(&returns, ann_factor)
}

/// Pain index: mean absolute drawdown.
///
/// Parameters
/// ----------
/// drawdown : list[float]
///     Pre-computed drawdown series (values <= 0).
///
/// Returns
/// -------
/// float
///     Pain index (non-negative).
#[pyfunction]
fn pain_index(drawdown_series: Vec<f64>) -> f64 {
    drawdown::pain_index(&drawdown_series)
}

/// Ulcer index: root-mean-square of drawdown depths.
///
/// Parameters
/// ----------
/// drawdown : list[float]
///     Pre-computed drawdown series.
///
/// Returns
/// -------
/// float
///     Ulcer index (non-negative).
#[pyfunction]
fn ulcer_index(drawdown_series: Vec<f64>) -> f64 {
    drawdown::ulcer_index(&drawdown_series)
}

/// Conditional Drawdown at Risk (CDaR).
///
/// Parameters
/// ----------
/// drawdown : list[float]
///     Pre-computed drawdown series.
/// confidence : float
///     Confidence level in ``(0, 1)``, e.g. ``0.95``.
///
/// Returns
/// -------
/// float
///     CDaR as a non-negative scalar.
#[pyfunction]
fn cdar(drawdown_series: Vec<f64>, confidence: f64) -> f64 {
    drawdown::cdar(&drawdown_series, confidence)
}

/// Recovery factor: total return / |max drawdown|.
///
/// Parameters
/// ----------
/// total_return : float
///     Total compounded return.
/// max_dd : float
///     Maximum drawdown (negative).
///
/// Returns
/// -------
/// float
///     Recovery factor.
#[pyfunction]
fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    drawdown::recovery_factor(total_return, max_dd)
}

/// Recovery factor computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Recovery factor.
#[pyfunction]
fn recovery_factor_from_returns(returns: Vec<f64>) -> f64 {
    drawdown::recovery_factor_from_returns(&returns)
}

/// Martin ratio: CAGR / Ulcer Index.
///
/// Parameters
/// ----------
/// cagr_val : float
///     Compound annual growth rate.
/// ulcer : float
///     Ulcer index value.
///
/// Returns
/// -------
/// float
///     Martin ratio.
#[pyfunction]
fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    drawdown::martin_ratio(cagr_val, ulcer)
}

/// Martin ratio computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// ann_factor : float
///     Annualization factor.
///
/// Returns
/// -------
/// float
///     Martin ratio.
#[pyfunction]
fn martin_ratio_from_returns(returns: Vec<f64>, ann_factor: f64) -> f64 {
    drawdown::martin_ratio_from_returns(&returns, ann_factor)
}

/// Sterling ratio: (CAGR - Rf) / |avg drawdown|.
///
/// Parameters
/// ----------
/// cagr_val : float
///     Compound annual growth rate.
/// avg_dd : float
///     Average worst drawdowns (negative).
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Sterling ratio.
#[pyfunction]
fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Sterling ratio computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// ann_factor : float
///     Annualization factor.
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Sterling ratio.
#[pyfunction]
fn sterling_ratio_from_returns(returns: Vec<f64>, ann_factor: f64, risk_free_rate: f64) -> f64 {
    drawdown::sterling_ratio_from_returns(&returns, ann_factor, risk_free_rate)
}

/// Burke ratio: (CAGR - Rf) / RMS of worst drawdowns.
///
/// Parameters
/// ----------
/// cagr_val : float
///     Compound annual growth rate.
/// dd_episodes : list[float]
///     Max-drawdown depths of each episode (negative).
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Burke ratio.
#[pyfunction]
fn burke_ratio(cagr_val: f64, dd_episodes: Vec<f64>, risk_free_rate: f64) -> f64 {
    drawdown::burke_ratio(cagr_val, &dd_episodes, risk_free_rate)
}

/// Pain ratio: (CAGR - Rf) / Pain Index.
///
/// Parameters
/// ----------
/// cagr_val : float
///     Compound annual growth rate.
/// pain : float
///     Pain index value.
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Pain ratio.
#[pyfunction]
fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    drawdown::pain_ratio(cagr_val, pain, risk_free_rate)
}

/// Pain ratio computed directly from a returns series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// ann_factor : float
///     Annualization factor.
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Pain ratio.
#[pyfunction]
fn pain_ratio_from_returns(returns: Vec<f64>, ann_factor: f64, risk_free_rate: f64) -> f64 {
    drawdown::pain_ratio_from_returns(&returns, ann_factor, risk_free_rate)
}

/// Register standalone drawdown functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(to_drawdown_series, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(average_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(calmar, m)?)?;
    m.add_function(wrap_pyfunction!(calmar_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(pain_index, m)?)?;
    m.add_function(wrap_pyfunction!(ulcer_index, m)?)?;
    m.add_function(wrap_pyfunction!(cdar, m)?)?;
    m.add_function(wrap_pyfunction!(recovery_factor, m)?)?;
    m.add_function(wrap_pyfunction!(recovery_factor_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(martin_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(martin_ratio_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(sterling_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(sterling_ratio_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(burke_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(pain_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(pain_ratio_from_returns, m)?)?;
    Ok(vec![
        "to_drawdown_series",
        "max_drawdown",
        "max_drawdown_from_returns",
        "average_drawdown",
        "calmar",
        "calmar_from_returns",
        "pain_index",
        "ulcer_index",
        "cdar",
        "recovery_factor",
        "recovery_factor_from_returns",
        "martin_ratio",
        "martin_ratio_from_returns",
        "sterling_ratio",
        "sterling_ratio_from_returns",
        "burke_ratio",
        "pain_ratio",
        "pain_ratio_from_returns",
    ])
}
