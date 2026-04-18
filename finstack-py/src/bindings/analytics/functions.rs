//! Standalone analytics functions mirroring the `finstack_analytics` crate root.

use super::performance::parse_freq;
use super::types::*;
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_analytics as fa;
use pyo3::prelude::*;

// ===================================================================
// Aggregation
// ===================================================================

/// Group returns by period and return ``(period_id_str, compounded_return)`` pairs.
#[pyfunction]
#[pyo3(signature = (dates, returns, freq = "monthly"))]
fn group_by_period(
    dates: Vec<Bound<'_, PyAny>>,
    returns: Vec<f64>,
    freq: &str,
) -> PyResult<Vec<(String, f64)>> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    let pk = parse_freq(freq)?;
    let grouped = fa::aggregation::group_by_period(&rd, &returns, pk, None);
    Ok(grouped
        .iter()
        .map(|(pid, r)| (format!("{pid}"), *r))
        .collect())
}

/// Compute period statistics from a list of periodic return values.
///
/// Accepts a flat list of returns (e.g. monthly returns). The PeriodId
/// labels are synthetic — only the return values matter for statistics.
#[pyfunction]
fn period_stats(returns: Vec<f64>) -> PyPeriodStats {
    let tuples: Vec<(finstack_core::dates::PeriodId, f64)> = returns
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let pid = finstack_core::dates::PeriodId::month(2000, (i as u8 % 12) + 1);
            (pid, r)
        })
        .collect();
    PyPeriodStats {
        inner: fa::aggregation::period_stats(&tuples),
    }
}

// ===================================================================
// Benchmark
// ===================================================================

/// Align benchmark returns to target dates using zero-fill for missing.
#[pyfunction]
fn align_benchmark(
    bench_returns: Vec<f64>,
    bench_dates: Vec<Bound<'_, PyAny>>,
    target_dates: Vec<Bound<'_, PyAny>>,
) -> PyResult<Vec<f64>> {
    let bd: Vec<time::Date> = bench_dates
        .iter()
        .map(py_to_date)
        .collect::<PyResult<_>>()?;
    let td: Vec<time::Date> = target_dates
        .iter()
        .map(py_to_date)
        .collect::<PyResult<_>>()?;
    Ok(fa::benchmark::align_benchmark(&bench_returns, &bd, &td))
}

/// Align benchmark returns with a specific missing-date policy.
#[pyfunction]
fn align_benchmark_with_policy(
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
    fa::benchmark::align_benchmark_with_policy(&bench_returns, &bd, &td, policy.inner)
        .map_err(core_to_py)
}

/// Beta regression of portfolio against benchmark.
#[pyfunction]
fn calc_beta(portfolio: Vec<f64>, benchmark: Vec<f64>) -> PyBetaResult {
    PyBetaResult {
        inner: fa::benchmark::calc_beta(&portfolio, &benchmark),
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

/// M-squared from return series.
#[pyfunction]
#[pyo3(signature = (portfolio, benchmark, ann_factor = 252.0, risk_free_rate = 0.0))]
fn m_squared_from_returns(
    portfolio: Vec<f64>,
    benchmark: Vec<f64>,
    ann_factor: f64,
    risk_free_rate: f64,
) -> f64 {
    fa::benchmark::m_squared_from_returns(&portfolio, &benchmark, ann_factor, risk_free_rate)
}

// ===================================================================
// Consecutive
// ===================================================================

/// Count longest consecutive run of positive values.
#[pyfunction]
fn count_consecutive(values: Vec<f64>) -> usize {
    fa::consecutive::count_consecutive(&values, |x| x > 0.0)
}

// ===================================================================
// Drawdown
// ===================================================================

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
fn avg_drawdown(drawdown: Vec<f64>, n: usize) -> f64 {
    fa::drawdown::avg_drawdown(&drawdown, n)
}

/// Simple arithmetic average of drawdown values.
#[pyfunction]
fn average_drawdown(drawdowns: Vec<f64>) -> f64 {
    fa::drawdown::average_drawdown(&drawdowns)
}

/// Maximum drawdown from a drawdown series (already computed from returns).
#[pyfunction]
fn max_drawdown(drawdown: Vec<f64>) -> f64 {
    fa::drawdown::max_drawdown(&drawdown)
}

/// Maximum drawdown computed directly from returns.
#[pyfunction]
fn max_drawdown_from_returns(returns: Vec<f64>) -> f64 {
    fa::drawdown::max_drawdown_from_returns(&returns)
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

/// Calmar ratio from returns.
#[pyfunction]
#[pyo3(signature = (returns, ann_factor = 252.0))]
fn calmar_from_returns(returns: Vec<f64>, ann_factor: f64) -> f64 {
    fa::drawdown::calmar_from_returns(&returns, ann_factor)
}

/// Recovery factor from pre-computed values.
#[pyfunction]
fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    fa::drawdown::recovery_factor(total_return, max_dd)
}

/// Recovery factor from returns.
#[pyfunction]
fn recovery_factor_from_returns(returns: Vec<f64>) -> f64 {
    fa::drawdown::recovery_factor_from_returns(&returns)
}

/// Martin ratio from pre-computed values.
#[pyfunction]
fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    fa::drawdown::martin_ratio(cagr_val, ulcer)
}

/// Martin ratio from returns.
#[pyfunction]
#[pyo3(signature = (returns, ann_factor = 252.0))]
fn martin_ratio_from_returns(returns: Vec<f64>, ann_factor: f64) -> f64 {
    fa::drawdown::martin_ratio_from_returns(&returns, ann_factor)
}

/// Sterling ratio from pre-computed values.
#[pyfunction]
#[pyo3(signature = (cagr_val, avg_dd, risk_free_rate = 0.0))]
fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Sterling ratio from returns.
#[pyfunction]
#[pyo3(signature = (returns, ann_factor = 252.0, risk_free_rate = 0.0))]
fn sterling_ratio_from_returns(returns: Vec<f64>, ann_factor: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::sterling_ratio_from_returns(&returns, ann_factor, risk_free_rate)
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

/// Pain ratio from returns.
#[pyfunction]
#[pyo3(signature = (returns, ann_factor = 252.0, risk_free_rate = 0.0))]
fn pain_ratio_from_returns(returns: Vec<f64>, ann_factor: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::pain_ratio_from_returns(&returns, ann_factor, risk_free_rate)
}

// ===================================================================
// Returns
// ===================================================================

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

// ===================================================================
// Risk metrics — return-based
// ===================================================================

/// CAGR between two dates.
#[pyfunction]
fn cagr(returns: Vec<f64>, start: Bound<'_, PyAny>, end: Bound<'_, PyAny>) -> PyResult<f64> {
    let s = py_to_date(&start)?;
    let e = py_to_date(&end)?;
    Ok(fa::risk_metrics::cagr(&returns, s, e))
}

/// CAGR from an annualization factor.
#[pyfunction]
fn cagr_from_periods(returns: Vec<f64>, ann_factor: f64) -> f64 {
    fa::risk_metrics::cagr_from_periods(&returns, ann_factor)
}

/// Arithmetic mean return.
#[pyfunction]
#[pyo3(signature = (returns, annualize = false, ann_factor = 1.0))]
fn mean_return(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    fa::risk_metrics::mean_return(&returns, annualize, ann_factor)
}

/// Volatility (standard deviation of returns).
#[pyfunction]
#[pyo3(signature = (returns, annualize = true, ann_factor = 252.0))]
fn volatility(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    fa::risk_metrics::volatility(&returns, annualize, ann_factor)
}

/// Sharpe ratio from pre-computed annualized return and vol.
#[pyfunction]
#[pyo3(signature = (ann_return, ann_vol, risk_free_rate = 0.0))]
fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    fa::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Downside deviation.
#[pyfunction]
#[pyo3(signature = (returns, mar = 0.0, annualize = true, ann_factor = 252.0))]
fn downside_deviation(returns: Vec<f64>, mar: f64, annualize: bool, ann_factor: f64) -> f64 {
    fa::risk_metrics::downside_deviation(&returns, mar, annualize, ann_factor)
}

/// Sortino ratio.
#[pyfunction]
#[pyo3(signature = (returns, annualize = true, ann_factor = 252.0))]
fn sortino(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    fa::risk_metrics::sortino(&returns, annualize, ann_factor)
}

/// Geometric mean of returns.
#[pyfunction]
fn geometric_mean(returns: Vec<f64>) -> f64 {
    fa::risk_metrics::geometric_mean(&returns)
}

/// Omega ratio.
#[pyfunction]
#[pyo3(signature = (returns, threshold = 0.0))]
fn omega_ratio(returns: Vec<f64>, threshold: f64) -> f64 {
    fa::risk_metrics::omega_ratio(&returns, threshold)
}

/// Gain-to-pain ratio.
#[pyfunction]
fn gain_to_pain(returns: Vec<f64>) -> f64 {
    fa::risk_metrics::gain_to_pain(&returns)
}

/// Modified Sharpe ratio.
#[pyfunction]
#[pyo3(signature = (returns, risk_free_rate = 0.0, confidence = 0.95, ann_factor = 252.0))]
fn modified_sharpe(
    returns: Vec<f64>,
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    fa::risk_metrics::modified_sharpe(&returns, risk_free_rate, confidence, ann_factor)
}

/// Monte Carlo ruin probability estimation.
#[pyfunction]
fn estimate_ruin(
    returns: Vec<f64>,
    definition: &PyRuinDefinition,
    model: &PyRuinModel,
) -> PyRuinEstimate {
    PyRuinEstimate {
        inner: fa::risk_metrics::estimate_ruin(&returns, definition.inner, &model.inner),
    }
}

// ===================================================================
// Risk metrics — rolling
// ===================================================================

/// Rolling Sharpe ratio with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = 63, ann_factor = 252.0, risk_free_rate = 0.0))]
fn rolling_sharpe(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> PyResult<PyRollingSharpe> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingSharpe {
        inner: fa::risk_metrics::rolling_sharpe(&returns, &rd, window, ann_factor, risk_free_rate),
    })
}

/// Rolling Sortino ratio with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = 63, ann_factor = 252.0))]
fn rolling_sortino(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: usize,
    ann_factor: f64,
) -> PyResult<PyRollingSortino> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingSortino {
        inner: fa::risk_metrics::rolling_sortino(&returns, &rd, window, ann_factor),
    })
}

/// Rolling volatility with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = 63, ann_factor = 252.0))]
fn rolling_volatility(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: usize,
    ann_factor: f64,
) -> PyResult<PyRollingVolatility> {
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingVolatility {
        inner: fa::risk_metrics::rolling_volatility(&returns, &rd, window, ann_factor),
    })
}

/// Rolling Sharpe values only (no dates).
#[pyfunction]
#[pyo3(signature = (returns, window = 63, ann_factor = 252.0, risk_free_rate = 0.0))]
fn rolling_sharpe_values(
    returns: Vec<f64>,
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Vec<f64> {
    fa::risk_metrics::rolling_sharpe_values(&returns, window, ann_factor, risk_free_rate)
}

/// Rolling Sortino values only (no dates).
#[pyfunction]
#[pyo3(signature = (returns, window = 63, ann_factor = 252.0))]
fn rolling_sortino_values(returns: Vec<f64>, window: usize, ann_factor: f64) -> Vec<f64> {
    fa::risk_metrics::rolling_sortino_values(&returns, window, ann_factor)
}

/// Rolling volatility values only (no dates).
#[pyfunction]
#[pyo3(signature = (returns, window = 63, ann_factor = 252.0))]
fn rolling_volatility_values(returns: Vec<f64>, window: usize, ann_factor: f64) -> Vec<f64> {
    fa::risk_metrics::rolling_volatility_values(&returns, window, ann_factor)
}

// ===================================================================
// Risk metrics — tail risk
// ===================================================================

/// Historical Value-at-Risk.
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95, ann_factor = None))]
fn value_at_risk(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    fa::risk_metrics::value_at_risk(&returns, confidence, ann_factor)
}

/// Expected Shortfall (CVaR).
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95, ann_factor = None))]
fn expected_shortfall(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    fa::risk_metrics::expected_shortfall(&returns, confidence, ann_factor)
}

/// Parametric VaR (Gaussian assumption).
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95, ann_factor = None))]
fn parametric_var(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    fa::risk_metrics::parametric_var(&returns, confidence, ann_factor)
}

/// Cornish-Fisher VaR (skewness/kurtosis adjusted).
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95, ann_factor = None))]
fn cornish_fisher_var(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    fa::risk_metrics::cornish_fisher_var(&returns, confidence, ann_factor)
}

/// Skewness of returns.
#[pyfunction]
fn skewness(returns: Vec<f64>) -> f64 {
    fa::risk_metrics::skewness(&returns)
}

/// Excess kurtosis of returns.
#[pyfunction]
fn kurtosis(returns: Vec<f64>) -> f64 {
    fa::risk_metrics::kurtosis(&returns)
}

/// Tail ratio (upper quantile / |lower quantile|).
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95))]
fn tail_ratio(returns: Vec<f64>, confidence: f64) -> f64 {
    fa::risk_metrics::tail_ratio(&returns, confidence)
}

/// Outlier win ratio.
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95))]
fn outlier_win_ratio(returns: Vec<f64>, confidence: f64) -> f64 {
    fa::risk_metrics::outlier_win_ratio(&returns, confidence)
}

/// Outlier loss ratio.
#[pyfunction]
#[pyo3(signature = (returns, confidence = 0.95))]
fn outlier_loss_ratio(returns: Vec<f64>, confidence: f64) -> f64 {
    fa::risk_metrics::outlier_loss_ratio(&returns, confidence)
}

// ===================================================================
// Registration
// ===================================================================

#[allow(clippy::too_many_lines)]
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Aggregation
    m.add_function(wrap_pyfunction!(group_by_period, m)?)?;
    m.add_function(wrap_pyfunction!(period_stats, m)?)?;
    // Benchmark
    m.add_function(wrap_pyfunction!(align_benchmark, m)?)?;
    m.add_function(wrap_pyfunction!(align_benchmark_with_policy, m)?)?;
    m.add_function(wrap_pyfunction!(calc_beta, m)?)?;
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
    m.add_function(wrap_pyfunction!(m_squared_from_returns, m)?)?;
    // Consecutive
    m.add_function(wrap_pyfunction!(count_consecutive, m)?)?;
    // Drawdown
    m.add_function(wrap_pyfunction!(to_drawdown_series, m)?)?;
    m.add_function(wrap_pyfunction!(drawdown_details, m)?)?;
    m.add_function(wrap_pyfunction!(avg_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(average_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(max_drawdown_duration, m)?)?;
    m.add_function(wrap_pyfunction!(cdar, m)?)?;
    m.add_function(wrap_pyfunction!(ulcer_index, m)?)?;
    m.add_function(wrap_pyfunction!(pain_index, m)?)?;
    m.add_function(wrap_pyfunction!(calmar, m)?)?;
    m.add_function(wrap_pyfunction!(calmar_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(recovery_factor, m)?)?;
    m.add_function(wrap_pyfunction!(recovery_factor_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(martin_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(martin_ratio_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(sterling_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(sterling_ratio_from_returns, m)?)?;
    m.add_function(wrap_pyfunction!(burke_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(pain_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(pain_ratio_from_returns, m)?)?;
    // Returns
    m.add_function(wrap_pyfunction!(simple_returns, m)?)?;
    m.add_function(wrap_pyfunction!(clean_returns, m)?)?;
    m.add_function(wrap_pyfunction!(excess_returns, m)?)?;
    m.add_function(wrap_pyfunction!(convert_to_prices, m)?)?;
    m.add_function(wrap_pyfunction!(rebase, m)?)?;
    m.add_function(wrap_pyfunction!(comp_sum, m)?)?;
    m.add_function(wrap_pyfunction!(comp_total, m)?)?;
    // Risk metrics — return-based
    m.add_function(wrap_pyfunction!(cagr, m)?)?;
    m.add_function(wrap_pyfunction!(cagr_from_periods, m)?)?;
    m.add_function(wrap_pyfunction!(mean_return, m)?)?;
    m.add_function(wrap_pyfunction!(volatility, m)?)?;
    m.add_function(wrap_pyfunction!(sharpe, m)?)?;
    m.add_function(wrap_pyfunction!(downside_deviation, m)?)?;
    m.add_function(wrap_pyfunction!(sortino, m)?)?;
    m.add_function(wrap_pyfunction!(geometric_mean, m)?)?;
    m.add_function(wrap_pyfunction!(omega_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(gain_to_pain, m)?)?;
    m.add_function(wrap_pyfunction!(modified_sharpe, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_ruin, m)?)?;
    // Risk metrics — rolling
    m.add_function(wrap_pyfunction!(rolling_sharpe, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_sortino, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_volatility, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_sharpe_values, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_sortino_values, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_volatility_values, m)?)?;
    // Risk metrics — tail
    m.add_function(wrap_pyfunction!(value_at_risk, m)?)?;
    m.add_function(wrap_pyfunction!(expected_shortfall, m)?)?;
    m.add_function(wrap_pyfunction!(parametric_var, m)?)?;
    m.add_function(wrap_pyfunction!(cornish_fisher_var, m)?)?;
    m.add_function(wrap_pyfunction!(skewness, m)?)?;
    m.add_function(wrap_pyfunction!(kurtosis, m)?)?;
    m.add_function(wrap_pyfunction!(tail_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(outlier_win_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(outlier_loss_ratio, m)?)?;
    Ok(())
}
