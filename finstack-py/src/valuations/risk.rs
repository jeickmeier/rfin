//! Python bindings for risk ladder calculations (KRD, CS01, etc.).
//!
//! Exposes risk ladder calculations by delegating to Rust `finstack-valuations`
//! metrics (no pricing logic implemented in Python).

use crate::core::market_data::PyMarketContext;
use crate::core::utils as core_utils;
use finstack_valuations::metrics::MetricId;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;

/// Compute Key Rate Duration (KRD) DV01 ladder for a bond.
///
/// Returns a dictionary with bucket labels and DV01 values that can be
/// converted to a Polars/Pandas DataFrame.
///
/// # Arguments
///
/// * `bond` - Bond instrument to analyze
/// * `market` - Market context with discount curve
/// * `as_of` - Valuation date  
/// * `buckets_years` - Optional list of tenor points in years (currently ignored; bucket
///   configuration is controlled in the Rust metrics layer).
/// * `bump_bp` - Parallel shift size in basis points (currently ignored; bump size is
///   controlled via pricing overrides in the Rust metrics layer).
///
/// # Returns
///
/// Dictionary with 'buckets' (list of str) and 'dv01' (list of float) keys.
///
/// # Example (Python)
///
/// ```python
/// import polars as pl
/// from finstack.valuations import krd_dv01_ladder
/// from datetime import date
///
/// ladder = krd_dv01_ladder(bond, market, date(2025, 1, 1))
/// df = pl.DataFrame(ladder)
/// print(df)
/// # shape: (11, 2)
/// # ┌────────┬──────────┐
/// # │ bucket │ dv01     │
/// # │ ---    │ ---      │
/// # │ str    │ f64      │
/// # ╞════════╪══════════╡
/// # │ 3m     │ 12.34    │
/// # │ 6m     │ 23.45    │
/// # │ ...    │ ...      │
/// # └────────┴──────────┘
/// ```
#[pyfunction]
#[pyo3(signature = (bond, market, as_of, buckets_years=None, bump_bp=None))]
#[allow(unused_variables)]
pub fn krd_dv01_ladder(
    py: Python<'_>,
    bond: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> PyResult<PyObject> {
    // Convert Python types to Rust types
    let as_of_date = core_utils::py_to_date(&as_of)?;

    // Extract instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let instrument = bond_handle.instrument;

    // Use registered BucketedDv01 metric in Rust; this currently returns a single
    // scalar representing the total bucketed DV01. Full per-bucket ladders are
    // computed inside the metrics layer using `MetricContext`.
    let metrics = vec![MetricId::BucketedDv01];
    let result = instrument
        .price_with_metrics(&market.inner, as_of_date, &metrics)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let total = *result
        .measures
        .get(MetricId::BucketedDv01.as_str())
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bucketed DV01 metric not available for this instrument",
            )
        })?;

    // For now, expose a single 'total' bucket; when a Rust API for per-bucket
    // ladders is available we can extend this without changing the Python shape.
    let dict = PyDict::new(py);
    dict.set_item("buckets", vec!["total"])?;
    dict.set_item("dv01", vec![total])?;

    Ok(dict.into())
}

/// Compute CS01 ladder for a bond.
///
/// Similar to KRD but for credit spread sensitivity.
///
/// # Arguments
///
/// * `bond` - Bond instrument to analyze
/// * `market` - Market context with credit/hazard curves
/// * `as_of` - Valuation date
/// * `buckets_years` - Optional list of tenor points in years (currently ignored).
/// * `bump_bp` - Parallel shift size in basis points (currently ignored).
///
/// # Returns
///
/// Dictionary with 'buckets' and 'cs01' keys for DataFrame conversion.
///
/// # Note
///
/// Currently uses discount curve as a proxy for credit spreads. Proper CS01
/// calculation requires dedicated credit curve infrastructure (hazard rates).
#[pyfunction]
#[pyo3(signature = (bond, market, as_of, buckets_years=None, bump_bp=None))]
#[allow(unused_variables)]
pub fn cs01_ladder(
    py: Python<'_>,
    bond: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> PyResult<PyObject> {
    // Convert Python types to Rust types
    let as_of_date = core_utils::py_to_date(&as_of)?;

    // Extract instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let instrument = bond_handle.instrument;

    // Use registered BucketedCs01 metric in Rust; as with DV01, this currently
    // exposes a total scalar at the ValuationResult level.
    let metrics = vec![MetricId::BucketedCs01];
    let result = instrument
        .price_with_metrics(&market.inner, as_of_date, &metrics)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let total = *result
        .measures
        .get(MetricId::BucketedCs01.as_str())
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bucketed CS01 metric not available for this instrument",
            )
        })?;

    let dict = PyDict::new(py);
    dict.set_item("buckets", vec!["total"])?;
    dict.set_item("cs01", vec![total])?;

    Ok(dict.into())
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "risk")?;
    module.setattr(
        "__doc__",
        "Risk ladder calculations (KRD DV01, CS01) for bonds and credit instruments.",
    )?;
    module.add_function(wrap_pyfunction!(krd_dv01_ladder, &module)?)?;
    module.add_function(wrap_pyfunction!(cs01_ladder, &module)?)?;
    let exports = vec!["krd_dv01_ladder", "cs01_ladder"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
