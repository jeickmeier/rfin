//! Python bindings for standalone benchmark-relative analytics functions.
//!
//! Thin wrappers around `finstack_analytics::benchmark` that accept and
//! return Python-native types.

use finstack_analytics::benchmark;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Tracking error between a portfolio and benchmark.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
/// annualize : bool
///     If ``True``, scale by ``sqrt(ann_factor)``.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Tracking error.
#[pyfunction]
#[pyo3(signature = (returns, benchmark_returns, annualize=true, ann_factor=252.0))]
fn tracking_error(
    returns: Vec<f64>,
    benchmark_returns: Vec<f64>,
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    benchmark::tracking_error(&returns, &benchmark_returns, annualize, ann_factor)
}

/// Information ratio: excess return / tracking error.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
/// annualize : bool
///     If ``True``, annualize.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Information ratio.
#[pyfunction]
#[pyo3(signature = (returns, benchmark_returns, annualize=true, ann_factor=252.0))]
fn information_ratio(
    returns: Vec<f64>,
    benchmark_returns: Vec<f64>,
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    benchmark::information_ratio(&returns, &benchmark_returns, annualize, ann_factor)
}

/// R-squared: correlation squared between portfolio and benchmark returns.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// float
///     R-squared value in ``[0, 1]``.
#[pyfunction]
fn r_squared(returns: Vec<f64>, benchmark_returns: Vec<f64>) -> f64 {
    benchmark::r_squared(&returns, &benchmark_returns)
}

/// Beta with confidence interval via OLS regression.
///
/// Parameters
/// ----------
/// portfolio : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// dict
///     Keys: ``beta``, ``std_err``, ``ci_lower``, ``ci_upper``.
#[pyfunction]
fn calc_beta(
    py: Python<'_>,
    portfolio: Vec<f64>,
    benchmark_returns: Vec<f64>,
) -> PyResult<Py<PyDict>> {
    let result = benchmark::calc_beta(&portfolio, &benchmark_returns);
    let dict = PyDict::new(py);
    dict.set_item("beta", result.beta)?;
    dict.set_item("std_err", result.std_err)?;
    dict.set_item("ci_lower", result.ci_lower)?;
    dict.set_item("ci_upper", result.ci_upper)?;
    Ok(dict.into())
}

/// Greeks: alpha, beta, and R-squared from OLS regression.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
/// ann_factor : float
///     Periods per year for alpha annualization.
///
/// Returns
/// -------
/// dict
///     Keys: ``alpha``, ``beta``, ``r_squared``.
#[pyfunction]
#[pyo3(signature = (returns, benchmark_returns, ann_factor=252.0))]
fn greeks(
    py: Python<'_>,
    returns: Vec<f64>,
    benchmark_returns: Vec<f64>,
    ann_factor: f64,
) -> PyResult<Py<PyDict>> {
    let result = benchmark::greeks(&returns, &benchmark_returns, ann_factor);
    let dict = PyDict::new(py);
    dict.set_item("alpha", result.alpha)?;
    dict.set_item("beta", result.beta)?;
    dict.set_item("r_squared", result.r_squared)?;
    Ok(dict.into())
}

/// Up-market capture ratio.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// float
///     Up capture ratio.
#[pyfunction]
fn up_capture(returns: Vec<f64>, benchmark_returns: Vec<f64>) -> f64 {
    benchmark::up_capture(&returns, &benchmark_returns)
}

/// Down-market capture ratio.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// float
///     Down capture ratio.
#[pyfunction]
fn down_capture(returns: Vec<f64>, benchmark_returns: Vec<f64>) -> f64 {
    benchmark::down_capture(&returns, &benchmark_returns)
}

/// Capture ratio: up capture / down capture.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// float
///     Capture ratio.
#[pyfunction]
fn capture_ratio(returns: Vec<f64>, benchmark_returns: Vec<f64>) -> f64 {
    benchmark::capture_ratio(&returns, &benchmark_returns)
}

/// Batting average: fraction of periods where portfolio beats benchmark.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Portfolio return series.
/// benchmark_returns : list[float]
///     Benchmark return series.
///
/// Returns
/// -------
/// float
///     Batting average in ``[0, 1]``.
#[pyfunction]
fn batting_average(returns: Vec<f64>, benchmark_returns: Vec<f64>) -> f64 {
    benchmark::batting_average(&returns, &benchmark_returns)
}

/// Treynor ratio: excess return per unit of systematic risk (beta).
///
/// Parameters
/// ----------
/// ann_return : float
///     Annualized portfolio return.
/// risk_free_rate : float
///     Annualized risk-free rate.
/// beta : float
///     Portfolio beta to the benchmark.
///
/// Returns
/// -------
/// float
///     Treynor ratio.
#[pyfunction]
#[pyo3(signature = (ann_return, risk_free_rate=0.0, beta=1.0))]
fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    benchmark::treynor(ann_return, risk_free_rate, beta)
}

/// M-squared (Modigliani-Modigliani) measure.
///
/// Parameters
/// ----------
/// ann_return : float
///     Annualized portfolio return.
/// ann_vol : float
///     Annualized portfolio volatility.
/// bench_vol : float
///     Annualized benchmark volatility.
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     M-squared measure.
#[pyfunction]
#[pyo3(signature = (ann_return, ann_vol, bench_vol, risk_free_rate=0.0))]
fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    benchmark::m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

/// Register standalone benchmark functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(tracking_error, m)?)?;
    m.add_function(wrap_pyfunction!(information_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(r_squared, m)?)?;
    m.add_function(wrap_pyfunction!(calc_beta, m)?)?;
    m.add_function(wrap_pyfunction!(greeks, m)?)?;
    m.add_function(wrap_pyfunction!(up_capture, m)?)?;
    m.add_function(wrap_pyfunction!(down_capture, m)?)?;
    m.add_function(wrap_pyfunction!(capture_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(batting_average, m)?)?;
    m.add_function(wrap_pyfunction!(treynor, m)?)?;
    m.add_function(wrap_pyfunction!(m_squared, m)?)?;
    Ok(vec![
        "tracking_error",
        "information_ratio",
        "r_squared",
        "calc_beta",
        "greeks",
        "up_capture",
        "down_capture",
        "capture_ratio",
        "batting_average",
        "treynor",
        "m_squared",
    ])
}
