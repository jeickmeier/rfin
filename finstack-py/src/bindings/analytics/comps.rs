//! Python bindings for the comparable company analysis module.
//!
//! Exposes a function-based API for cross-sectional peer analytics:
//!
//! - Descriptive peer statistics (`peer_stats`).
//! - Percentile rank and z-score of a subject within a peer distribution.
//! - Single-factor OLS regression for fair-value estimation.
//! - Safe division helper (`compute_multiple`).
//! - Multi-dimension composite rich/cheap scoring (`score_relative_value`).
//!
//! The scoring API takes plain dicts/lists from Python rather than the
//! strongly-typed `CompanyMetrics`/`PeerSet` structs used in Rust. Each
//! peer is a dict keyed by metric name; dimensions are `(name, weight)`
//! tuples. This keeps the Python surface small while still exercising the
//! underlying `finstack_analytics::comps::score_relative_value` logic.

use finstack_analytics::comps::{
    peer_stats as core_peer_stats, percentile_rank as core_percentile_rank,
    regression_fair_value as core_regression, score_relative_value as core_score,
    z_score as core_z_score, CompanyMetrics, MetricExtractor, PeerSet, PeriodBasis,
    ScoringDimension,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::errors::core_to_py;

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Percentile rank of ``value`` within ``peer_values`` (0-100 scale).
///
/// Uses the "fraction of values less than or equal" convention. Returns
/// 50.0 as a neutral fallback when ``peer_values`` is empty.
///
/// Arguments:
///     value: The subject value to rank.
///     peer_values: Peer distribution (need not be sorted).
///
/// Returns:
///     Percentile rank in [0, 100].
#[pyfunction]
#[pyo3(text_signature = "(value, peer_values)")]
fn percentile_rank(value: f64, peer_values: Vec<f64>) -> f64 {
    core_percentile_rank(&peer_values, value).unwrap_or(0.5) * 100.0
}

/// Standard (z-) score of ``value`` in the peer distribution.
///
/// Returns ``0.0`` if fewer than two peers are provided or the peer
/// distribution has zero variance.
///
/// Arguments:
///     value: The subject value.
///     peer_values: Peer distribution.
///
/// Returns:
///     ``(value - mean(peers)) / stddev(peers)``.
#[pyfunction]
#[pyo3(text_signature = "(value, peer_values)")]
fn z_score(value: f64, peer_values: Vec<f64>) -> f64 {
    core_z_score(&peer_values, value).unwrap_or(0.0)
}

/// Descriptive statistics for a peer distribution.
///
/// Arguments:
///     peer_values: Peer distribution (need not be sorted).
///
/// Returns:
///     Dict with keys ``{"mean", "median", "q1", "q3", "iqr", "std_dev",
///     "min", "max", "n"}``. Returns an empty dict when ``peer_values``
///     is empty.
#[pyfunction]
#[pyo3(text_signature = "(peer_values)")]
fn peer_stats<'py>(py: Python<'py>, peer_values: Vec<f64>) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    if let Some(stats) = core_peer_stats(&peer_values) {
        d.set_item("mean", stats.mean)?;
        d.set_item("median", stats.median)?;
        d.set_item("q1", stats.q1)?;
        d.set_item("q3", stats.q3)?;
        d.set_item("iqr", stats.q3 - stats.q1)?;
        d.set_item("std_dev", stats.std_dev)?;
        d.set_item("min", stats.min)?;
        d.set_item("max", stats.max)?;
        d.set_item("n", stats.count)?;
    }
    Ok(d)
}

/// Single-factor OLS fit and evaluation at the subject's X.
///
/// Regresses ``y_values`` on ``x_values`` and returns the fitted value
/// and residual for the subject. Conventions:
///
/// - ``fitted_value = intercept + slope * subject_x``
/// - ``residual = subject_y - fitted_value`` if a subject Y is known,
///   otherwise ``0.0`` (this binding only receives ``subject_x`` and
///   computes the fitted value).
///
/// Arguments:
///     x_values: Peer X observations (independent variable).
///     y_values: Peer Y observations (dependent variable). Must be
///         the same length as ``x_values``.
///     subject_x: Subject's X value at which to evaluate the fit.
///
/// Returns:
///     Dict with keys ``{"slope", "intercept", "r_squared",
///     "fitted_value", "residual", "n"}``. Returns an empty dict if
///     fewer than three observations are available or the regression
///     cannot be computed (e.g., zero variance in X).
#[pyfunction]
#[pyo3(text_signature = "(x_values, y_values, subject_x)")]
fn regression_fair_value<'py>(
    py: Python<'py>,
    x_values: Vec<f64>,
    y_values: Vec<f64>,
    subject_x: f64,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    // The core function needs a subject_y to compute the residual. Since
    // this minimal binding asks the caller for only subject_x, pass 0.0
    // and report the raw fitted_value. Callers that care about residual
    // can subtract the fitted_value from their own observed subject_y.
    if let Some(reg) = core_regression(&x_values, &y_values, subject_x, 0.0) {
        d.set_item("slope", reg.slope)?;
        d.set_item("intercept", reg.intercept)?;
        d.set_item("r_squared", reg.r_squared)?;
        d.set_item("fitted_value", reg.fitted_value)?;
        // residual here is (0 - fitted_value); expose as the raw fitted
        // value's negation so callers see something meaningful, but the
        // more useful interpretation is: residual = subject_y - fitted.
        d.set_item("residual", reg.residual)?;
        d.set_item("n", reg.n)?;
    }
    Ok(d)
}

// ---------------------------------------------------------------------------
// Multiples
// ---------------------------------------------------------------------------

/// Safe division: ``numerator / denominator`` with guards.
///
/// Returns ``f64::NAN`` if the denominator is non-positive or either
/// input is non-finite. This mirrors the guard used inside
/// ``finstack_analytics::comps::multiples::compute_multiple`` for the
/// denominator term of a valuation multiple.
///
/// Arguments:
///     numerator: Typically EV, market cap, price, or spread.
///     denominator: Typically EBITDA, earnings, book value, or leverage.
///
/// Returns:
///     ``numerator / denominator`` when the inputs are valid, otherwise
///     ``NaN``.
#[pyfunction]
#[pyo3(text_signature = "(numerator, denominator)")]
fn compute_multiple(numerator: f64, denominator: f64) -> f64 {
    if denominator <= 0.0 || !denominator.is_finite() || !numerator.is_finite() {
        f64::NAN
    } else {
        numerator / denominator
    }
}

// ---------------------------------------------------------------------------
// Composite relative-value scoring
// ---------------------------------------------------------------------------

/// Convert a ``{metric_name: value}`` dict into a `CompanyMetrics`.
///
/// Known field names (e.g. ``"leverage"``, ``"oas_bps"``, ``"ebitda"``)
/// are mapped onto their dedicated optional fields; everything else is
/// stored in the `custom` map. Non-numeric entries are skipped.
fn dict_to_company_metrics(id: &str, d: &Bound<'_, PyDict>) -> PyResult<CompanyMetrics> {
    let mut m = CompanyMetrics::new(id);
    for (key, val) in d.iter() {
        let name: String = key.extract()?;
        let Ok(v) = val.extract::<f64>() else {
            continue;
        };
        match name.as_str() {
            "enterprise_value" => m.enterprise_value = Some(v),
            "market_cap" => m.market_cap = Some(v),
            "share_price" => m.share_price = Some(v),
            "oas_bps" => m.oas_bps = Some(v),
            "yield_pct" => m.yield_pct = Some(v),
            "ebitda" => m.ebitda = Some(v),
            "revenue" => m.revenue = Some(v),
            "ebit" => m.ebit = Some(v),
            "ufcf" => m.ufcf = Some(v),
            "lfcf" => m.lfcf = Some(v),
            "net_income" => m.net_income = Some(v),
            "book_value" => m.book_value = Some(v),
            "tangible_book_value" => m.tangible_book_value = Some(v),
            "dividends_per_share" => m.dividends_per_share = Some(v),
            "leverage" => m.leverage = Some(v),
            "interest_coverage" => m.interest_coverage = Some(v),
            "revenue_growth" => m.revenue_growth = Some(v),
            "ebitda_margin" => m.ebitda_margin = Some(v),
            _ => {
                m.custom.insert(name, v);
            }
        }
    }
    Ok(m)
}

/// Whether ``name`` maps onto a named field on `CompanyMetrics` (vs. a
/// custom-map entry). Used to pick the right `MetricExtractor` variant.
fn is_named_field(name: &str) -> bool {
    matches!(
        name,
        "enterprise_value"
            | "market_cap"
            | "share_price"
            | "oas_bps"
            | "yield_pct"
            | "ebitda"
            | "revenue"
            | "ebit"
            | "ufcf"
            | "lfcf"
            | "net_income"
            | "book_value"
            | "tangible_book_value"
            | "dividends_per_share"
            | "leverage"
            | "interest_coverage"
            | "revenue_growth"
            | "ebitda_margin"
    )
}

/// Score a subject against its peers across multiple weighted dimensions.
///
/// Each dimension is a ``(metric_name, weight)`` tuple. For each
/// dimension the binding computes percentile rank, z-score, and (since
/// no X extractors are provided) a univariate score; the composite is
/// the weighted average where positive = cheap, negative = rich.
///
/// Arguments:
///     subject_metrics: Dict of ``{metric_name: value}`` for the subject.
///     peer_metrics: List of dicts, one per peer, same schema as the
///         subject.
///     dimensions: List of ``(metric_name, weight)`` tuples selecting
///         which metrics to score and their composite weights.
///
/// Returns:
///     Dict with keys ``{"composite_score", "confidence", "peer_count",
///     "by_dimension"}`` where ``by_dimension`` is a dict mapping each
///     metric name to ``{"percentile", "z_score", "weight"}``.
#[pyfunction]
#[pyo3(text_signature = "(subject_metrics, peer_metrics, dimensions)")]
fn score_relative_value<'py>(
    py: Python<'py>,
    subject_metrics: &Bound<'_, PyDict>,
    peer_metrics: Vec<Bound<'_, PyDict>>,
    dimensions: Vec<(String, f64)>,
) -> PyResult<Bound<'py, PyDict>> {
    // Build CompanyMetrics for subject + peers.
    let subject = dict_to_company_metrics("SUBJECT", subject_metrics)?;
    let mut peers: Vec<CompanyMetrics> = Vec::with_capacity(peer_metrics.len());
    for (i, pd) in peer_metrics.iter().enumerate() {
        peers.push(dict_to_company_metrics(&format!("PEER_{i}"), pd)?);
    }

    let peer_set = PeerSet::new(subject, peers, PeriodBasis::Ltm);

    // Map (name, weight) pairs to ScoringDimension (univariate).
    let scoring_dims: Vec<ScoringDimension> = dimensions
        .into_iter()
        .map(|(name, weight)| {
            let extractor = if is_named_field(&name) {
                MetricExtractor::Named(name.clone())
            } else {
                MetricExtractor::Custom(name.clone())
            };
            ScoringDimension {
                label: name,
                y_extractor: extractor,
                x_extractors: vec![],
                weight,
            }
        })
        .collect();

    let result = core_score(&peer_set, &scoring_dims).map_err(core_to_py)?;

    let out = PyDict::new(py);
    out.set_item("composite_score", result.composite_score)?;
    out.set_item("confidence", result.confidence)?;
    out.set_item("peer_count", result.peer_count)?;

    let by_dim = PyDict::new(py);
    for d in &result.dimensions {
        let dim_dict = PyDict::new(py);
        dim_dict.set_item("percentile", d.percentile)?;
        dim_dict.set_item("z_score", d.z_score)?;
        dim_dict.set_item("weight", d.weight)?;
        by_dim.set_item(&d.label, dim_dict)?;
    }
    out.set_item("by_dimension", by_dim)?;

    Ok(out)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register comps bindings on the analytics submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(percentile_rank, m)?)?;
    m.add_function(wrap_pyfunction!(z_score, m)?)?;
    m.add_function(wrap_pyfunction!(peer_stats, m)?)?;
    m.add_function(wrap_pyfunction!(regression_fair_value, m)?)?;
    m.add_function(wrap_pyfunction!(compute_multiple, m)?)?;
    m.add_function(wrap_pyfunction!(score_relative_value, m)?)?;
    Ok(())
}
