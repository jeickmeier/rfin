use super::super::performance::parse_freq;
use super::super::types::PyPeriodStats;
use crate::bindings::core::dates::utils::py_to_date;
use finstack_analytics as fa;
use pyo3::prelude::*;

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
    PyPeriodStats {
        inner: fa::aggregation::period_stats_from_returns(&returns),
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(group_by_period, m)?)?;
    m.add_function(wrap_pyfunction!(period_stats, m)?)?;
    Ok(())
}
