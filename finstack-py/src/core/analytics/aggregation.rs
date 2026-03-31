//! Python bindings for period aggregation utilities.
//!
//! Wraps `finstack_analytics::aggregation` for grouping returns by period
//! and computing period-level statistics.

use crate::core::dates::utils::py_to_date;
use finstack_analytics::aggregation;
use finstack_core::dates::PeriodKind;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Parse a frequency string into a `PeriodKind`.
fn parse_freq(s: &str) -> PyResult<PeriodKind> {
    s.parse::<PeriodKind>().map_err(|_| {
        PyValueError::new_err(format!(
            "Unknown frequency '{s}'. Expected: daily, weekly, monthly, quarterly, semiannual, annual"
        ))
    })
}

/// Group daily returns by period, compounding within each bucket.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// returns : list[float]
///     Return series aligned with ``dates``.
/// freq : str
///     Aggregation frequency: ``"daily"``, ``"weekly"``, ``"monthly"``,
///     ``"quarterly"``, ``"semiannual"``, ``"annual"``.
///
/// Returns
/// -------
/// list[tuple[str, float]]
///     ``(period_label, compounded_return)`` pairs in chronological order.
#[pyfunction]
#[pyo3(signature = (dates, returns, freq="monthly"))]
fn group_by_period(
    dates: &Bound<'_, PyAny>,
    returns: Vec<f64>,
    freq: &str,
) -> PyResult<Vec<(String, f64)>> {
    let date_list: Vec<Bound<'_, PyAny>> = dates.extract()?;
    let rust_dates: Vec<finstack_core::dates::Date> = date_list
        .iter()
        .map(|d| py_to_date(d))
        .collect::<PyResult<Vec<_>>>()?;
    let period_kind = parse_freq(freq)?;
    let grouped = aggregation::group_by_period(&rust_dates, &returns, period_kind, None);
    Ok(grouped
        .into_iter()
        .map(|(pid, val)| (pid.to_string(), val))
        .collect())
}

/// Compute period-level statistics from a return series.
///
/// Groups returns into buckets of the given frequency, compounds within
/// each, then computes win rate, payoff ratio, Kelly criterion, etc.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// returns : list[float]
///     Return series aligned with ``dates``.
/// freq : str
///     Aggregation frequency.
///
/// Returns
/// -------
/// dict
///     Keys: ``best``, ``worst``, ``consecutive_wins``, ``consecutive_losses``,
///     ``win_rate``, ``avg_return``, ``avg_win``, ``avg_loss``,
///     ``payoff_ratio``, ``profit_ratio``, ``profit_factor``,
///     ``cpc_ratio``, ``kelly_criterion``.
#[pyfunction]
#[pyo3(signature = (dates, returns, freq="monthly"))]
fn period_stats(
    py: Python<'_>,
    dates: &Bound<'_, PyAny>,
    returns: Vec<f64>,
    freq: &str,
) -> PyResult<Py<PyDict>> {
    let date_list: Vec<Bound<'_, PyAny>> = dates.extract()?;
    let rust_dates: Vec<finstack_core::dates::Date> = date_list
        .iter()
        .map(|d| py_to_date(d))
        .collect::<PyResult<Vec<_>>>()?;
    let period_kind = parse_freq(freq)?;
    let grouped = aggregation::group_by_period(&rust_dates, &returns, period_kind, None);
    let ps = aggregation::period_stats(&grouped);

    let dict = PyDict::new(py);
    dict.set_item("best", ps.best)?;
    dict.set_item("worst", ps.worst)?;
    dict.set_item("consecutive_wins", ps.consecutive_wins)?;
    dict.set_item("consecutive_losses", ps.consecutive_losses)?;
    dict.set_item("win_rate", ps.win_rate)?;
    dict.set_item("avg_return", ps.avg_return)?;
    dict.set_item("avg_win", ps.avg_win)?;
    dict.set_item("avg_loss", ps.avg_loss)?;
    dict.set_item("payoff_ratio", ps.payoff_ratio)?;
    dict.set_item("profit_ratio", ps.profit_ratio)?;
    dict.set_item("profit_factor", ps.profit_factor)?;
    dict.set_item("cpc_ratio", ps.cpc_ratio)?;
    dict.set_item("kelly_criterion", ps.kelly_criterion)?;
    Ok(dict.into())
}

/// Return grouped period returns as a flat list suitable for further analysis.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// returns : list[float]
///     Return series aligned with ``dates``.
/// freq : str
///     Aggregation frequency.
///
/// Returns
/// -------
/// list[float]
///     Compounded returns per period, in chronological order.
#[pyfunction]
#[pyo3(signature = (dates, returns, freq="monthly"))]
fn grouped_returns(
    dates: &Bound<'_, PyAny>,
    returns: Vec<f64>,
    freq: &str,
) -> PyResult<Py<PyList>> {
    let py = dates.py();
    let date_list: Vec<Bound<'_, PyAny>> = dates.extract()?;
    let rust_dates: Vec<finstack_core::dates::Date> = date_list
        .iter()
        .map(|d| py_to_date(d))
        .collect::<PyResult<Vec<_>>>()?;
    let period_kind = parse_freq(freq)?;
    let grouped = aggregation::group_by_period(&rust_dates, &returns, period_kind, None);
    let vals: Vec<f64> = grouped.into_iter().map(|(_, v)| v).collect();
    Ok(PyList::new(py, vals)?.into())
}

/// Register standalone aggregation functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(group_by_period, m)?)?;
    m.add_function(wrap_pyfunction!(period_stats, m)?)?;
    m.add_function(wrap_pyfunction!(grouped_returns, m)?)?;
    Ok(vec!["group_by_period", "period_stats", "grouped_returns"])
}
