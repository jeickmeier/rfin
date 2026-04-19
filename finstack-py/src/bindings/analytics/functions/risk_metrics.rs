use super::super::types::{
    PyCagrBasis, PyRollingSharpe, PyRollingSortino, PyRollingVolatility, PyRuinDefinition,
    PyRuinEstimate, PyRuinModel,
};
use crate::bindings::core::dates::utils::py_to_date;
use finstack_analytics as fa;
use pyo3::prelude::*;

/// CAGR using a supplied annualization basis.
#[pyfunction]
fn cagr(returns: Vec<f64>, basis: &PyCagrBasis) -> f64 {
    fa::risk_metrics::cagr(&returns, basis.inner)
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
#[pyo3(signature = (returns, annualize = true, ann_factor = 252.0, mar = 0.0))]
fn sortino(returns: Vec<f64>, annualize: bool, ann_factor: f64, mar: f64) -> f64 {
    fa::risk_metrics::sortino(&returns, annualize, ann_factor, mar)
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
