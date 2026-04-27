use super::super::types::{
    PyCagrBasis, PyRollingSharpe, PyRollingSortino, PyRollingVolatility, PyRuinDefinition,
    PyRuinEstimate, PyRuinModel,
};
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_analytics as fa;
use finstack_analytics::registry::{embedded_defaults, RiskMetricPythonDefaults};
use pyo3::prelude::*;

fn py_risk_defaults() -> PyResult<&'static RiskMetricPythonDefaults> {
    embedded_defaults()
        .map(|defaults| &defaults.python_bindings)
        .map_err(core_to_py)
}

/// CAGR using a supplied annualization basis.
#[pyfunction]
fn cagr(returns: Vec<f64>, basis: &PyCagrBasis) -> f64 {
    fa::risk_metrics::cagr(&returns, basis.inner)
}

/// Arithmetic mean return.
#[pyfunction]
#[pyo3(signature = (returns, annualize = None, ann_factor = None))]
fn mean_return(
    returns: Vec<f64>,
    annualize: Option<bool>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let defaults = &py_risk_defaults()?.mean_return;
    Ok(fa::risk_metrics::mean_return(
        &returns,
        annualize.unwrap_or(defaults.annualize),
        ann_factor.unwrap_or(defaults.ann_factor),
    ))
}

/// Volatility (standard deviation of returns).
#[pyfunction]
#[pyo3(signature = (returns, annualize = None, ann_factor = None))]
fn volatility(
    returns: Vec<f64>,
    annualize: Option<bool>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let defaults = &py_risk_defaults()?.volatility;
    Ok(fa::risk_metrics::volatility(
        &returns,
        annualize.unwrap_or(defaults.annualize),
        ann_factor.unwrap_or(defaults.ann_factor),
    ))
}

/// Sharpe ratio from pre-computed annualized return and vol.
#[pyfunction]
#[pyo3(signature = (ann_return, ann_vol, risk_free_rate = 0.0))]
fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    fa::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Downside deviation.
#[pyfunction]
#[pyo3(signature = (returns, mar = None, annualize = None, ann_factor = None))]
fn downside_deviation(
    returns: Vec<f64>,
    mar: Option<f64>,
    annualize: Option<bool>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let defaults = &py_risk_defaults()?.downside_deviation;
    Ok(fa::risk_metrics::downside_deviation(
        &returns,
        mar.unwrap_or(defaults.mar),
        annualize.unwrap_or(defaults.annualize),
        ann_factor.unwrap_or(defaults.ann_factor),
    ))
}

/// Sortino ratio.
#[pyfunction]
#[pyo3(signature = (returns, annualize = None, ann_factor = None, mar = None))]
fn sortino(
    returns: Vec<f64>,
    annualize: Option<bool>,
    ann_factor: Option<f64>,
    mar: Option<f64>,
) -> PyResult<f64> {
    let defaults = &py_risk_defaults()?.sortino;
    Ok(fa::risk_metrics::sortino(
        &returns,
        annualize.unwrap_or(defaults.annualize),
        ann_factor.unwrap_or(defaults.ann_factor),
        mar.unwrap_or(defaults.mar),
    ))
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
#[pyo3(signature = (returns, risk_free_rate = None, confidence = None, ann_factor = None))]
fn modified_sharpe(
    returns: Vec<f64>,
    risk_free_rate: Option<f64>,
    confidence: Option<f64>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let defaults = &py_risk_defaults()?.modified_sharpe;
    Ok(fa::risk_metrics::modified_sharpe(
        &returns,
        risk_free_rate.unwrap_or(defaults.risk_free_rate),
        confidence.unwrap_or(defaults.confidence),
        ann_factor.unwrap_or(defaults.ann_factor),
    ))
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

/// Rolling Sharpe ratio with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = None, ann_factor = None, risk_free_rate = None))]
fn rolling_sharpe(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: Option<usize>,
    ann_factor: Option<f64>,
    risk_free_rate: Option<f64>,
) -> PyResult<PyRollingSharpe> {
    let defaults = &py_risk_defaults()?.rolling;
    let window = window.unwrap_or(defaults.window);
    let ann_factor = ann_factor.unwrap_or(defaults.ann_factor);
    let risk_free_rate = risk_free_rate.unwrap_or(defaults.risk_free_rate);
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingSharpe {
        inner: fa::risk_metrics::rolling_sharpe(&returns, &rd, window, ann_factor, risk_free_rate),
    })
}

/// Rolling Sortino ratio with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = None, ann_factor = None))]
fn rolling_sortino(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: Option<usize>,
    ann_factor: Option<f64>,
) -> PyResult<PyRollingSortino> {
    let defaults = &py_risk_defaults()?.rolling;
    let window = window.unwrap_or(defaults.window);
    let ann_factor = ann_factor.unwrap_or(defaults.ann_factor);
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingSortino {
        inner: fa::risk_metrics::rolling_sortino(&returns, &rd, window, ann_factor),
    })
}

/// Rolling volatility with date labels.
#[pyfunction]
#[pyo3(signature = (returns, dates, window = None, ann_factor = None))]
fn rolling_volatility(
    returns: Vec<f64>,
    dates: Vec<Bound<'_, PyAny>>,
    window: Option<usize>,
    ann_factor: Option<f64>,
) -> PyResult<PyRollingVolatility> {
    let defaults = &py_risk_defaults()?.rolling;
    let window = window.unwrap_or(defaults.window);
    let ann_factor = ann_factor.unwrap_or(defaults.ann_factor);
    let rd: Vec<time::Date> = dates.iter().map(py_to_date).collect::<PyResult<_>>()?;
    Ok(PyRollingVolatility {
        inner: fa::risk_metrics::rolling_volatility(&returns, &rd, window, ann_factor),
    })
}

/// Historical Value-at-Risk.
#[pyfunction]
#[pyo3(signature = (returns, confidence = None))]
fn value_at_risk(returns: Vec<f64>, confidence: Option<f64>) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::value_at_risk(&returns, confidence))
}

/// Expected Shortfall (CVaR).
#[pyfunction]
#[pyo3(signature = (returns, confidence = None))]
fn expected_shortfall(returns: Vec<f64>, confidence: Option<f64>) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::expected_shortfall(&returns, confidence))
}

/// Parametric VaR (Gaussian assumption).
#[pyfunction]
#[pyo3(signature = (returns, confidence = None, ann_factor = None))]
fn parametric_var(
    returns: Vec<f64>,
    confidence: Option<f64>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::parametric_var(
        &returns, confidence, ann_factor,
    ))
}

/// Cornish-Fisher VaR (skewness/kurtosis adjusted).
#[pyfunction]
#[pyo3(signature = (returns, confidence = None, ann_factor = None))]
fn cornish_fisher_var(
    returns: Vec<f64>,
    confidence: Option<f64>,
    ann_factor: Option<f64>,
) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::cornish_fisher_var(
        &returns, confidence, ann_factor,
    ))
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
#[pyo3(signature = (returns, confidence = None))]
fn tail_ratio(returns: Vec<f64>, confidence: Option<f64>) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::tail_ratio(&returns, confidence))
}

/// Outlier win ratio.
#[pyfunction]
#[pyo3(signature = (returns, confidence = None))]
fn outlier_win_ratio(returns: Vec<f64>, confidence: Option<f64>) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::outlier_win_ratio(&returns, confidence))
}

/// Outlier loss ratio.
#[pyfunction]
#[pyo3(signature = (returns, confidence = None))]
fn outlier_loss_ratio(returns: Vec<f64>, confidence: Option<f64>) -> PyResult<f64> {
    let confidence = confidence.unwrap_or(py_risk_defaults()?.tail_risk.confidence);
    Ok(fa::risk_metrics::outlier_loss_ratio(&returns, confidence))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cagr, m)?)?;
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
    m.add_function(wrap_pyfunction!(rolling_sharpe, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_sortino, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_volatility, m)?)?;
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
