//! Python bindings for VaR backtesting: breach classification,
//! Kupiec POF, Christoffersen conditional coverage, and Basel
//! traffic-light classification.

use super::types::{
    PyBacktestResult, PyChristoffersenResult, PyKupiecResult, PyTrafficLightResult,
};
use finstack_analytics::backtesting as bt;
use finstack_analytics::backtesting::Breach;
use pyo3::prelude::*;

// -------------------------------------------------------------------
// classify_breaches
// -------------------------------------------------------------------

/// Classify each observation as a VaR breach (hit) or miss.
///
/// A breach occurs when the realized P&L is more negative than the
/// VaR forecast. Returns a list of ``(index, var_forecast, realized_pnl)``
/// tuples for each observed breach.
///
/// # Arguments
///
/// * ``var_forecasts`` - Daily VaR forecasts (negative = loss threshold).
/// * ``realized_pnl`` - Daily realized P&L.
///
/// # Returns
///
/// List of ``(index, var_forecast, realized_pnl)`` tuples, one per breach.
/// Empty if the inputs have mismatched lengths or no breaches occurred.
#[pyfunction]
fn classify_breaches(var_forecasts: Vec<f64>, realized_pnl: Vec<f64>) -> Vec<(usize, f64, f64)> {
    let breaches = bt::classify_breaches(&var_forecasts, &realized_pnl);
    breaches
        .iter()
        .enumerate()
        .filter_map(|(i, b)| match b {
            Breach::Hit => Some((i, var_forecasts[i], realized_pnl[i])),
            Breach::Miss => None,
        })
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

// -------------------------------------------------------------------
// Registration
// -------------------------------------------------------------------

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(classify_breaches, m)?)?;
    m.add_function(wrap_pyfunction!(kupiec_test, m)?)?;
    m.add_function(wrap_pyfunction!(christoffersen_test, m)?)?;
    m.add_function(wrap_pyfunction!(traffic_light, m)?)?;
    m.add_function(wrap_pyfunction!(run_backtest, m)?)?;
    Ok(())
}
