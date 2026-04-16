//! Python bindings for VaR backtesting: breach classification,
//! Kupiec POF, Christoffersen conditional coverage, and Basel
//! traffic-light classification.
//!
//! Returns are native Python types (lists of tuples, tuples, dicts) so
//! callers do not need custom `#[pyclass]` wrappers for every result.

use finstack_analytics::backtesting as bt;
use finstack_analytics::backtesting::{Breach, TrafficLightZone};
use pyo3::prelude::*;
use pyo3::types::PyDict;

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
/// * ``var_method`` - Informational label (``"historical"``, ``"parametric"``,
///   ``"cornish_fisher"``). Not used for classification; included for symmetry
///   with the multi-model comparison orchestrator.
///
/// # Returns
///
/// List of ``(index, var_forecast, realized_pnl)`` tuples, one per breach.
/// Empty if the inputs have mismatched lengths or no breaches occurred.
#[pyfunction]
#[pyo3(signature = (var_forecasts, realized_pnl, var_method = "historical"))]
fn classify_breaches(
    var_forecasts: Vec<f64>,
    realized_pnl: Vec<f64>,
    var_method: &str,
) -> Vec<(usize, f64, f64)> {
    // ``var_method`` is accepted for API symmetry; classification is method-agnostic.
    let _ = var_method;

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
// Helpers
// -------------------------------------------------------------------

/// Reconstruct a breach sequence from a count and total, placing hits
/// uniformly. Only the aggregate breach count matters for `kupiec_test`,
/// so this synthetic placement is sufficient.
fn synth_breaches(breaches: usize, n: usize) -> Vec<Breach> {
    let mut seq = vec![Breach::Miss; n];
    for i in 0..breaches.min(n) {
        seq[i] = Breach::Hit;
    }
    seq
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
/// Tuple ``(lr_statistic, p_value, reject_h0_5pct)``. ``reject_h0_5pct``
/// is True when ``p_value < 0.05``.
#[pyfunction]
fn kupiec_test(breaches: usize, n: usize, confidence: f64) -> (f64, f64, bool) {
    let seq = synth_breaches(breaches, n);
    let r = bt::kupiec_test(&seq, confidence);
    (r.lr_statistic, r.p_value, r.reject_h0_5pct)
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
///
/// # Returns
///
/// Tuple ``(lr_cc, p_value_cc, reject_h0_5pct)``. ``reject_h0_5pct`` is
/// True when ``p_value_cc < 0.05``.
///
/// Uses confidence level ``0.99`` internally for the unconditional
/// component; only the independence component depends on ordering.
#[pyfunction]
#[pyo3(signature = (breach_indicators, confidence = 0.99))]
fn christoffersen_test(breach_indicators: Vec<bool>, confidence: f64) -> (f64, f64, bool) {
    let seq: Vec<Breach> = breach_indicators
        .into_iter()
        .map(|b| if b { Breach::Hit } else { Breach::Miss })
        .collect();
    let r = bt::christoffersen_test(&seq, confidence);
    (r.lr_cc, r.p_value_cc, r.reject_h0_5pct)
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
/// Tuple ``(zone_name, capital_multiplier)`` where ``zone_name`` is one
/// of ``"Green"``, ``"Yellow"``, ``"Red"``.
#[pyfunction]
fn traffic_light(breaches: usize, n: usize, confidence: f64) -> (String, f64) {
    let seq = synth_breaches(breaches, n);
    let r = bt::traffic_light(&seq, confidence, n);
    let zone = match r.zone {
        TrafficLightZone::Green => "Green",
        TrafficLightZone::Yellow => "Yellow",
        TrafficLightZone::Red => "Red",
    };
    (zone.to_string(), r.capital_multiplier)
}

// -------------------------------------------------------------------
// run_backtest (convenience: full report as a dict)
// -------------------------------------------------------------------

/// Run a complete VaR backtest and return all statistics in a dict.
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
/// Dict with keys ``kupiec``, ``christoffersen``, ``traffic_light``,
/// ``breach_count``, ``confidence``.
#[pyfunction]
#[pyo3(signature = (var_forecasts, realized_pnl, confidence = 0.99, window_size = 250))]
fn run_backtest<'py>(
    py: Python<'py>,
    var_forecasts: Vec<f64>,
    realized_pnl: Vec<f64>,
    confidence: f64,
    window_size: usize,
) -> PyResult<Bound<'py, PyDict>> {
    let cfg = bt::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    let result = bt::run_backtest(&var_forecasts, &realized_pnl, &cfg);

    let out = PyDict::new(py);

    let k = PyDict::new(py);
    k.set_item("lr_statistic", result.kupiec.lr_statistic)?;
    k.set_item("p_value", result.kupiec.p_value)?;
    k.set_item("breach_count", result.kupiec.breach_count)?;
    k.set_item("expected_count", result.kupiec.expected_count)?;
    k.set_item("total_observations", result.kupiec.total_observations)?;
    k.set_item("observed_rate", result.kupiec.observed_rate)?;
    k.set_item("reject_h0_5pct", result.kupiec.reject_h0_5pct)?;
    out.set_item("kupiec", k)?;

    let c = PyDict::new(py);
    c.set_item("lr_uc", result.christoffersen.lr_uc)?;
    c.set_item("lr_ind", result.christoffersen.lr_ind)?;
    c.set_item("lr_cc", result.christoffersen.lr_cc)?;
    c.set_item("p_value_uc", result.christoffersen.p_value_uc)?;
    c.set_item("p_value_ind", result.christoffersen.p_value_ind)?;
    c.set_item("p_value_cc", result.christoffersen.p_value_cc)?;
    c.set_item(
        "transition_counts",
        result.christoffersen.transition_counts.to_vec(),
    )?;
    c.set_item("reject_h0_5pct", result.christoffersen.reject_h0_5pct)?;
    out.set_item("christoffersen", c)?;

    let tl = PyDict::new(py);
    let zone = match result.traffic_light.zone {
        TrafficLightZone::Green => "Green",
        TrafficLightZone::Yellow => "Yellow",
        TrafficLightZone::Red => "Red",
    };
    tl.set_item("zone", zone)?;
    tl.set_item("exceptions", result.traffic_light.exceptions)?;
    tl.set_item(
        "capital_multiplier",
        result.traffic_light.capital_multiplier,
    )?;
    tl.set_item("window_size", result.traffic_light.window_size)?;
    tl.set_item("confidence", result.traffic_light.confidence)?;
    out.set_item("traffic_light", tl)?;

    out.set_item("breach_count", result.kupiec.breach_count)?;
    out.set_item("confidence", result.confidence)?;

    Ok(out)
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
