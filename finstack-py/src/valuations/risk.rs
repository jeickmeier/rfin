//! Python bindings for risk ladder calculations (KRD, CS01, etc.).
//!
//! Exposes bucketed risk metrics in DataFrame-friendly format.

use crate::core::market_data::PyMarketContext;
use crate::core::utils as core_utils;
use finstack_core::market_data::bumps::BumpSpec;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::standard_ir_dv01_buckets;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

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
    // Convert Python types to Rust types
    let as_of_date = core_utils::py_to_date(&as_of)?;
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);
    let bump = bump_bp.unwrap_or(1.0);

    // Extract instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let instrument: Arc<dyn Instrument> = Arc::from(bond_handle.instrument);

    // Call Rust helper function
    let ladder = compute_key_rate_dv01_ladder(&instrument, &market.inner, as_of_date, &buckets, bump)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    // Convert to Python dict format
    let bucket_labels: Vec<String> = ladder.iter().map(|(label, _)| label.clone()).collect();
    let dv01_values: Vec<f64> = ladder.iter().map(|(_, value)| *value).collect();

    let dict = PyDict::new(py);
    dict.set_item("bucket", bucket_labels)?;
    dict.set_item("dv01", dv01_values)?;

    Ok(dict.into())
}

/// Compute key-rate DV01 ladder using Rust bumping infrastructure.
///
/// This is a type-erased helper that works with trait objects.
fn compute_key_rate_dv01_ladder(
    instrument: &Arc<dyn Instrument>,
    market: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    buckets: &[f64],
    bump_bp: f64,
) -> finstack_core::Result<Vec<(String, f64)>> {
    use finstack_core::types::CurveId;
    use hashbrown::HashMap;
    
    // Calculate base PV
    let base_pv = instrument.value(market, as_of)?;

    // Collect all discount curves from market
    let discount_curve_ids: Vec<CurveId> = market
        .curves_of_type("Discount")
        .map(|(id, _)| id.clone())
        .collect();

    if discount_curve_ids.is_empty() {
        return Ok(vec![]);
    }

    // Calculate DV01 for each bucket
    let mut results = Vec::new();
    
    for &time_years in buckets {
        let label = format_bucket_label(time_years);
        
        // Create key-rate bump for all discount curves
        let mut bumps = HashMap::new();
        for curve_id in &discount_curve_ids {
            bumps.insert(curve_id.clone(), BumpSpec::key_rate_bp(time_years, bump_bp));
        }
        
        // Apply bumps and reprice
        let bumped_market = market.bump(bumps)?;
        let bumped_pv = instrument.value(&bumped_market, as_of)?;
        
        // Calculate sensitivity
        let dv01 = (bumped_pv.amount() - base_pv.amount()) / bump_bp;
        
        results.push((label, dv01));
    }

    Ok(results)
}

/// Generate bucket label from years (e.g., 0.25 -> "3m", 5.0 -> "5y").
fn format_bucket_label(years: f64) -> String {
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
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
/// * `buckets_years` - Optional list of tenor points in years
/// * `bump_bp` - Parallel shift size in basis points (default: 1.0)
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
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);
    let bump = bump_bp.unwrap_or(1.0);

    // Extract instrument
    let bond_handle = super::instruments::extract_instrument(&bond)?;
    let instrument: Arc<dyn Instrument> = Arc::from(bond_handle.instrument);

    // Call Rust helper function
    // TODO: Implement proper credit curve bumping when hazard rate infrastructure is ready
    let ladder = compute_key_rate_dv01_ladder(&instrument, &market.inner, as_of_date, &buckets, bump)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    // Convert to Python dict format
    let bucket_labels: Vec<String> = ladder.iter().map(|(label, _)| label.clone()).collect();
    let cs01_values: Vec<f64> = ladder.iter().map(|(_, value)| *value).collect();

    let dict = PyDict::new(py);
    dict.set_item("bucket", bucket_labels)?;
    dict.set_item("cs01", cs01_values)?;

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
