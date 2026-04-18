use super::super::types::PyDrawdownEpisode;
use crate::bindings::core::dates::utils::py_to_date;
use finstack_analytics as fa;
use pyo3::prelude::*;

/// Drawdown series from returns.
#[pyfunction]
fn to_drawdown_series(returns: Vec<f64>) -> Vec<f64> {
    fa::drawdown::to_drawdown_series(&returns)
}

/// Top-N drawdown episodes with date information.
#[pyfunction]
#[pyo3(signature = (drawdown, dates, n = 5))]
fn drawdown_details(
    drawdown: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    n: usize,
) -> PyResult<Vec<PyDrawdownEpisode>> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(fa::drawdown::drawdown_details(&drawdown, &rd, n)
        .into_iter()
        .map(|e| PyDrawdownEpisode { inner: e })
        .collect())
}

/// Average of the N deepest drawdowns.
#[pyfunction]
#[pyo3(signature = (drawdown, n = 5))]
fn mean_episode_drawdown(drawdown: Vec<f64>, n: usize) -> f64 {
    fa::drawdown::mean_episode_drawdown(&drawdown, n)
}

/// Simple arithmetic average of drawdown values.
#[pyfunction]
fn mean_drawdown(drawdowns: Vec<f64>) -> f64 {
    fa::drawdown::mean_drawdown(&drawdowns)
}

/// Maximum drawdown from a drawdown series (already computed from returns).
#[pyfunction]
fn max_drawdown(drawdown: Vec<f64>) -> f64 {
    fa::drawdown::max_drawdown(&drawdown)
}

/// Maximum drawdown duration in calendar days.
#[pyfunction]
fn max_drawdown_duration(drawdown: Vec<f64>, dates: Vec<Bound<'_, PyAny>>) -> PyResult<i64> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(fa::drawdown::max_drawdown_duration(&drawdown, &rd))
}

/// Conditional Drawdown at Risk.
#[pyfunction]
#[pyo3(signature = (drawdown, confidence = 0.95))]
fn cdar(drawdown: Vec<f64>, confidence: f64) -> f64 {
    fa::drawdown::cdar(&drawdown, confidence)
}

/// Ulcer index.
#[pyfunction]
fn ulcer_index(drawdown: Vec<f64>) -> f64 {
    fa::drawdown::ulcer_index(&drawdown)
}

/// Pain index (average drawdown depth).
#[pyfunction]
fn pain_index(drawdown: Vec<f64>) -> f64 {
    fa::drawdown::pain_index(&drawdown)
}

/// Calmar ratio from pre-computed CAGR and max drawdown.
#[pyfunction]
fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    fa::drawdown::calmar(cagr_val, max_dd)
}

/// Recovery factor from pre-computed values.
#[pyfunction]
fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    fa::drawdown::recovery_factor(total_return, max_dd)
}

/// Martin ratio from pre-computed values.
#[pyfunction]
fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    fa::drawdown::martin_ratio(cagr_val, ulcer)
}

/// Sterling ratio from pre-computed values.
#[pyfunction]
#[pyo3(signature = (cagr_val, avg_dd, risk_free_rate = 0.0))]
fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Burke ratio from pre-computed values.
#[pyfunction]
#[pyo3(signature = (cagr_val, dd_episodes, risk_free_rate = 0.0))]
fn burke_ratio(cagr_val: f64, dd_episodes: Vec<f64>, risk_free_rate: f64) -> f64 {
    fa::drawdown::burke_ratio(cagr_val, &dd_episodes, risk_free_rate)
}

/// Pain ratio from pre-computed values.
#[pyfunction]
#[pyo3(signature = (cagr_val, pain, risk_free_rate = 0.0))]
fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::pain_ratio(cagr_val, pain, risk_free_rate)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_drawdown_series, m)?)?;
    m.add_function(wrap_pyfunction!(drawdown_details, m)?)?;
    m.add_function(wrap_pyfunction!(mean_episode_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(mean_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown_duration, m)?)?;
    m.add_function(wrap_pyfunction!(cdar, m)?)?;
    m.add_function(wrap_pyfunction!(ulcer_index, m)?)?;
    m.add_function(wrap_pyfunction!(pain_index, m)?)?;
    m.add_function(wrap_pyfunction!(calmar, m)?)?;
    m.add_function(wrap_pyfunction!(recovery_factor, m)?)?;
    m.add_function(wrap_pyfunction!(martin_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(sterling_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(burke_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(pain_ratio, m)?)?;
    Ok(())
}
