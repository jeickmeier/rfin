//! Python bindings for Calibration Bump Helpers.
//!
//! Provides curve bumping utilities for scenario analysis and sensitivity calculations.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::term_structures::{PyDiscountCurve, PyHazardCurve, PyInflationCurve};
use crate::errors::core_to_py;
use crate::statements::utils::py_to_json;
use crate::valuations::calibration::quote::PyRatesQuote;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::api::schema::DiscountCurveParams;
use finstack_valuations::calibration::bumps::{
    bump_discount_curve, bump_discount_curve_synthetic, bump_hazard_shift, bump_hazard_spreads,
    bump_inflation_rates, BumpRequest,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

// =============================================================================
// BumpRequest Enum
// =============================================================================

/// Request for a curve bump operation.
///
/// Create using static methods `parallel()` or `tenors()`.
///
/// Examples:
///     >>> # Parallel +10bp bump
///     >>> bump = BumpRequest.parallel(10.0)
///
///     >>> # Key-rate bumps at specific tenors
///     >>> bump = BumpRequest.tenors([(2.0, 5.0), (5.0, 10.0), (10.0, 15.0)])
#[pyclass(
    module = "finstack.valuations.bumps",
    name = "BumpRequest",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBumpRequest {
    pub(crate) inner: BumpRequest,
}

#[pymethods]
impl PyBumpRequest {
    /// Create a parallel bump (shift all rates by the same amount).
    ///
    /// Args:
    ///     bp: Bump size in basis points. Positive = rates up, negative = rates down.
    ///
    /// Returns:
    ///     BumpRequest: A parallel bump request.
    ///
    /// Examples:
    ///     >>> bump = BumpRequest.parallel(10.0)  # +10bp parallel shift
    #[staticmethod]
    fn parallel(bp: f64) -> Self {
        Self {
            inner: BumpRequest::Parallel(bp),
        }
    }

    /// Create a tenor-specific (key-rate) bump.
    ///
    /// Args:
    ///     tenors: List of (tenor_years, bump_bp) tuples. Each tuple specifies
    ///         a maturity in years and the bump size in basis points.
    ///
    /// Returns:
    ///     BumpRequest: A tenor-specific bump request.
    ///
    /// Examples:
    ///     >>> # Bump 2Y by +5bp, 5Y by +10bp, 10Y by +15bp
    ///     >>> bump = BumpRequest.tenors([(2.0, 5.0), (5.0, 10.0), (10.0, 15.0)])
    #[staticmethod]
    fn tenors(tenors: Vec<(f64, f64)>) -> PyResult<Self> {
        if tenors.is_empty() {
            return Err(PyValueError::new_err(
                "tenors list must have at least one element",
            ));
        }
        Ok(Self {
            inner: BumpRequest::Tenors(tenors),
        })
    }

    /// Check if this is a parallel bump.
    fn is_parallel(&self) -> bool {
        matches!(self.inner, BumpRequest::Parallel(_))
    }

    /// Check if this is a tenor-specific bump.
    fn is_tenors(&self) -> bool {
        matches!(self.inner, BumpRequest::Tenors(_))
    }

    /// Get the parallel bump size (if parallel), or None.
    fn parallel_bp(&self) -> Option<f64> {
        match &self.inner {
            BumpRequest::Parallel(bp) => Some(*bp),
            BumpRequest::Tenors(_) => None,
        }
    }

    /// Get the tenor bumps (if tenors), or None.
    fn tenor_bumps<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
        match &self.inner {
            BumpRequest::Parallel(_) => Ok(None),
            BumpRequest::Tenors(tenors) => {
                let list = PyList::new(py, tenors.iter().map(|(t, bp)| (*t, *bp)))?;
                Ok(Some(list))
            }
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            BumpRequest::Parallel(bp) => format!("BumpRequest.parallel({:.2})", bp),
            BumpRequest::Tenors(tenors) => format!("BumpRequest.tenors({:?})", tenors),
        }
    }
}

// =============================================================================
// Bump Functions
// =============================================================================

/// Bump a discount curve by synthesizing quotes and re-calibrating.
///
/// This function extracts par rates from the current curve, applies the bump,
/// and re-calibrates. Use when original quotes are unavailable.
///
/// Args:
///     curve: The discount curve to bump.
///     market: Market context containing the curve.
///     bump: The bump request (parallel or tenor-specific).
///     as_of: Valuation date.
///
/// Returns:
///     DiscountCurve: A new bumped discount curve.
///
/// Examples:
///     >>> bumped = bump_discount_curve_synthetic(
///     ...     curve=discount_curve,
///     ...     market=market_context,
///     ...     bump=BumpRequest.parallel(10.0),
///     ...     as_of=(2025, 1, 1)
///     ... )
#[pyfunction]
#[pyo3(name = "bump_discount_curve_synthetic", signature = (curve, market, bump, as_of))]
fn py_bump_discount_curve_synthetic(
    py: Python<'_>,
    curve: &PyDiscountCurve,
    market: &PyMarketContext,
    bump: &PyBumpRequest,
    as_of: &Bound<'_, PyAny>,
) -> PyResult<PyDiscountCurve> {
    let as_of_date = py_to_date(as_of)?;
    let curve_inner = curve.inner.clone();
    let market_inner = market.inner.clone();
    let bump_inner = bump.inner.clone();
    let bumped = py
        .detach(|| {
            bump_discount_curve_synthetic(
                &curve_inner,
                &market_inner,
                &bump_inner,
                as_of_date,
                None,
            )
        })
        .map_err(core_to_py)?;
    Ok(PyDiscountCurve::new_arc(Arc::new(bumped)))
}

/// Bump a hazard curve by re-calibrating from par spreads.
///
/// This function extracts par spreads, applies the bump, and re-calibrates
/// the hazard curve. Requires a discount curve for discounting.
///
/// Args:
///     hazard_curve: The hazard curve to bump.
///     market: Market context containing necessary curves.
///     bump: The bump request (parallel or tenor-specific).
///     discount_id: Identifier for the discount curve to use.
///
/// Returns:
///     HazardCurve: A new bumped hazard curve.
///
/// Examples:
///     >>> bumped = bump_hazard_spreads(
///     ...     hazard_curve=hazard,
///     ...     market=market_context,
///     ...     bump=BumpRequest.parallel(50.0),  # +50bp spread
///     ...     discount_id="USD-OIS"
///     ... )
#[pyfunction]
#[pyo3(name = "bump_hazard_spreads", signature = (hazard_curve, market, bump, discount_id))]
fn py_bump_hazard_spreads(
    py: Python<'_>,
    hazard_curve: &PyHazardCurve,
    market: &PyMarketContext,
    bump: &PyBumpRequest,
    discount_id: &str,
) -> PyResult<PyHazardCurve> {
    let discount_curve_id = CurveId::new(discount_id);
    let hazard_inner = hazard_curve.inner.clone();
    let market_inner = market.inner.clone();
    let bump_inner = bump.inner.clone();
    let bumped = py
        .detach(|| {
            bump_hazard_spreads(
                &hazard_inner,
                &market_inner,
                &bump_inner,
                Some(&discount_curve_id),
            )
        })
        .map_err(core_to_py)?;
    Ok(PyHazardCurve::new_arc(Arc::new(bumped)))
}

/// Bump a hazard curve directly without re-calibration.
///
/// This function applies a direct shift to hazard rates without re-calibrating
/// from par spreads. Faster but less accurate for large bumps.
///
/// Args:
///     hazard_curve: The hazard curve to bump.
///     bump: The bump request (parallel or tenor-specific).
///
/// Returns:
///     HazardCurve: A new bumped hazard curve.
///
/// Examples:
///     >>> bumped = bump_hazard_shift(
///     ...     hazard_curve=hazard,
///     ...     bump=BumpRequest.parallel(10.0)  # +10bp hazard rate
///     ... )
#[pyfunction]
#[pyo3(name = "bump_hazard_shift", signature = (hazard_curve, bump))]
fn py_bump_hazard_shift(
    py: Python<'_>,
    hazard_curve: &PyHazardCurve,
    bump: &PyBumpRequest,
) -> PyResult<PyHazardCurve> {
    let hazard_inner = hazard_curve.inner.clone();
    let bump_inner = bump.inner.clone();
    let bumped = py
        .detach(|| bump_hazard_shift(&hazard_inner, &bump_inner))
        .map_err(core_to_py)?;
    Ok(PyHazardCurve::new_arc(Arc::new(bumped)))
}

/// Bump an inflation curve by re-calibrating from implied rates.
///
/// This function extracts implied zero-coupon swap rates, applies the bump,
/// and re-calibrates the inflation curve.
///
/// Args:
///     curve: The inflation curve to bump.
///     market: Market context containing necessary curves.
///     bump: The bump request (parallel or tenor-specific).
///     discount_id: Identifier for the discount curve to use.
///     as_of: Valuation date.
///
/// Returns:
///     InflationCurve: A new bumped inflation curve.
///
/// Examples:
///     >>> bumped = bump_inflation_rates(
///     ...     curve=inflation_curve,
///     ...     market=market_context,
///     ...     bump=BumpRequest.parallel(25.0),  # +25bp inflation
///     ...     discount_id="USD-OIS",
///     ...     as_of=(2025, 1, 1)
///     ... )
#[pyfunction]
#[pyo3(name = "bump_inflation_rates", signature = (curve, market, bump, discount_id, as_of))]
fn py_bump_inflation_rates(
    py: Python<'_>,
    curve: &PyInflationCurve,
    market: &PyMarketContext,
    bump: &PyBumpRequest,
    discount_id: &str,
    as_of: &Bound<'_, PyAny>,
) -> PyResult<PyInflationCurve> {
    let as_of_date = py_to_date(as_of)?;
    let discount_curve_id = CurveId::new(discount_id);
    let curve_inner = curve.inner.clone();
    let market_inner = market.inner.clone();
    let bump_inner = bump.inner.clone();
    let bumped = py
        .detach(|| {
            bump_inflation_rates(
                &curve_inner,
                &market_inner,
                &bump_inner,
                &discount_curve_id,
                as_of_date,
            )
        })
        .map_err(core_to_py)?;
    Ok(PyInflationCurve::new_arc(Arc::new(bumped)))
}

/// Bump a discount curve by shocking rate quotes and re-calibrating.
///
/// This applies the bump to the provided rate quotes, then re-executes
/// the calibration step to produce a new discount curve.
///
/// Args:
///     quotes: List of RatesQuote objects used in the original calibration.
///     params: Dict matching the DiscountCurveParams calibration schema.
///     market: Market context providing any required dependencies.
///     bump: The bump request (parallel or tenor-specific).
///
/// Returns:
///     DiscountCurve: A new bumped discount curve.
///
/// Examples:
///     >>> bumped = bump_discount_curve(
///     ...     quotes=rates_quotes,
///     ...     params={"curve_id": "USD-OIS", "currency": "USD", "base_date": "2025-01-01", "method": "bootstrap"},
///     ...     market=market_context,
///     ...     bump=BumpRequest.parallel(10.0)
///     ... )
#[pyfunction]
#[pyo3(name = "bump_discount_curve", signature = (quotes, params, market, bump))]
fn py_bump_discount_curve(
    py: Python<'_>,
    quotes: Vec<PyRatesQuote>,
    params: &Bound<'_, pyo3::types::PyAny>,
    market: &PyMarketContext,
    bump: &PyBumpRequest,
) -> PyResult<PyDiscountCurve> {
    let rust_quotes: Vec<_> = quotes.iter().map(|q| q.inner.clone()).collect();
    let json_value = py_to_json(params)?;
    let disc_params: DiscountCurveParams = serde_json::from_value(json_value).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid DiscountCurveParams dict: {e}"))
    })?;
    let bumped = py
        .detach(|| bump_discount_curve(&rust_quotes, &disc_params, &market.inner, &bump.inner))
        .map_err(core_to_py)?;
    Ok(PyDiscountCurve::new_arc(Arc::new(bumped)))
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "bumps")?;
    module.setattr(
        "__doc__",
        "Curve bumping utilities for scenario analysis and sensitivity calculations.",
    )?;

    // Add classes
    module.add_class::<PyBumpRequest>()?;

    // Add functions
    module.add_function(wrap_pyfunction!(py_bump_discount_curve, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bump_discount_curve_synthetic, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bump_hazard_spreads, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bump_hazard_shift, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bump_inflation_rates, &module)?)?;

    let exports = vec![
        "BumpRequest",
        "bump_discount_curve",
        "bump_discount_curve_synthetic",
        "bump_hazard_spreads",
        "bump_hazard_shift",
        "bump_inflation_rates",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
