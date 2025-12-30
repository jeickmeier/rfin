//! Market context: aggregate curves, surfaces, FX, and scalars for pricing.
//!
//! Provides a Python-facing `MarketContext` that stores discount/forward/hazard/
//! inflation curves, volatility surfaces, FX matrices, scalars and time series.
//! Used by valuation routines to resolve dependencies by id. Insertion helpers
//! replace existing entries by id; retrieval raises `ValueError` when missing.
use super::bumps::PyMarketBump;
use super::dividends::PyDividendSchedule;
use super::fx::PyFxMatrix;
use super::scalars::{PyMarketScalar, PyScalarTimeSeries};
use super::surfaces::PyVolSurface;
use super::term_structures::{
    PyBaseCorrelationCurve, PyCreditIndexData, PyDiscountCurve, PyForwardCurve, PyHazardCurve,
    PyInflationCurve,
};
use crate::errors::core_to_py;
use finstack_core::market_data::context::{ContextStats, MarketContext};
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyList, PyModule};
use pyo3::Bound;

/// Aggregates curves, surfaces, FX matrices, and scalar data for pricing.
///
/// Parameters
/// ----------
/// None
///     Construct via :class:`MarketContext()` and populate using ``insert_*`` helpers.
///
/// Returns
/// -------
/// MarketContext
///     Mutable aggregation container shared across valuation routines.
#[pyclass(module = "finstack.core.market_data", name = "MarketContext")]
#[derive(Clone, Default)]
pub struct PyMarketContext {
    pub(crate) inner: MarketContext,
}

impl PyMarketContext {
    fn stats_to_dict(py: Python<'_>, stats: ContextStats) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("total_curves", stats.total_curves)?;
        dict.set_item("surface_count", stats.surface_count)?;
        dict.set_item("price_count", stats.price_count)?;
        dict.set_item("series_count", stats.series_count)?;
        dict.set_item("inflation_index_count", stats.inflation_index_count)?;
        dict.set_item("credit_index_count", stats.credit_index_count)?;
        dict.set_item("dividend_schedule_count", stats.dividend_schedule_count)?;
        dict.set_item("collateral_mapping_count", stats.collateral_mapping_count)?;
        dict.set_item("has_fx", stats.has_fx)?;

        let counts = PyDict::new(py);
        for (kind, value) in stats.curve_counts {
            counts.set_item(kind, value)?;
        }
        dict.set_item("curve_counts", counts)?;
        Ok(dict.into())
    }
}

#[pymethods]
impl PyMarketContext {
    #[new]
    #[pyo3(text_signature = "()")]
    /// Create an empty market context.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Empty context with no registered market objects.
    fn new() -> Self {
        Self {
            inner: MarketContext::new(),
        }
    }

    /// Produce a shallow clone of the context sharing underlying data.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Context referencing the same curves, surfaces, and scalars.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, bumps)")]
    /// Apply market bumps and return a new context with shifted data.
    ///
    /// Parameters
    /// ----------
    /// bumps : Sequence[MarketBump]
    ///     Iterable of :class:`MarketBump` objects describing the shifts.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     New context instance containing bumped market data.
    fn apply_bumps(&self, bumps: Bound<'_, PyAny>) -> PyResult<Self> {
        let iter = PyIterator::from_object(&bumps)?;
        let mut parsed = Vec::new();
        for item in iter {
            let obj = item?;
            let bump = obj
                .extract::<PyRef<PyMarketBump>>()
                .map_err(|_| PyTypeError::new_err("bumps must contain MarketBump objects"))?;
            parsed.push(bump.inner.clone());
        }
        let bumped = self.inner.bump(parsed).map_err(core_to_py)?;
        Ok(Self { inner: bumped })
    }

    #[pyo3(text_signature = "(self, curve)")]
    /// Insert or replace a discount curve by id.
    ///
    /// Parameters
    /// ----------
    /// curve : DiscountCurve
    ///     Curve to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_discount(&mut self, curve: &PyDiscountCurve) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_discount(curve.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, curve)")]
    /// Insert or replace a forward curve by id.
    ///
    /// Parameters
    /// ----------
    /// curve : ForwardCurve
    ///     Curve to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_forward(&mut self, curve: &PyForwardCurve) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_forward(curve.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, curve)")]
    /// Insert or replace a hazard (credit) curve by id.
    ///
    /// Parameters
    /// ----------
    /// curve : HazardCurve
    ///     Curve describing survival probabilities.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_hazard(&mut self, curve: &PyHazardCurve) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_hazard(curve.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, curve)")]
    /// Insert or replace an inflation curve by id.
    ///
    /// Parameters
    /// ----------
    /// curve : InflationCurve
    ///     Curve to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_inflation(&mut self, curve: &PyInflationCurve) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_inflation(curve.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, curve)")]
    /// Insert or replace a base-correlation curve for structured credit.
    ///
    /// Parameters
    /// ----------
    /// curve : BaseCorrelationCurve
    ///     Surface to store.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_base_correlation(&mut self, curve: &PyBaseCorrelationCurve) -> PyResult<()> {
        self.inner =
            std::mem::take(&mut self.inner).insert_base_correlation(curve.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, fx_matrix)")]
    /// Attach an FX matrix used for multi-currency conversions.
    ///
    /// Parameters
    /// ----------
    /// fx_matrix : FxMatrix
    ///     FX matrix to store.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_fx(&mut self, fx_matrix: &PyFxMatrix) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_fx_arc(fx_matrix.inner.clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, surface)")]
    /// Insert or replace a volatility surface by id.
    ///
    /// Parameters
    /// ----------
    /// surface : VolSurface
    ///     Surface to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_surface(&mut self, surface: &PyVolSurface) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_surface(surface.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, id, scalar)")]
    /// Store a market scalar (price, spread, etc.) under a string key.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Identifier for the scalar.
    /// scalar : MarketScalar
    ///     Scalar to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_price(&mut self, id: &str, scalar: &PyMarketScalar) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_price(id, scalar.inner.clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, series)")]
    /// Insert or replace a scalar time series.
    ///
    /// Parameters
    /// ----------
    /// series : ScalarTimeSeries
    ///     Series to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_series(&mut self, series: &PyScalarTimeSeries) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).insert_series(series.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, schedule)")]
    /// Register a dividend schedule for an equity id.
    ///
    /// Parameters
    /// ----------
    /// schedule : DividendSchedule
    ///     Schedule to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_dividends(&mut self, schedule: &PyDividendSchedule) -> PyResult<()> {
        self.inner =
            std::mem::take(&mut self.inner).insert_dividends(schedule.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, id, data)")]
    /// Insert aggregated credit index market data.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Credit index identifier.
    /// data : CreditIndexData
    ///     Data bundle to register.
    ///
    /// Returns
    /// -------
    /// None
    fn insert_credit_index(&mut self, id: &str, data: &PyCreditIndexData) -> PyResult<()> {
        self.inner =
            std::mem::take(&mut self.inner).insert_credit_index(id, data.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, csa_code, curve_id)")]
    /// Map a CSA/collateral code to a discount curve id.
    ///
    /// Parameters
    /// ----------
    /// csa_code : str
    ///     CSA or collateral identifier.
    /// curve_id : str
    ///     Discount curve id used for collateral discounting.
    ///
    /// Returns
    /// -------
    /// None
    fn map_collateral(&mut self, csa_code: &str, curve_id: &str) -> PyResult<()> {
        self.inner =
            std::mem::take(&mut self.inner).map_collateral(csa_code, CurveId::from(curve_id));
        Ok(())
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a discount curve by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Discount curve identifier.
    ///
    /// Returns
    /// -------
    /// DiscountCurve
    ///     Requested discount curve.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve is not present.
    fn discount(&self, id: &str) -> PyResult<PyDiscountCurve> {
        let arc = self.inner.get_discount(id).map_err(core_to_py)?;
        Ok(PyDiscountCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a forward curve by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Forward curve identifier.
    ///
    /// Returns
    /// -------
    /// ForwardCurve
    ///     Requested forward curve.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve is not present.
    fn forward(&self, id: &str) -> PyResult<PyForwardCurve> {
        let arc = self.inner.get_forward(id).map_err(core_to_py)?;
        Ok(PyForwardCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a hazard curve by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Hazard curve identifier.
    ///
    /// Returns
    /// -------
    /// HazardCurve
    ///     Requested hazard curve.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve is not present.
    fn hazard(&self, id: &str) -> PyResult<PyHazardCurve> {
        let arc = self.inner.get_hazard(id).map_err(core_to_py)?;
        Ok(PyHazardCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve an inflation curve by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Inflation curve identifier.
    ///
    /// Returns
    /// -------
    /// InflationCurve
    ///     Requested inflation curve.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve is not present.
    fn inflation(&self, id: &str) -> PyResult<PyInflationCurve> {
        let arc = self.inner.get_inflation(id).map_err(core_to_py)?;
        Ok(PyInflationCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a base-correlation curve by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Base-correlation curve identifier.
    ///
    /// Returns
    /// -------
    /// BaseCorrelationCurve
    ///     Requested base-correlation curve.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve is not present.
    fn base_correlation(&self, id: &str) -> PyResult<PyBaseCorrelationCurve> {
        let arc = self.inner.get_base_correlation(id).map_err(core_to_py)?;
        Ok(PyBaseCorrelationCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a volatility surface by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Surface identifier.
    ///
    /// Returns
    /// -------
    /// VolSurface
    ///     Requested volatility surface.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the surface is not present.
    fn surface(&self, id: &str) -> PyResult<PyVolSurface> {
        let arc = self.inner.surface(id).map_err(core_to_py)?;
        Ok(PyVolSurface::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a scalar price by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Scalar identifier.
    ///
    /// Returns
    /// -------
    /// MarketScalar
    ///     Requested scalar value.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the price is not present.
    fn price(&self, id: &str) -> PyResult<PyMarketScalar> {
        let scalar = self.inner.price(id).map_err(core_to_py)?;
        Ok(PyMarketScalar::new(scalar.clone()))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve a scalar time series by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Series identifier.
    ///
    /// Returns
    /// -------
    /// ScalarTimeSeries
    ///     Requested series.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the series is not present.
    fn series(&self, id: &str) -> PyResult<PyScalarTimeSeries> {
        let series = self.inner.series(id).map_err(core_to_py)?;
        Ok(PyScalarTimeSeries::new_arc(series.clone()))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Retrieve credit index data by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Credit index identifier.
    ///
    /// Returns
    /// -------
    /// CreditIndexData
    ///     Credit index data bundle.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the index is not present.
    fn credit_index(&self, id: &str) -> PyResult<PyCreditIndexData> {
        let data = self.inner.credit_index(id).map_err(core_to_py)?;
        Ok(PyCreditIndexData::new_arc(data))
    }

    #[pyo3(text_signature = "(self, id)")]
    /// Look up a dividend schedule by id.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Dividend schedule identifier.
    ///
    /// Returns
    /// -------
    /// DividendSchedule or None
    ///     Dividend schedule if found, otherwise ``None``.
    fn dividend_schedule(&self, id: &str) -> Option<PyDividendSchedule> {
        self.inner
            .dividend_schedule(id)
            .map(|schedule| PyDividendSchedule::new(schedule.as_ref().clone()))
    }

    /// All curve identifiers stored in the context.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     All curve ids currently registered.
    fn curve_ids(&self) -> Vec<String> {
        self.inner.curve_ids().map(|id| id.to_string()).collect()
    }

    #[pyo3(text_signature = "(self, curve_type)")]
    /// Curve identifiers filtered by type label (e.g. ``"discount"``).
    ///
    /// Parameters
    /// ----------
    /// curve_type : str
    ///     Curve category (``"discount"``, ``"forward"``, etc.).
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Curve ids matching ``curve_type``.
    fn curve_ids_by_type(&self, curve_type: &str) -> Vec<String> {
        self.inner
            .curves_of_type(curve_type)
            .map(|(id, _)| id.to_string())
            .collect()
    }

    /// Count of curves keyed by type.
    ///
    /// Returns
    /// -------
    /// dict[str, int]
    ///     Mapping of curve type to count.
    fn count_by_type(&self) -> HashMap<String, usize> {
        self.inner
            .count_by_type()
            .into_iter()
            .map(|(kind, value)| (kind.to_string(), value))
            .collect()
    }

    /// Snapshot of context statistics as a dictionary.
    ///
    /// Returns
    /// -------
    /// dict[str, Any]
    ///     Dictionary containing counts for curves, surfaces, prices, and FX.
    fn stats(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let snapshot = self.inner.stats();
        Self::stats_to_dict(py, snapshot)
    }

    /// Whether the context contains no market objects.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` if no curves, surfaces, or scalars are registered.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Total number of stored market objects (curves, surfaces, scalars, etc.).
    ///
    /// Returns
    /// -------
    /// int
    ///     Total population of the context.
    fn total_objects(&self) -> usize {
        self.inner.total_objects()
    }

    /// ``True`` if an FX matrix has been configured.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` when :meth:`insert_fx` has been called with a matrix.
    fn has_fx(&self) -> bool {
        self.inner.fx().is_some()
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "MarketContext(curves={}, fx={}, surfaces={})",
            stats.total_curves, stats.has_fx, stats.surface_count
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "context")?;
    module.setattr(
        "__doc__",
        "Aggregation container for market data (curves, surfaces, FX, and scalars).",
    )?;
    module.add_class::<PyMarketContext>()?;
    let exports = ["MarketContext"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    for name in exports {
        let attr = module.getattr(name)?;
        parent.setattr(name, attr)?;
    }
    Ok(exports.to_vec())
}
