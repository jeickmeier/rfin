//! Python bindings for risk ladder calculations (KRD, CS01, etc.).
//!
//! Exposes bucketed risk metrics in DataFrame-friendly format.

use crate::core::market_data::PyMarketContext;
use crate::core::utils as core_utils;
use finstack_valuations::metrics::standard_ir_dv01_buckets;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
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
/// * `buckets_years` - Optional list of tenor points in years (default: standard buckets)
/// * `bump_bp` - Parallel shift size in basis points (default: 1.0)
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
pub fn krd_dv01_ladder(
    py: Python<'_>,
    bond: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> PyResult<PyObject> {
    let as_of_date = core_utils::py_to_date(&as_of)?;
    let bump = bump_bp.unwrap_or(1.0);
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);

    // Extract bond instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let bond_inst = bond_handle.instrument;

    // Price bond at base case
    let base_pv = bond_inst.value(&market.inner, as_of_date)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    // Get discount curve - use first discount curve in market (simplified)
    let mut disc_iter = market.inner.curves_of_type("Discount");
    let (_, disc_storage) = disc_iter.next().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "No discount curves available in market context"
        )
    })?;
    let disc = disc_storage.discount().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to extract discount curve")
    })?;

    // Compute DV01 for each bucket
    let mut bucket_labels = Vec::new();
    let mut dv01_values = Vec::new();

    for t in buckets {
        let label = if t < 1.0 {
            format!("{}m", (t * 12.0).round() as i32)
        } else {
            format!("{}y", t as i32)
        };

        // Bump curve at this key rate
        let bumped = disc.with_key_rate_bump_years(t, bump);
        let temp_market = market.inner.clone().insert_discount(bumped);

        // Revalue with bumped curve
        let pv_bumped = bond_inst.value(&temp_market, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // DV01 = (PV_bumped - PV_base) / 10000
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;

        bucket_labels.push(label);
        dv01_values.push(dv01);
    }

    // Return as dict that can be easily converted to DataFrame
    let dict = PyDict::new(py);
    dict.set_item("bucket", bucket_labels)?;
    dict.set_item("dv01", dv01_values)?;

    Ok(dict.into())
}

/// Compute CS01 ladder for a bond.
///
/// Similar to KRD but for credit spread sensitivity.
///
/// # Arguments
///
/// * `bond` - Bond instrument to analyze
/// * `market` - Market context with discount curve
/// * `as_of` - Valuation date
/// * `buckets_years` - Optional list of tenor points in years
/// * `bump_bp` - Parallel shift size in basis points (default: 1.0)
///
/// # Returns
///
/// Dictionary with 'buckets' and 'cs01' keys for DataFrame conversion.
#[pyfunction]
#[pyo3(signature = (bond, market, as_of, buckets_years=None, bump_bp=None))]
pub fn cs01_ladder(
    py: Python<'_>,
    bond: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> PyResult<PyObject> {
    let as_of_date = core_utils::py_to_date(&as_of)?;
    let bump = bump_bp.unwrap_or(1.0);
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);

    // Extract bond instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let bond_inst = bond_handle.instrument;

    // Price bond at base case
    let base_pv = bond_inst.value(&market.inner, as_of_date)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    // Get discount curve (CS01 uses discount curve as proxy for credit spread)
    let mut disc_iter = market.inner.curves_of_type("Discount");
    let (_, disc_storage) = disc_iter.next().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "No discount curves available in market context"
        )
    })?;
    let disc = disc_storage.discount().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to extract discount curve")
    })?;

    // Compute CS01 for each bucket (simplified: same as DV01 for now)
    let mut bucket_labels = Vec::new();
    let mut cs01_values = Vec::new();

    for t in buckets {
        let label = if t < 1.0 {
            format!("{}m", (t * 12.0).round() as i32)
        } else {
            format!("{}y", t as i32)
        };

        // Bump curve at this key rate
        let bumped = disc.with_key_rate_bump_years(t, bump);
        let temp_market = market.inner.clone().insert_discount(bumped);

        // Revalue
        let pv_bumped = bond_inst.value(&temp_market, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // CS01 = (PV_bumped - PV_base) / 10000
        let cs01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;

        bucket_labels.push(label);
        cs01_values.push(cs01);
    }

    // Return as dict
    let dict = PyDict::new(py);
    dict.set_item("bucket", bucket_labels)?;
    dict.set_item("cs01", cs01_values)?;

    Ok(dict.into())
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(krd_dv01_ladder, module)?)?;
    module.add_function(wrap_pyfunction!(cs01_ladder, module)?)?;
    Ok(vec!["krd_dv01_ladder", "cs01_ladder"])
}

