//! Python bindings for the structured report-generation components.
//!
//! This module exposes the composable data components in
//! [`finstack_valuations::reporting`] as PyO3 functions that accept simple
//! Python inputs and return ``{"json": <dict>, "markdown": <str>}`` payloads.
//!
//! The ``json`` key contains the same structured dict produced by each
//! component's ``to_json()`` implementation (parsed into a real Python
//! ``dict`` via :mod:`json`), while ``markdown`` is the rendered table
//! string from ``to_markdown()``. Consumers can drop the markdown into a
//! Jupyter display, write the JSON to a file, or feed either into a
//! downstream template engine.
//!
//! A handful of formatting helpers (``format_bps``, ``format_pct``,
//! ``format_currency``, ``format_ratio``, ``format_scientific``) are exposed
//! as free functions for display-string formatting.

use crate::errors::display_to_py;
use finstack_valuations::reporting::format as fmt;
use finstack_valuations::reporting::{
    BucketFrequency, CashflowLadder, ReportComponent, ScenarioMatrix, WaterfallData,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 date string into a `time::Date`.
fn parse_date(s: &str) -> PyResult<time::Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    time::Date::parse(s, &format)
        .map_err(|e| PyValueError::new_err(format!("Invalid date '{s}': {e}")))
}

/// Convert a [`serde_json::Value`] into a native Python object using ``json.loads``.
fn value_to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
    let s = serde_json::to_string(value).map_err(display_to_py)?;
    let json_mod = py.import("json")?;
    json_mod.call_method1("loads", (s,))
}

/// Build the canonical ``{"json": ..., "markdown": ...}`` Python dict for a
/// [`ReportComponent`].
fn component_to_pydict<'py, C: ReportComponent>(
    py: Python<'py>,
    component: &C,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    let json_value = component.to_json();
    out.set_item("json", value_to_py(py, &json_value)?)?;
    out.set_item("markdown", component.to_markdown())?;
    out.set_item("component_type", component.component_type())?;
    Ok(out)
}

/// Parse a bucket-frequency string (case-insensitive) into [`BucketFrequency`].
fn parse_bucket_frequency(s: &str) -> PyResult<BucketFrequency> {
    match s.to_ascii_lowercase().as_str() {
        "monthly" => Ok(BucketFrequency::Monthly),
        "quarterly" => Ok(BucketFrequency::Quarterly),
        "semi_annual" | "semiannual" | "semi-annual" => Ok(BucketFrequency::SemiAnnual),
        "annual" | "yearly" => Ok(BucketFrequency::Annual),
        other => Err(PyValueError::new_err(format!(
            "unknown frequency '{other}' (expected monthly/quarterly/semi_annual/annual)"
        ))),
    }
}

// ---------------------------------------------------------------------------
// MetricsTable
// ---------------------------------------------------------------------------

/// Build a structured metrics table from a flat ``{name: value}`` dict.
///
/// This is a convenience entry point that does not require a full
/// :class:`ValuationResult`. Metric units and directions are inferred from
/// the metric name (see the Rust ``reporting`` module for the inference
/// rules).
///
/// Parameters
/// ----------
/// instrument_id : str
///     Identifier used as the table header and in the serialized JSON.
/// as_of : str
///     Valuation date (ISO 8601, ``YYYY-MM-DD``).
/// currency : str
///     Currency code (e.g. ``"USD"``).
/// npv : float
///     Net present value, shown in the markdown subtitle.
/// metrics : dict[str, float]
///     Ordered mapping of metric id → value. ``dv01``, ``cs01`` are
///     recognised as currency-per-bp; ``ytm``, ``oas``, ``*_spread`` as
///     percentages; ``*_duration`` / ``wal`` as years; etc.
///
/// Returns
/// -------
/// dict
///     ``{"json": <dict>, "markdown": <str>, "component_type": "metrics_table"}``
#[pyfunction]
#[pyo3(signature = (instrument_id, as_of, currency, npv, metrics))]
fn metrics_table_from_dict<'py>(
    py: Python<'py>,
    instrument_id: &str,
    as_of: &str,
    currency: &str,
    npv: f64,
    metrics: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyDict>> {
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use finstack_valuations::metrics::MetricId;
    use finstack_valuations::reporting::MetricsTable;
    use finstack_valuations::results::ValuationResult;
    use indexmap::IndexMap;
    use std::str::FromStr;

    let date = parse_date(as_of)?;
    let ccy = Currency::from_str(currency)
        .map_err(|e| PyValueError::new_err(format!("invalid currency '{currency}': {e}")))?;

    let mut measures: IndexMap<MetricId, f64> = IndexMap::with_capacity(metrics.len());
    for (k, v) in metrics.iter() {
        let name: String = k.extract()?;
        let value: f64 = v.extract()?;
        // `MetricId::from_str` never fails — unknown names become `Custom`.
        let metric_id = MetricId::from_str(&name).unwrap_or_else(|_| MetricId::custom(&name));
        measures.insert(metric_id, value);
    }

    let result = ValuationResult::stamped(instrument_id, date, Money::new(npv, ccy))
        .with_measures(measures);
    let table = MetricsTable::from_valuation_result(&result);
    component_to_pydict(py, &table)
}

// ---------------------------------------------------------------------------
// CashflowLadder
// ---------------------------------------------------------------------------

/// Build a time-bucketed cashflow summary.
///
/// Parameters
/// ----------
/// instrument_id : str
///     Label for the ladder.
/// currency : str
///     Currency code (display-only — no FX conversion is performed).
/// dates : list[str]
///     ISO 8601 dates aligned with *principal* and *interest*.
/// principal : list[float]
///     Principal amounts per date; same length as *dates*.
/// interest : list[float]
///     Interest amounts per date; same length as *dates*.
/// frequency : str
///     Bucket frequency: ``"monthly"``, ``"quarterly"``,
///     ``"semi_annual"`` or ``"annual"``.
///
/// Returns
/// -------
/// dict
///     ``{"json": <dict>, "markdown": <str>, "component_type": "cashflow_ladder"}``.
///     The ``json`` payload contains ``buckets``, ``total``, and
///     ``weighted_avg_life``.
#[pyfunction]
#[pyo3(signature = (instrument_id, currency, dates, principal, interest, frequency="quarterly"))]
fn cashflow_ladder<'py>(
    py: Python<'py>,
    instrument_id: &str,
    currency: &str,
    dates: Vec<String>,
    principal: Vec<f64>,
    interest: Vec<f64>,
    frequency: &str,
) -> PyResult<Bound<'py, PyDict>> {
    if dates.len() != principal.len() || dates.len() != interest.len() {
        return Err(PyValueError::new_err(format!(
            "length mismatch: dates={}, principal={}, interest={}",
            dates.len(),
            principal.len(),
            interest.len(),
        )));
    }
    let freq = parse_bucket_frequency(frequency)?;
    let mut cashflows: Vec<(finstack_core::dates::Date, f64, f64)> =
        Vec::with_capacity(dates.len());
    for (i, date_str) in dates.iter().enumerate() {
        let d = parse_date(date_str)?;
        cashflows.push((d, principal[i], interest[i]));
    }

    let ladder = CashflowLadder::from_cashflows(instrument_id, currency, &cashflows, freq);
    component_to_pydict(py, &ladder)
}

// ---------------------------------------------------------------------------
// ScenarioMatrix
// ---------------------------------------------------------------------------

/// Build a scenario-name × metric matrix from a list of ``(name, {metric: value})``
/// entries.
///
/// Parameters
/// ----------
/// title : str
///     Display title for the matrix.
/// scenarios : list[tuple[str, dict[str, float]]]
///     Ordered list of ``(scenario_name, {metric_id: value})`` pairs.
///     Every scenario must use the same set of metric ids.
/// base_case : str | None
///     Optional scenario name to use for delta computation. Deltas are
///     produced only if the name matches an entry.
///
/// Returns
/// -------
/// dict
///     ``{"json": <dict>, "markdown": <str>, "component_type": "scenario_matrix"}``.
///     The ``json`` payload contains ``scenario_names``, ``metric_ids``,
///     ``values`` (``[scenarios][metrics]``) and optional ``deltas``.
#[pyfunction]
#[pyo3(signature = (title, scenarios, base_case=None))]
fn scenario_matrix<'py>(
    py: Python<'py>,
    title: &str,
    scenarios: &Bound<'py, PyList>,
    base_case: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    // Collect scenarios as a vec of (name, IndexMap<String, f64>) preserving order.
    let mut scenario_data: Vec<(String, indexmap::IndexMap<String, f64>)> = Vec::new();
    let mut metric_order: Vec<String> = Vec::new();

    for item in scenarios.iter() {
        let tup: (String, Bound<'_, PyDict>) = item.extract()?;
        let name = tup.0;
        let py_map = tup.1;
        let mut metrics: indexmap::IndexMap<String, f64> = indexmap::IndexMap::new();
        for (k, v) in py_map.iter() {
            let mk: String = k.extract()?;
            let mv: f64 = v.extract()?;
            if metric_order.iter().all(|m| m != &mk) {
                metric_order.push(mk.clone());
            }
            metrics.insert(mk, mv);
        }
        scenario_data.push((name, metrics));
    }

    // Build the matrix inline — we don't need a ValuationResult because we
    // already have the metric values; mirror the shape of
    // ScenarioMatrix::from_scenario_results without going through MetricId.
    let scenario_names: Vec<String> = scenario_data.iter().map(|(n, _)| n.clone()).collect();
    let values: Vec<Vec<f64>> = scenario_data
        .iter()
        .map(|(_, metrics)| {
            metric_order
                .iter()
                .map(|mid| *metrics.get(mid).unwrap_or(&f64::NAN))
                .collect()
        })
        .collect();

    let base_case_index =
        base_case.and_then(|name| scenario_names.iter().position(|s| s == name));
    let deltas = base_case_index.map(|base_idx| {
        let base_row = values[base_idx].clone();
        values
            .iter()
            .map(|row| {
                row.iter()
                    .zip(base_row.iter())
                    .map(|(v, b)| v - b)
                    .collect::<Vec<f64>>()
            })
            .collect::<Vec<Vec<f64>>>()
    });

    // Re-use the Rust renderer by constructing a ScenarioMatrix directly via its public fields.
    let matrix = ScenarioMatrix {
        title: title.to_string(),
        scenario_names,
        metric_ids: metric_order,
        values,
        base_case_index,
        deltas,
    };
    component_to_pydict(py, &matrix)
}

// ---------------------------------------------------------------------------
// WaterfallData
// ---------------------------------------------------------------------------

/// Build an ordered waterfall from factor contributions.
///
/// Parameters
/// ----------
/// title : str
///     Display title for the chart.
/// currency : str
///     Currency code (display-only).
/// start_value : float
///     Opening value (e.g. T₀ portfolio value).
/// end_value : float
///     Closing value (e.g. T₁ portfolio value).
/// steps : list[tuple[str, float]]
///     Ordered ``(label, contribution)`` pairs. The residual
///     (``end_value - start_value - sum(steps)``) is computed automatically.
///
/// Returns
/// -------
/// dict
///     ``{"json": <dict>, "markdown": <str>, "component_type": "waterfall_data"}``.
#[pyfunction]
#[pyo3(signature = (title, currency, start_value, end_value, steps))]
fn waterfall_from_steps<'py>(
    py: Python<'py>,
    title: &str,
    currency: &str,
    start_value: f64,
    end_value: f64,
    steps: Vec<(String, f64)>,
) -> PyResult<Bound<'py, PyDict>> {
    let waterfall =
        WaterfallData::from_attribution(title, currency, start_value, end_value, &steps);
    component_to_pydict(py, &waterfall)
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// Format a decimal fraction in basis points (e.g. ``0.0025 -> "25.0 bps"``).
#[pyfunction]
#[pyo3(signature = (value, decimals=1))]
fn format_bps(value: f64, decimals: usize) -> String {
    fmt::format_bps(value, decimals)
}

/// Format a decimal fraction as a percentage (e.g. ``0.0534 -> "5.34%"``).
#[pyfunction]
#[pyo3(signature = (value, decimals=2))]
fn format_pct(value: f64, decimals: usize) -> String {
    fmt::format_pct(value, decimals)
}

/// Format a currency amount with thousands separators
/// (e.g. ``1234567.89, "USD" -> "USD 1,234,567.89"``).
#[pyfunction]
#[pyo3(signature = (value, currency="USD", decimals=2))]
fn format_currency(value: f64, currency: &str, decimals: usize) -> String {
    fmt::format_currency(value, currency, decimals)
}

/// Format a dimensionless ratio with an ``x`` suffix
/// (e.g. ``3.5 -> "3.50x"``).
#[pyfunction]
#[pyo3(signature = (value, decimals=2))]
fn format_ratio(value: f64, decimals: usize) -> String {
    fmt::format_ratio(value, decimals)
}

/// Format a number in scientific notation
/// (e.g. ``0.000123 -> "1.23e-4"``).
#[pyfunction]
#[pyo3(signature = (value, sig_figs=3))]
fn format_scientific(value: f64, sig_figs: usize) -> String {
    fmt::format_scientific(value, sig_figs)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register reporting functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(metrics_table_from_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(cashflow_ladder, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(scenario_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(waterfall_from_steps, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(format_bps, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(format_pct, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(format_currency, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(format_ratio, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(format_scientific, m)?)?;
    Ok(())
}
