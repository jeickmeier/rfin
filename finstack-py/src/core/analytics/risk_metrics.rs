//! Python bindings for standalone risk-metric functions.
//!
//! Thin wrappers around `finstack_analytics::risk_metrics` that operate on
//! plain `list[float]` slices.

use finstack_analytics::risk_metrics;
use pyo3::prelude::*;

/// Compound annual growth rate from a return series using a period-based
/// annualization factor.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// ann_factor : float
///     Periods per year (e.g. ``252.0`` for daily).
///
/// Returns
/// -------
/// float
///     CAGR.
#[pyfunction]
fn cagr_from_periods(returns: Vec<f64>, ann_factor: f64) -> f64 {
    risk_metrics::cagr_from_periods(&returns, ann_factor)
}

/// Mean return.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// annualize : bool
///     If ``True``, scale by ``ann_factor``.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Mean return (annualized if requested).
#[pyfunction]
#[pyo3(signature = (returns, annualize=true, ann_factor=252.0))]
fn mean_return(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    risk_metrics::mean_return(&returns, annualize, ann_factor)
}

/// Volatility (standard deviation of returns).
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// annualize : bool
///     If ``True``, scale by ``sqrt(ann_factor)``.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Volatility.
#[pyfunction]
#[pyo3(signature = (returns, annualize=true, ann_factor=252.0))]
fn volatility(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    risk_metrics::volatility(&returns, annualize, ann_factor)
}

/// Sharpe ratio from pre-computed annualized return and volatility.
///
/// Parameters
/// ----------
/// ann_return : float
///     Annualized return.
/// ann_vol : float
///     Annualized volatility.
/// risk_free_rate : float
///     Annualized risk-free rate.
///
/// Returns
/// -------
/// float
///     Sharpe ratio.
#[pyfunction]
#[pyo3(signature = (ann_return, ann_vol, risk_free_rate=0.0))]
fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Sortino ratio (downside-risk-adjusted return).
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// annualize : bool
///     If ``True``, annualize both numerator and denominator.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Sortino ratio.
#[pyfunction]
#[pyo3(signature = (returns, annualize=true, ann_factor=252.0))]
fn sortino(returns: Vec<f64>, annualize: bool, ann_factor: f64) -> f64 {
    risk_metrics::sortino(&returns, annualize, ann_factor)
}

/// Downside deviation below a minimum acceptable return.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// mar : float
///     Minimum acceptable return threshold.
/// annualize : bool
///     If ``True``, scale by ``sqrt(ann_factor)``.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Downside deviation.
#[pyfunction]
#[pyo3(signature = (returns, mar=0.0, annualize=false, ann_factor=252.0))]
fn downside_deviation(returns: Vec<f64>, mar: f64, annualize: bool, ann_factor: f64) -> f64 {
    risk_metrics::downside_deviation(&returns, mar, annualize, ann_factor)
}

/// Historical Value-at-Risk.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// confidence : float
///     Confidence level in ``(0, 1)``, e.g. ``0.95``.
/// ann_factor : float, optional
///     If provided, annualize the VaR.
///
/// Returns
/// -------
/// float
///     VaR as a positive loss.
#[pyfunction]
#[pyo3(signature = (returns, confidence=0.95, ann_factor=None))]
fn value_at_risk(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    risk_metrics::value_at_risk(&returns, confidence, ann_factor)
}

/// Expected shortfall (CVaR).
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// confidence : float
///     Confidence level in ``(0, 1)``.
/// ann_factor : float, optional
///     If provided, annualize.
///
/// Returns
/// -------
/// float
///     Expected shortfall.
#[pyfunction]
#[pyo3(signature = (returns, confidence=0.95, ann_factor=None))]
fn expected_shortfall(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    risk_metrics::expected_shortfall(&returns, confidence, ann_factor)
}

/// Parametric (Gaussian) VaR.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// confidence : float
///     Confidence level.
/// ann_factor : float, optional
///     If provided, annualize using a finite, strictly positive periods-per-year
///     factor.
///
/// Returns
/// -------
/// float
///     Parametric VaR. Returns ``NaN`` when ``ann_factor`` is zero, negative,
///     or non-finite.
#[pyfunction]
#[pyo3(signature = (returns, confidence=0.95, ann_factor=None))]
fn parametric_var(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    risk_metrics::parametric_var(&returns, confidence, ann_factor)
}

/// Cornish-Fisher adjusted VaR.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// confidence : float
///     Confidence level.
/// ann_factor : float, optional
///     If provided, annualize using a finite, strictly positive periods-per-year
///     factor.
///
/// Returns
/// -------
/// float
///     Cornish-Fisher VaR. Returns ``NaN`` when ``ann_factor`` is zero,
///     negative, or non-finite.
#[pyfunction]
#[pyo3(signature = (returns, confidence=0.95, ann_factor=None))]
fn cornish_fisher_var(returns: Vec<f64>, confidence: f64, ann_factor: Option<f64>) -> f64 {
    risk_metrics::cornish_fisher_var(&returns, confidence, ann_factor)
}

/// Skewness of a return series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Sample skewness.
#[pyfunction]
fn skewness(returns: Vec<f64>) -> f64 {
    risk_metrics::skewness(&returns)
}

/// Excess kurtosis of a return series.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Excess kurtosis.
#[pyfunction]
fn kurtosis(returns: Vec<f64>) -> f64 {
    risk_metrics::kurtosis(&returns)
}

/// Geometric mean return per period.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Geometric mean.
#[pyfunction]
fn geometric_mean(returns: Vec<f64>) -> f64 {
    risk_metrics::geometric_mean(&returns)
}

/// Omega ratio: probability-weighted gain-to-loss ratio.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// threshold : float
///     Return threshold (typically ``0.0``).
///
/// Returns
/// -------
/// float
///     Omega ratio.
#[pyfunction]
#[pyo3(signature = (returns, threshold=0.0))]
fn omega_ratio(returns: Vec<f64>, threshold: f64) -> f64 {
    risk_metrics::omega_ratio(&returns, threshold)
}

/// Gain-to-pain ratio: sum of returns / sum of |negative returns|.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
///
/// Returns
/// -------
/// float
///     Gain-to-pain ratio.
#[pyfunction]
fn gain_to_pain(returns: Vec<f64>) -> f64 {
    risk_metrics::gain_to_pain(&returns)
}

/// Tail ratio: upper quantile / |lower quantile|.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// confidence : float
///     Quantile level (e.g. ``0.95``).
///
/// Returns
/// -------
/// float
///     Tail ratio.
#[pyfunction]
#[pyo3(signature = (returns, confidence=0.95))]
fn tail_ratio(returns: Vec<f64>, confidence: f64) -> f64 {
    risk_metrics::tail_ratio(&returns, confidence)
}

/// Modified Sharpe ratio using Cornish-Fisher VaR as the risk measure.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Simple period returns.
/// risk_free_rate : float
///     Annualized risk-free rate.
/// confidence : float
///     VaR confidence level.
/// ann_factor : float
///     Periods per year.
///
/// Returns
/// -------
/// float
///     Modified Sharpe ratio.
#[pyfunction]
#[pyo3(signature = (returns, risk_free_rate=0.0, confidence=0.95, ann_factor=252.0))]
fn modified_sharpe(
    returns: Vec<f64>,
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    risk_metrics::modified_sharpe(&returns, risk_free_rate, confidence, ann_factor)
}

/// Register standalone risk-metric functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(cagr_from_periods, m)?)?;
    m.add_function(wrap_pyfunction!(mean_return, m)?)?;
    m.add_function(wrap_pyfunction!(volatility, m)?)?;
    m.add_function(wrap_pyfunction!(sharpe, m)?)?;
    m.add_function(wrap_pyfunction!(sortino, m)?)?;
    m.add_function(wrap_pyfunction!(downside_deviation, m)?)?;
    m.add_function(wrap_pyfunction!(value_at_risk, m)?)?;
    m.add_function(wrap_pyfunction!(expected_shortfall, m)?)?;
    m.add_function(wrap_pyfunction!(parametric_var, m)?)?;
    m.add_function(wrap_pyfunction!(cornish_fisher_var, m)?)?;
    m.add_function(wrap_pyfunction!(skewness, m)?)?;
    m.add_function(wrap_pyfunction!(kurtosis, m)?)?;
    m.add_function(wrap_pyfunction!(geometric_mean, m)?)?;
    m.add_function(wrap_pyfunction!(omega_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(gain_to_pain, m)?)?;
    m.add_function(wrap_pyfunction!(tail_ratio, m)?)?;
    m.add_function(wrap_pyfunction!(modified_sharpe, m)?)?;
    Ok(vec![
        "cagr_from_periods",
        "mean_return",
        "volatility",
        "sharpe",
        "sortino",
        "downside_deviation",
        "value_at_risk",
        "expected_shortfall",
        "parametric_var",
        "cornish_fisher_var",
        "skewness",
        "kurtosis",
        "geometric_mean",
        "omega_ratio",
        "gain_to_pain",
        "tail_ratio",
        "modified_sharpe",
    ])
}
