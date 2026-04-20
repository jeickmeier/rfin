//! Python bindings for VaR backtesting: breach classification,
//! Kupiec POF, Christoffersen conditional coverage, and Basel
//! traffic-light classification.

use std::str::FromStr;

use super::types::{
    PyBacktestResult, PyChristoffersenResult, PyKupiecResult, PyMultiModelComparison,
    PyPnlExplanation, PyTrafficLightResult,
};
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_analytics::backtesting as bt;
use finstack_analytics::backtesting::Breach;
use finstack_analytics::lookback as lb;
use pyo3::prelude::*;
use pyo3::types::PyAny;

fn parse_var_method(method: &str) -> PyResult<bt::VarMethod> {
    bt::VarMethod::from_str(method).map_err(pyo3::exceptions::PyValueError::new_err)
}

// -------------------------------------------------------------------
// classify_breaches
// -------------------------------------------------------------------

/// Classify each observation as a VaR breach (hit) or miss.
///
/// A breach occurs when the realized P&L is more negative than the
/// VaR forecast. Returns a dense boolean series aligned 1:1 with the
/// input observations (``True`` = breach, ``False`` = miss).
///
/// # Arguments
///
/// * ``var_forecasts`` - Daily VaR forecasts (negative = loss threshold).
/// * ``realized_pnl`` - Daily realized P&L.
///
/// # Returns
///
/// Dense boolean breach indicator series aligned with the inputs.
/// Empty if the inputs have mismatched lengths or are empty.
#[pyfunction]
fn classify_breaches(var_forecasts: Vec<f64>, realized_pnl: Vec<f64>) -> Vec<bool> {
    bt::classify_breaches(&var_forecasts, &realized_pnl)
        .into_iter()
        .map(|breach| breach == Breach::Hit)
        .collect()
}

// -------------------------------------------------------------------
// kupiec_test
// -------------------------------------------------------------------

/// Kupiec Proportion of Failures (POF) unconditional coverage test.
///
/// Tests H0: observed breach rate equals 1 - confidence.
///
/// # Arguments
///
/// * ``breaches`` - Number of observed VaR breaches.
/// * ``n`` - Total observations.
/// * ``confidence`` - VaR confidence level (e.g. ``0.99``).
///
/// # Returns
///
/// :class:`KupiecResult` with test statistic, p-value, and breach counts.
#[pyfunction]
fn kupiec_test(breaches: usize, n: usize, confidence: f64) -> PyKupiecResult {
    PyKupiecResult {
        inner: bt::kupiec_test(breaches, n, confidence),
    }
}

// -------------------------------------------------------------------
// christoffersen_test
// -------------------------------------------------------------------

/// Christoffersen joint conditional coverage test.
///
/// Accepts a boolean breach indicator series (``True`` = hit) to preserve
/// the serial ordering required for the independence component.
///
/// # Arguments
///
/// * ``breach_indicators`` - Boolean series (``True`` = breach).
/// * ``confidence`` - VaR confidence level (e.g. ``0.99``).
///
/// # Returns
///
/// :class:`ChristoffersenResult` with LR statistics, p-values, and
/// transition counts.
#[pyfunction]
#[pyo3(signature = (breach_indicators, confidence = 0.99))]
fn christoffersen_test(breach_indicators: Vec<bool>, confidence: f64) -> PyChristoffersenResult {
    let seq: Vec<Breach> = breach_indicators
        .into_iter()
        .map(|b| if b { Breach::Hit } else { Breach::Miss })
        .collect();
    PyChristoffersenResult {
        inner: bt::christoffersen_test(&seq, confidence),
    }
}

// -------------------------------------------------------------------
// traffic_light
// -------------------------------------------------------------------

/// Basel Committee traffic-light classification of VaR model adequacy.
///
/// # Arguments
///
/// * ``breaches`` - Number of VaR exceptions in the evaluation window.
/// * ``n`` - Window size (typically 250 trading days).
/// * ``confidence`` - VaR confidence level (typically ``0.99``).
///
/// # Returns
///
/// :class:`TrafficLightResult` with zone, exceptions, and capital multiplier.
#[pyfunction]
fn traffic_light(breaches: usize, n: usize, confidence: f64) -> PyTrafficLightResult {
    PyTrafficLightResult {
        inner: bt::traffic_light(breaches, n, confidence),
    }
}

// -------------------------------------------------------------------
// run_backtest
// -------------------------------------------------------------------

/// Run a complete VaR backtest and return all statistics.
///
/// Aggregates Kupiec, Christoffersen, and traffic-light results.
///
/// # Arguments
///
/// * ``var_forecasts`` - Daily VaR forecasts (negative = loss threshold).
/// * ``realized_pnl`` - Daily realized P&L.
/// * ``confidence`` - VaR confidence level. Default ``0.99``.
/// * ``window_size`` - Traffic-light window size. Default ``250``.
///
/// # Returns
///
/// :class:`BacktestResult` aggregating all test outcomes.
#[pyfunction]
#[pyo3(signature = (var_forecasts, realized_pnl, confidence = 0.99, window_size = 250))]
fn run_backtest(
    var_forecasts: Vec<f64>,
    realized_pnl: Vec<f64>,
    confidence: f64,
    window_size: usize,
) -> PyBacktestResult {
    let cfg = bt::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    PyBacktestResult {
        inner: bt::run_backtest(&var_forecasts, &realized_pnl, &cfg),
    }
}

/// Build rolling VaR forecasts and aligned realized P&L using a canonical Rust method.
#[pyfunction]
#[pyo3(signature = (returns, lookback, confidence = 0.99, method = "Historical"))]
fn rolling_var_forecasts(
    returns: Vec<f64>,
    lookback: usize,
    confidence: f64,
    method: &str,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let parsed_method = parse_var_method(method)?;
    Ok(bt::rolling_var_forecasts(
        &returns,
        lookback,
        confidence,
        parsed_method,
    ))
}

/// Compare multiple model forecast series against the same realized P&L.
#[pyfunction]
#[pyo3(signature = (models, realized_pnl, confidence = 0.99, window_size = 250))]
fn compare_var_backtests(
    models: Vec<(String, Vec<f64>)>,
    realized_pnl: Vec<f64>,
    confidence: f64,
    window_size: usize,
) -> PyResult<PyMultiModelComparison> {
    let cfg = bt::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    let parsed_models: Vec<(bt::VarMethod, Vec<f64>)> = models
        .into_iter()
        .map(|(method, forecasts)| Ok((parse_var_method(&method)?, forecasts)))
        .collect::<PyResult<_>>()?;
    let refs: Vec<(bt::VarMethod, &[f64])> = parsed_models
        .iter()
        .map(|(method, forecasts)| (*method, forecasts.as_slice()))
        .collect();
    Ok(PyMultiModelComparison {
        inner: bt::compare_var_backtests(&refs, &realized_pnl, &cfg),
    })
}

/// Basel FRTB-style P&L explanation diagnostics.
#[pyfunction]
fn pnl_explanation(
    hypothetical_pnl: Vec<f64>,
    risk_theoretical_pnl: Vec<f64>,
    var: Vec<f64>,
) -> PyPnlExplanation {
    PyPnlExplanation {
        inner: bt::pnl_explanation(&hypothetical_pnl, &risk_theoretical_pnl, &var),
    }
}

/// Month-to-date index range into a sorted date array.
#[pyfunction]
#[pyo3(signature = (dates, as_of, offset_days = 0))]
fn mtd_select(
    py: Python<'_>,
    dates: Vec<Py<PyAny>>,
    as_of: Py<PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let parsed_dates = dates
        .into_iter()
        .map(|date| py_to_date(date.bind(py)))
        .collect::<PyResult<Vec<_>>>()?;
    let as_of = py_to_date(as_of.bind(py))?;
    let range = lb::mtd_select(&parsed_dates, as_of, offset_days);
    Ok((range.start, range.end))
}

/// Quarter-to-date index range into a sorted date array.
#[pyfunction]
#[pyo3(signature = (dates, as_of, offset_days = 0))]
fn qtd_select(
    py: Python<'_>,
    dates: Vec<Py<PyAny>>,
    as_of: Py<PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let parsed_dates = dates
        .into_iter()
        .map(|date| py_to_date(date.bind(py)))
        .collect::<PyResult<Vec<_>>>()?;
    let as_of = py_to_date(as_of.bind(py))?;
    let range = lb::qtd_select(&parsed_dates, as_of, offset_days);
    Ok((range.start, range.end))
}

/// Year-to-date index range into a sorted date array.
#[pyfunction]
#[pyo3(signature = (dates, as_of, offset_days = 0))]
fn ytd_select(
    py: Python<'_>,
    dates: Vec<Py<PyAny>>,
    as_of: Py<PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let parsed_dates = dates
        .into_iter()
        .map(|date| py_to_date(date.bind(py)))
        .collect::<PyResult<Vec<_>>>()?;
    let as_of = py_to_date(as_of.bind(py))?;
    let range = lb::ytd_select(&parsed_dates, as_of, offset_days);
    Ok((range.start, range.end))
}

/// Fiscal-year-to-date index range into a sorted date array.
#[pyfunction]
#[pyo3(signature = (dates, as_of, fiscal_start_month, fiscal_start_day = 1, offset_days = 0))]
fn fytd_select(
    py: Python<'_>,
    dates: Vec<Py<PyAny>>,
    as_of: Py<PyAny>,
    fiscal_start_month: u8,
    fiscal_start_day: u8,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let parsed_dates = dates
        .into_iter()
        .map(|date| py_to_date(date.bind(py)))
        .collect::<PyResult<Vec<_>>>()?;
    let as_of = py_to_date(as_of.bind(py))?;
    let config = finstack_core::dates::FiscalConfig::new(fiscal_start_month, fiscal_start_day)
        .map_err(core_to_py)?;
    let range = lb::fytd_select(&parsed_dates, as_of, config, offset_days);
    Ok((range.start, range.end))
}

// -------------------------------------------------------------------
// Registration
// -------------------------------------------------------------------

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(classify_breaches, m)?)?;
    m.add_function(wrap_pyfunction!(kupiec_test, m)?)?;
    m.add_function(wrap_pyfunction!(christoffersen_test, m)?)?;
    m.add_function(wrap_pyfunction!(traffic_light, m)?)?;
    m.add_function(wrap_pyfunction!(run_backtest, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_var_forecasts, m)?)?;
    m.add_function(wrap_pyfunction!(compare_var_backtests, m)?)?;
    m.add_function(wrap_pyfunction!(pnl_explanation, m)?)?;
    m.add_function(wrap_pyfunction!(mtd_select, m)?)?;
    m.add_function(wrap_pyfunction!(qtd_select, m)?)?;
    m.add_function(wrap_pyfunction!(ytd_select, m)?)?;
    m.add_function(wrap_pyfunction!(fytd_select, m)?)?;
    Ok(())
}
