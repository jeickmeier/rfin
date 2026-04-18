use super::super::types::{
    PyBenchmarkAlignmentPolicy, PyBetaResult, PyGreeksResult, PyMultiFactorResult, PyRollingGreeks,
};
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_analytics as fa;
use pyo3::prelude::*;

/// Align benchmark returns to target dates using an explicit missing-date policy.
#[pyfunction]
fn align_benchmark(
    bench_returns: Vec<f64>,
    bench_dates: Vec<Bound<'_, PyAny>>,
    target_dates: Vec<Bound<'_, PyAny>>,
    policy: &PyBenchmarkAlignmentPolicy,
) -> PyResult<Vec<f64>> {
    let bd: Vec<time::Date> = bench_dates
        .iter()
        .map(py_to_date)
        .collect::<PyResult<_>>()?;
    let td: Vec<time::Date> = target_dates
        .iter()
        .map(py_to_date)
        .collect::<PyResult<_>>()?;
    fa::benchmark::align_benchmark(&bench_returns, &bd, &td, policy.inner).map_err(core_to_py)
}

/// Beta regression of portfolio against benchmark.
#[pyfunction]
fn beta(portfolio: Vec<f64>, benchmark: Vec<f64>) -> PyBetaResult {
    PyBetaResult {
        inner: fa::benchmark::beta(&portfolio, &benchmark),
    }
}

/// Single-index greeks (alpha, beta, R²).
#[pyfunction]
#[pyo3(signature = (returns, benchmark, ann_factor = 252.0))]
fn greeks(returns: Vec<f64>, benchmark: Vec<f64>, ann_factor: f64) -> PyGreeksResult {
    PyGreeksResult {
        inner: fa::benchmark::greeks(&returns, &benchmark, ann_factor),
    }
}

/// Rolling greeks over a window.
#[pyfunction]
#[pyo3(signature = (returns, benchmark, dates, window = 63, ann_factor = 252.0))]
fn rolling_greeks(
    returns: Vec<f64>,
    benchmark: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: usize,
    ann_factor: f64,
) -> PyResult<PyRollingGreeks> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingGreeks {
        inner: fa::benchmark::rolling_greeks(&returns, &benchmark, &rd, window, ann_factor),
    })
}

/// Annualized tracking error.
#[pyfunction]
#[pyo3(signature = (returns, benchmark, annualize = true, ann_factor = 252.0))]
fn tracking_error(returns: Vec<f64>, benchmark: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    fa::benchmark::tracking_error(&returns, &benchmark, annualize, ann_factor)
}

/// Information ratio.
#[pyfunction]
#[pyo3(signature = (returns, benchmark, annualize = true, ann_factor = 252.0))]
fn information_ratio(
    returns: Vec<f64>,
    benchmark: Vec<f64>,
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    fa::benchmark::information_ratio(&returns, &benchmark, annualize, ann_factor)
}

/// R-squared against benchmark.
#[pyfunction]
fn r_squared(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::r_squared(&returns, &benchmark)
}

/// Up-capture ratio.
#[pyfunction]
fn up_capture(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::up_capture(&returns, &benchmark)
}

/// Down-capture ratio.
#[pyfunction]
fn down_capture(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::down_capture(&returns, &benchmark)
}

/// Capture ratio (up/down).
#[pyfunction]
fn capture_ratio(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::capture_ratio(&returns, &benchmark)
}

/// Batting average vs benchmark.
#[pyfunction]
fn batting_average(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::batting_average(&returns, &benchmark)
}

/// Multi-factor regression.
#[pyfunction]
#[pyo3(signature = (returns, factors, ann_factor = 252.0))]
fn multi_factor_greeks(
    returns: Vec<f64>,
    factors: Vec<Vec<f64>>,
    ann_factor: f64,
) -> PyResult<PyMultiFactorResult> {
    let refs: Vec<&[f64]> = factors.iter().map(|v| v.as_slice()).collect();
    fa::benchmark::multi_factor_greeks(&returns, &refs, ann_factor)
        .map(|r| PyMultiFactorResult { inner: r })
        .map_err(core_to_py)
}

/// Treynor ratio from pre-computed values.
#[pyfunction]
fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    fa::benchmark::treynor(ann_return, risk_free_rate, beta)
}

/// M-squared from pre-computed values.
#[pyfunction]
fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    fa::benchmark::m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(align_benchmark, m)?)?;
    m.add_function(wrap_pyfunction!(beta, m)?)?;
    m.add_function(wrap_pyfunction!(greeks, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_greeks, m)?)?;
    m.add_function(wrap_pyfunction!(tracking_error, m)?)?;
    m.add_function(wrap_pyfunction!(information_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(r_squared, m)?)?;
    m.add_function(wrap_pyfunction!(up_capture, m)?)?;
    m.add_function(wrap_pyfunction!(down_capture, m)?)?;
    m.add_function(wrap_pyfunction!(capture_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(batting_average, m)?)?;
    m.add_function(wrap_pyfunction!(multi_factor_greeks, m)?)?;
    m.add_function(wrap_pyfunction!(treynor, m)?)?;
    m.add_function(wrap_pyfunction!(m_squared, m)?)?;
    Ok(())
}
