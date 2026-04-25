//! Python bindings for benchmark-relative analytics: alignment, tracking
//! error, capture ratios, OLS regressions, and multi-factor greeks.
//!
//! Inputs are simple decimal returns (`0.01` = 1%). The annualization
//! factor is the number of periods per year (e.g. ``252`` for daily).

use super::super::types::{
    PyBenchmarkAlignmentPolicy, PyBetaResult, PyGreeksResult, PyMultiFactorResult, PyRollingGreeks,
};
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_analytics as fa;
use pyo3::prelude::*;

/// Align benchmark returns to a target date grid.
///
/// Args:
///     bench_returns: Raw benchmark returns aligned with ``bench_dates``.
///     bench_dates: Dates corresponding to ``bench_returns``.
///     target_dates: Dates the caller wants the output aligned to.
///     policy: How to handle target dates that are missing from
///         ``bench_dates``. Use :class:`BenchmarkAlignmentPolicy.zero_on_missing`
///         to fill gaps with ``0.0`` or
///         :class:`BenchmarkAlignmentPolicy.error_on_missing` to raise.
///
/// Returns:
///     A list of benchmark returns aligned 1:1 with ``target_dates``.
///
/// Raises:
///     ValueError: When the alignment policy is ``error_on_missing`` and
///         ``target_dates`` contains a date not present in ``bench_dates``.
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

/// OLS beta of a portfolio against a benchmark, with inferential stats.
///
/// Args:
///     portfolio: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``portfolio``.
///
/// Returns:
///     :class:`BetaResult` with the point estimate, standard error, and a
///     95% confidence interval.
#[pyfunction]
fn beta(portfolio: Vec<f64>, benchmark: Vec<f64>) -> PyBetaResult {
    PyBetaResult {
        inner: fa::benchmark::beta(&portfolio, &benchmark),
    }
}

/// Single-index greeks: annualized alpha, beta, R², and adjusted R².
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///     ann_factor: Periods per year used to annualize alpha
///         (e.g. ``252``). Defaults to ``252.0``.
///
/// Returns:
///     :class:`GreeksResult` with annualized alpha, beta, R², adjusted R².
#[pyfunction]
#[pyo3(signature = (returns, benchmark, ann_factor = 252.0))]
fn greeks(returns: Vec<f64>, benchmark: Vec<f64>, ann_factor: f64) -> PyGreeksResult {
    PyGreeksResult {
        inner: fa::benchmark::greeks(&returns, &benchmark, ann_factor),
    }
}

/// Rolling greeks (alpha, beta) over a sliding window.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///     dates: Dates aligned with ``returns``.
///     window: Look-back window length in periods. Defaults to ``63``.
///     ann_factor: Periods per year for annualization. Defaults to ``252.0``.
///
/// Returns:
///     :class:`RollingGreeks` with parallel ``dates``, ``alphas``, and
///     ``betas`` arrays. Each output value is right-labeled by the last
///     date in its window. Output length is ``len(returns) - window + 1``.
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

/// Tracking error: standard deviation of active returns.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///     annualize: If ``True``, scale by ``sqrt(ann_factor)``. Default ``True``.
///     ann_factor: Periods per year (e.g. ``252``). Default ``252.0``.
///
/// Returns:
///     A non-negative scalar. ``0.0`` if active returns are all equal.
#[pyfunction]
#[pyo3(signature = (returns, benchmark, annualize = true, ann_factor = 252.0))]
fn tracking_error(returns: Vec<f64>, benchmark: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    fa::benchmark::tracking_error(&returns, &benchmark, annualize, ann_factor)
}

/// Information ratio: mean active return divided by tracking error.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///     annualize: If ``True``, the numerator and denominator are annualized
///         consistently. Default ``True``.
///     ann_factor: Periods per year. Default ``252.0``.
///
/// Returns:
///     A scalar information ratio. ``+inf`` (or ``-inf``) when tracking
///     error is zero and the mean active return is non-zero.
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

/// Coefficient of determination of an OLS regression of ``returns`` on
/// ``benchmark``.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///
/// Returns:
///     R² in ``[0.0, 1.0]``.
#[pyfunction]
fn r_squared(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::r_squared(&returns, &benchmark)
}

/// Up-market capture ratio.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///
/// Returns:
///     Compounded return of the portfolio in benchmark-up periods divided
///     by the same for the benchmark. Values above ``1.0`` mean the
///     portfolio outperforms the benchmark in up periods.
#[pyfunction]
fn up_capture(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::up_capture(&returns, &benchmark)
}

/// Down-market capture ratio.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///
/// Returns:
///     Compounded return of the portfolio in benchmark-down periods divided
///     by the same for the benchmark. Values below ``1.0`` mean the
///     portfolio loses less than the benchmark in down periods.
#[pyfunction]
fn down_capture(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::down_capture(&returns, &benchmark)
}

/// Capture ratio: ``up_capture / down_capture``.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///
/// Returns:
///     A scalar. Above ``1.0`` indicates favorable capture asymmetry.
#[pyfunction]
fn capture_ratio(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::capture_ratio(&returns, &benchmark)
}

/// Batting average: fraction of periods where the portfolio outperforms.
///
/// Args:
///     returns: Portfolio returns.
///     benchmark: Benchmark returns aligned with ``returns``.
///
/// Returns:
///     Fraction in ``[0.0, 1.0]``.
#[pyfunction]
fn batting_average(returns: Vec<f64>, benchmark: Vec<f64>) -> f64 {
    fa::benchmark::batting_average(&returns, &benchmark)
}

/// Multi-factor OLS regression of ``returns`` on ``factors``.
///
/// Args:
///     returns: Portfolio returns.
///     factors: List of factor return series, each aligned with
///         ``returns``. All factors must have the same length as
///         ``returns``.
///     ann_factor: Periods per year for annualizing alpha and residual
///         volatility. Default ``252.0``.
///
/// Returns:
///     :class:`MultiFactorResult` with annualized alpha, factor loadings,
///     R², adjusted R², and annualized residual volatility.
///
/// Raises:
///     ValueError: When factor lengths are mismatched, factor values are
///         non-finite, ``ann_factor`` is non-positive, or the design
///         matrix is singular or near-singular.
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

/// Treynor ratio: ``(ann_return - risk_free_rate) / beta``.
///
/// Args:
///     ann_return: Annualized portfolio return (decimal).
///     risk_free_rate: Annualized risk-free rate (decimal).
///     beta: Portfolio beta versus the benchmark.
///
/// Returns:
///     Treynor ratio as a scalar. ``±inf`` when ``beta`` is zero.
#[pyfunction]
fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    fa::benchmark::treynor(ann_return, risk_free_rate, beta)
}

/// Modigliani-Modigliani (M²) measure: leverage-adjusted excess return.
///
/// Args:
///     ann_return: Annualized portfolio return (decimal).
///     ann_vol: Annualized portfolio volatility (decimal).
///     bench_vol: Annualized benchmark volatility (decimal).
///     risk_free_rate: Annualized risk-free rate (decimal).
///
/// Returns:
///     M² as a scalar return value, comparable to the benchmark on equal
///     volatility footing.
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
