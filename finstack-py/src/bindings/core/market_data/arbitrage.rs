//! Python bindings for `finstack_core::market_data::arbitrage`.
//!
//! Exposes a function-based API for model-free volatility surface arbitrage
//! detection. Vol surfaces are constructed internally from flat arrays so
//! callers can work directly with numpy-friendly inputs without first
//! building a `VolSurface` wrapper.

use std::collections::HashMap;

use finstack_core::market_data::arbitrage::{
    check_surface, ArbitrageCheck, ArbitrageCheckConfig, ArbitrageSeverity, ArbitrageType,
    ArbitrageViolation, ButterflyCheck, CalendarSpreadCheck, LocalVolDensityCheck,
};
use finstack_core::market_data::surfaces::VolSurface;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::errors::core_to_py;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a [`VolSurface`] from Python-friendly flat arrays.
///
/// `vols` is indexed as `vols[expiry_idx][strike_idx]`.
fn build_surface(
    id: &str,
    expiries: &[f64],
    strikes: &[f64],
    vols: &[Vec<f64>],
) -> PyResult<VolSurface> {
    if vols.len() != expiries.len() {
        return Err(PyValueError::new_err(format!(
            "vols has {} rows but expiries has {} entries",
            vols.len(),
            expiries.len()
        )));
    }
    let mut flat: Vec<f64> = Vec::with_capacity(expiries.len() * strikes.len());
    for (i, row) in vols.iter().enumerate() {
        if row.len() != strikes.len() {
            return Err(PyValueError::new_err(format!(
                "vols[{i}] has {} entries but strikes has {}",
                row.len(),
                strikes.len()
            )));
        }
        flat.extend_from_slice(row);
    }
    VolSurface::from_grid(id, expiries, strikes, &flat).map_err(core_to_py)
}

/// Stable lowercase string name for an [`ArbitrageType`].
fn arbitrage_type_str(t: ArbitrageType) -> &'static str {
    match t {
        ArbitrageType::Butterfly => "butterfly",
        ArbitrageType::CalendarSpread => "calendar_spread",
        ArbitrageType::LocalVolDensity => "local_vol_density",
        ArbitrageType::SviMomentBound => "svi_moment_bound",
        ArbitrageType::SviButterflyCondition => "svi_butterfly_condition",
        ArbitrageType::SviCalendarSpread => "svi_calendar_spread",
    }
}

/// Stable lowercase string name for an [`ArbitrageSeverity`].
fn severity_str(s: ArbitrageSeverity) -> &'static str {
    match s {
        ArbitrageSeverity::Negligible => "negligible",
        ArbitrageSeverity::Minor => "minor",
        ArbitrageSeverity::Major => "major",
        ArbitrageSeverity::Critical => "critical",
    }
}

/// Convert a single violation into a Python dict.
fn violation_to_dict<'py>(
    py: Python<'py>,
    v: &ArbitrageViolation,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("type", arbitrage_type_str(v.violation_type))?;
    d.set_item("severity", severity_str(v.severity))?;
    d.set_item("strike", v.location.strike)?;
    d.set_item("expiry", v.location.expiry)?;
    if let Some(ae) = v.location.adjacent_expiry {
        d.set_item("adjacent_expiry", ae)?;
    } else {
        d.set_item("adjacent_expiry", py.None())?;
    }
    d.set_item("magnitude", v.magnitude)?;
    d.set_item("value", v.magnitude)?;
    d.set_item("message", v.description.as_str())?;
    d.set_item("description", v.description.as_str())?;
    Ok(d)
}

/// Convert a slice of violations into a Python list of dicts.
fn violations_to_pylist<'py>(
    py: Python<'py>,
    violations: &[ArbitrageViolation],
) -> PyResult<Bound<'py, PyList>> {
    let items: Vec<Bound<'py, PyDict>> = violations
        .iter()
        .map(|v| violation_to_dict(py, v))
        .collect::<PyResult<Vec<_>>>()?;
    PyList::new(py, items)
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Check butterfly arbitrage (strike convexity of total variance).
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// tolerance : float, optional
///     Tolerance in total-variance units. Default ``1e-6``.
///
/// Returns
/// -------
/// list[dict]
///     One dict per violation with keys ``type``, ``severity``, ``strike``,
///     ``expiry``, ``adjacent_expiry``, ``magnitude``, ``value``,
///     ``message``, ``description``.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, tolerance = 1e-6))]
fn check_butterfly<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyList>> {
    let surface = build_surface("py-surface", &expiries, &strikes, &vols)?;
    let checker = ButterflyCheck { tolerance };
    let violations = checker.check(&surface);
    violations_to_pylist(py, &violations)
}

/// Check calendar spread arbitrage (expiry monotonicity of total variance).
///
/// Parameters mirror :func:`check_butterfly`.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, tolerance = 1e-6))]
fn check_calendar_spread<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyList>> {
    let surface = build_surface("py-surface", &expiries, &strikes, &vols)?;
    let checker = CalendarSpreadCheck { tolerance };
    let violations = checker.check(&surface);
    violations_to_pylist(py, &violations)
}

/// Check Dupire local-vol density positivity.
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// forward_prices : list[float]
///     Forward price per expiry (length must equal ``len(expiries)``).
///
/// Notes
/// -----
/// The underlying Rust check takes a single forward. When per-expiry forwards
/// are supplied, the check is run once per expiry with that expiry's forward
/// and only the corresponding expiry's violations are kept. This is
/// equivalent to the scalar case when all forwards are identical.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, forward_prices))]
fn check_local_vol_density<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    forward_prices: Vec<f64>,
) -> PyResult<Bound<'py, PyList>> {
    if forward_prices.len() != expiries.len() {
        return Err(PyValueError::new_err(format!(
            "forward_prices has {} entries but expiries has {}",
            forward_prices.len(),
            expiries.len()
        )));
    }
    let surface = build_surface("py-surface", &expiries, &strikes, &vols)?;

    // Fast path: all forwards equal -> single check.
    let all_equal = forward_prices
        .windows(2)
        .all(|w| (w[0] - w[1]).abs() < 1e-14);

    let mut violations: Vec<ArbitrageViolation> = Vec::new();
    if all_equal {
        let checker = LocalVolDensityCheck {
            forward: forward_prices[0],
            tolerance: 1e-10,
        };
        violations.extend(checker.check(&surface));
    } else {
        for (i, &t) in expiries.iter().enumerate() {
            let checker = LocalVolDensityCheck {
                forward: forward_prices[i],
                tolerance: 1e-10,
            };
            let slice_viols = checker
                .check(&surface)
                .into_iter()
                .filter(|v| (v.location.expiry - t).abs() < 1e-12);
            violations.extend(slice_viols);
        }
    }
    violations_to_pylist(py, &violations)
}

/// Run butterfly, calendar-spread, and local-vol density checks together.
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// tolerance : float, optional
///     Shared tolerance for all checks. Default ``1e-6``.
///
/// Returns
/// -------
/// dict
///     Aggregated report with keys ``total_violations``, ``passed``,
///     ``by_severity`` (dict ``severity -> count``), ``by_type``
///     (dict ``type -> count``), and ``violations`` (list[dict]).
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, tolerance = 1e-6))]
fn check_all<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyDict>> {
    let surface = build_surface("py-surface", &expiries, &strikes, &vols)?;
    let config = ArbitrageCheckConfig {
        check_butterfly: true,
        check_calendar_spread: true,
        // Local-vol check requires a forward; skip when not supplied here.
        check_local_vol_density: false,
        forward: None,
        tolerance,
        min_severity: ArbitrageSeverity::Negligible,
    };
    let report = check_surface(&surface, &config);

    let out = PyDict::new(py);
    out.set_item("total_violations", report.violations.len())?;
    out.set_item("passed", report.passed)?;

    let by_sev = PyDict::new(py);
    let mut sev_counts: HashMap<ArbitrageSeverity, usize> = HashMap::new();
    for v in &report.violations {
        *sev_counts.entry(v.severity).or_insert(0) += 1;
    }
    for sev in [
        ArbitrageSeverity::Negligible,
        ArbitrageSeverity::Minor,
        ArbitrageSeverity::Major,
        ArbitrageSeverity::Critical,
    ] {
        by_sev.set_item(severity_str(sev), sev_counts.get(&sev).copied().unwrap_or(0))?;
    }
    out.set_item("by_severity", by_sev)?;

    let by_type = PyDict::new(py);
    let mut type_counts: HashMap<ArbitrageType, usize> = HashMap::new();
    for v in &report.violations {
        *type_counts.entry(v.violation_type).or_insert(0) += 1;
    }
    for t in [
        ArbitrageType::Butterfly,
        ArbitrageType::CalendarSpread,
        ArbitrageType::LocalVolDensity,
    ] {
        by_type.set_item(arbitrage_type_str(t), type_counts.get(&t).copied().unwrap_or(0))?;
    }
    out.set_item("by_type", by_type)?;

    out.set_item("violations", violations_to_pylist(py, &report.violations)?)?;
    out.set_item("elapsed_us", report.elapsed_us)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register the `finstack.core.market_data.arbitrage` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "arbitrage")?;
    m.setattr(
        "__doc__",
        "Volatility surface arbitrage detection: butterfly, calendar spread, local-vol density.",
    )?;

    m.add_function(wrap_pyfunction!(check_butterfly, &m)?)?;
    m.add_function(wrap_pyfunction!(check_calendar_spread, &m)?)?;
    m.add_function(wrap_pyfunction!(check_local_vol_density, &m)?)?;
    m.add_function(wrap_pyfunction!(check_all, &m)?)?;

    let all = PyList::new(
        py,
        [
            "check_butterfly",
            "check_calendar_spread",
            "check_local_vol_density",
            "check_all",
        ],
    )?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core.market_data".to_string(),
        },
        Err(_) => "finstack.core.market_data".to_string(),
    };
    let qual = format!("{pkg}.arbitrage");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
