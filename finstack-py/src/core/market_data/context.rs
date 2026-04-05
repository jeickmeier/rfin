//! Market context: aggregate curves, surfaces, FX, and scalars for pricing.
//!
//! Provides a Python-facing `MarketContext` that stores discount/forward/hazard/
//! inflation curves, volatility surfaces, FX matrices, scalars and time series.
//! Used by valuation routines to resolve dependencies by id. Insertion helpers
//! replace existing entries by id; retrieval raises `ValueError` when missing.
use super::bumps::PyMarketBump;
use super::dividends::PyDividendSchedule;
use super::fx::PyFxMatrix;
use super::scalars::{PyInflationIndex, PyMarketScalar, PyScalarTimeSeries};
use super::surfaces::{PyFxDeltaVolSurface, PyVolSurface};
use super::term_structures::{
    PyBaseCorrelationCurve, PyCreditIndexData, PyDiscountCurve, PyForwardCurve, PyHazardCurve,
    PyInflationCurve, PyPriceCurve, PyVolatilityIndexCurve,
};
use crate::errors::core_to_py;
use finstack_core::market_data::context::{
    ContextStats, CurveStorage, MarketContext, MarketContextState,
};
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyIterator, PyList, PyModule, PyType};
use pyo3::Bound;
use pythonize::{depythonize, pythonize};

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
#[pyclass(
    module = "finstack.core.market_data",
    name = "MarketContext",
    from_py_object
)]
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

    fn insert_curve_like(&mut self, curve: Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(curve) = curve.extract::<PyRef<'_, PyDiscountCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyForwardCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyHazardCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyInflationCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyBaseCorrelationCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyPriceCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }
        if let Ok(curve) = curve.extract::<PyRef<'_, PyVolatilityIndexCurve>>() {
            self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
            return Ok(());
        }

        Err(PyTypeError::new_err(
            "insert() expects a curve type supported by MarketContext",
        ))
    }

    fn wrap_curve_storage(py: Python<'_>, storage: &CurveStorage) -> PyResult<Py<PyAny>> {
        match storage {
            CurveStorage::Discount(curve) => {
                Py::new(py, PyDiscountCurve::new_arc(curve.clone())).map(|obj| obj.into_any())
            }
            CurveStorage::Forward(curve) => {
                Py::new(py, PyForwardCurve::new_arc(curve.clone())).map(|obj| obj.into_any())
            }
            CurveStorage::Hazard(curve) => {
                Py::new(py, PyHazardCurve::new_arc(curve.clone())).map(|obj| obj.into_any())
            }
            CurveStorage::Inflation(curve) => {
                Py::new(py, PyInflationCurve::new_arc(curve.clone())).map(|obj| obj.into_any())
            }
            CurveStorage::BaseCorrelation(curve) => {
                Py::new(py, PyBaseCorrelationCurve::new_arc(curve.clone()))
                    .map(|obj| obj.into_any())
            }
            CurveStorage::Price(curve) => {
                Py::new(py, PyPriceCurve::new_arc(curve.clone())).map(|obj| obj.into_any())
            }
            CurveStorage::VolIndex(curve) => {
                Py::new(py, PyVolatilityIndexCurve::new_arc(curve.clone()))
                    .map(|obj| obj.into_any())
            }
            // BasisSpread and Parametric curves are not yet exposed to Python
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "this curve type is not yet available in the Python bindings",
            )),
        }
    }

    fn dict_to_market_context(data: &Bound<'_, PyAny>) -> PyResult<MarketContext> {
        depythonize(data).map_err(|e| {
            PyValueError::new_err(format!("failed to convert MarketContext data: {e}"))
        })
    }
}

pub(crate) fn market_context_from_value(value: &Bound<'_, PyAny>) -> PyResult<PyMarketContext> {
    if let Ok(context) = value.extract::<PyRef<'_, PyMarketContext>>() {
        return Ok(context.clone());
    }

    if let Ok(payload) = value.extract::<&str>() {
        return serde_json::from_str(payload)
            .map(|inner| PyMarketContext { inner })
            .map_err(|e| {
                PyValueError::new_err(format!("failed to deserialize MarketContext: {e}"))
            });
    }

    PyMarketContext::dict_to_market_context(value).map(|inner| PyMarketContext { inner })
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

    /// Serialize the market context to a Python dictionary.
    ///
    /// Returns
    /// -------
    /// dict[str, Any]
    ///     Versioned, JSON-compatible market snapshot.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        pythonize(py, &self.inner)
            .map(|obj| obj.unbind())
            .map_err(|e| {
                PyValueError::new_err(format!("failed to convert MarketContext to dict: {e}"))
            })
    }

    /// Serialize the market context to a JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     Versioned market snapshot JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("failed to serialize MarketContext: {e}")))
    }

    /// Construct a market context from a Python dictionary.
    ///
    /// Parameters
    /// ----------
    /// data : dict[str, Any]
    ///     Versioned market snapshot dictionary.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Restored market context.
    #[classmethod]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Self::dict_to_market_context(data).map(|inner| Self { inner })
    }

    /// Construct a market context from a JSON string.
    ///
    /// Parameters
    /// ----------
    /// payload : str
    ///     Versioned market snapshot JSON.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Restored market context.
    #[classmethod]
    fn from_json(_cls: &Bound<'_, PyType>, payload: &str) -> PyResult<Self> {
        serde_json::from_str(payload)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("failed to deserialize MarketContext: {e}")))
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
    /// Insert or replace a curve using the generic MarketContext insert path.
    ///
    /// This is a Python convenience shim over the canonical Rust `insert()`
    /// method and accepts curve objects such as `DiscountCurve`,
    /// `ForwardCurve`, `HazardCurve`, `InflationCurve`,
    /// `BaseCorrelationCurve`, `PriceCurve`, and `VolatilityIndexCurve`.
    fn insert(&mut self, curve: Bound<'_, PyAny>) -> PyResult<()> {
        self.insert_curve_like(curve)
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
        self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
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
        self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
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
        self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
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
        self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
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
        self.inner = std::mem::take(&mut self.inner).insert(curve.inner.as_ref().clone());
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
        self.inner = std::mem::take(&mut self.inner).insert_fx(fx_matrix.inner.clone());
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

    #[pyo3(text_signature = "(self, surface)")]
    fn insert_fx_delta_vol_surface(&mut self, surface: &PyFxDeltaVolSurface) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner)
            .insert_fx_delta_vol_surface(surface.inner.as_ref().clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, id, index)")]
    fn insert_inflation_index(&mut self, id: &str, index: &PyInflationIndex) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner)
            .insert_inflation_index(id, index.inner.as_ref().clone());
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

    #[pyo3(text_signature = "(self)")]
    fn clear_fx(&mut self) -> PyResult<()> {
        self.inner = std::mem::take(&mut self.inner).clear_fx();
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
    fn get_discount(&self, id: &str) -> PyResult<PyDiscountCurve> {
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
    fn get_forward(&self, id: &str) -> PyResult<PyForwardCurve> {
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
    fn get_hazard(&self, id: &str) -> PyResult<PyHazardCurve> {
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
    fn get_inflation_curve(&self, id: &str) -> PyResult<PyInflationCurve> {
        let arc = self.inner.get_inflation_curve(id).map_err(core_to_py)?;
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
    fn get_base_correlation(&self, id: &str) -> PyResult<PyBaseCorrelationCurve> {
        let arc = self.inner.get_base_correlation(id).map_err(core_to_py)?;
        Ok(PyBaseCorrelationCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn get_vol_index_curve(&self, id: &str) -> PyResult<PyVolatilityIndexCurve> {
        let arc = self.inner.get_vol_index_curve(id).map_err(core_to_py)?;
        Ok(PyVolatilityIndexCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn get_price_curve(&self, id: &str) -> PyResult<PyPriceCurve> {
        let arc = self.inner.get_price_curve(id).map_err(core_to_py)?;
        Ok(PyPriceCurve::new_arc(arc))
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
    /// ConfigurationError
    ///     If the surface is not present.
    fn get_surface(&self, id: &str) -> PyResult<PyVolSurface> {
        let arc = self.inner.get_surface(id).map_err(core_to_py)?;
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
    /// ConfigurationError
    ///     If the price is not present.
    fn get_price(&self, id: &str) -> PyResult<PyMarketScalar> {
        let scalar = self.inner.get_price(id).map_err(core_to_py)?;
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
    /// ConfigurationError
    ///     If the series is not present.
    fn get_series(&self, id: &str) -> PyResult<PyScalarTimeSeries> {
        let series = self.inner.get_series(id).map_err(core_to_py)?;
        Ok(PyScalarTimeSeries::new_arc(series.clone()))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn get_inflation_index(&self, id: &str) -> PyResult<PyInflationIndex> {
        let index = self.inner.get_inflation_index(id).map_err(core_to_py)?;
        Ok(PyInflationIndex::new_arc(index))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn get_fx_delta_vol_surface(&self, id: &str) -> PyResult<PyFxDeltaVolSurface> {
        let surface = self
            .inner
            .get_fx_delta_vol_surface(id)
            .map_err(core_to_py)?;
        Ok(PyFxDeltaVolSurface::new_arc(surface))
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
    /// ConfigurationError
    ///     If the index is not present.
    fn get_credit_index(&self, id: &str) -> PyResult<PyCreditIndexData> {
        let data = self.inner.get_credit_index(id).map_err(core_to_py)?;
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
    /// DividendSchedule
    ///     Dividend schedule for ``id``.
    ///
    /// Raises
    /// ------
    /// ConfigurationError
    ///     If the dividend schedule is not present.
    fn get_dividend_schedule(&self, id: &str) -> PyResult<PyDividendSchedule> {
        let schedule = self.inner.get_dividend_schedule(id).map_err(core_to_py)?;
        Ok(PyDividendSchedule::new(schedule.as_ref().clone()))
    }

    #[pyo3(text_signature = "(self, csa_code)")]
    fn get_collateral(&self, csa_code: &str) -> PyResult<PyDiscountCurve> {
        let state: MarketContextState = (&self.inner).into();
        let curve_id = state.collateral.get(csa_code).ok_or_else(|| {
            core_to_py(
                finstack_core::error::InputError::NotFound {
                    id: format!("collateral:{csa_code}"),
                }
                .into(),
            )
        })?;
        let arc = self.inner.get_discount(curve_id).map_err(core_to_py)?;
        Ok(PyDiscountCurve::new_arc(arc))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn curve(&self, py: Python<'_>, id: &str) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .curve(id)
            .map(|storage| Self::wrap_curve_storage(py, storage))
            .transpose()
    }

    #[pyo3(text_signature = "(self)")]
    fn fx(&self) -> Option<PyFxMatrix> {
        self.inner
            .fx()
            .map(|inner| PyFxMatrix::from_inner(inner.clone()))
    }

    #[pyo3(text_signature = "(self)")]
    fn surfaces_snapshot(&self) -> HashMap<String, PyVolSurface> {
        self.inner
            .surfaces_snapshot()
            .into_iter()
            .map(|(id, surface)| (id.to_string(), PyVolSurface::new_arc(surface)))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn prices_snapshot(&self) -> HashMap<String, PyMarketScalar> {
        self.inner
            .prices_snapshot()
            .into_iter()
            .map(|(id, scalar)| (id.to_string(), PyMarketScalar::new(scalar)))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn series_snapshot(&self) -> HashMap<String, PyScalarTimeSeries> {
        self.inner
            .series_snapshot()
            .into_iter()
            .map(|(id, series)| (id.to_string(), PyScalarTimeSeries::new_arc(series)))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn prices_iter(&self) -> Vec<(String, PyMarketScalar)> {
        self.inner
            .prices_iter()
            .map(|(id, scalar)| (id.to_string(), PyMarketScalar::new(scalar.clone())))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn series_iter(&self) -> Vec<(String, PyScalarTimeSeries)> {
        self.inner
            .series_iter()
            .map(|(id, series)| (id.to_string(), PyScalarTimeSeries::new_arc(series.clone())))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn inflation_indices_iter(&self) -> Vec<(String, PyInflationIndex)> {
        self.inner
            .inflation_indices_iter()
            .map(|(id, index)| (id.to_string(), PyInflationIndex::new_arc(index.clone())))
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    fn dividends_iter(&self) -> Vec<(String, PyDividendSchedule)> {
        self.inner
            .dividends_iter()
            .map(|(id, schedule)| {
                (
                    id.to_string(),
                    PyDividendSchedule::new(schedule.as_ref().clone()),
                )
            })
            .collect()
    }

    #[pyo3(text_signature = "(self, days)")]
    fn roll_forward(&self, days: i64) -> PyResult<Self> {
        let rolled = self.inner.roll_forward(days).map_err(core_to_py)?;
        Ok(Self { inner: rolled })
    }

    #[pyo3(text_signature = "(self, id, new_curve)")]
    fn update_base_correlation_curve(
        &mut self,
        id: &str,
        new_curve: &PyBaseCorrelationCurve,
    ) -> bool {
        self.inner
            .update_base_correlation_curve(id, new_curve.inner.clone())
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

    #[pyo3(text_signature = "(self, curve_type)")]
    fn curves_of_type(
        &self,
        py: Python<'_>,
        curve_type: &str,
    ) -> PyResult<Vec<(String, Py<PyAny>)>> {
        self.inner
            .curves_of_type(curve_type)
            .map(|(id, storage)| Ok((id.to_string(), Self::wrap_curve_storage(py, storage)?)))
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

    #[pyo3(text_signature = "(self, surfaces)")]
    fn replace_surfaces_mut(&mut self, surfaces: Bound<'_, PyAny>) -> PyResult<()> {
        let iter = PyIterator::from_object(&surfaces)?;
        let mut items = Vec::new();
        for item in iter {
            let tuple = item?;
            let (id, surface): (String, Py<PyVolSurface>) = tuple.extract()?;
            let surface_ref = surface.bind(tuple.py()).borrow();
            items.push((CurveId::new(id), surface_ref.inner.clone()));
        }
        self.inner.replace_surfaces_mut(items);
        Ok(())
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

    /// Support ``copy.copy(ctx)``.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Shallow clone of this context.
    fn __copy__(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Support ``copy.deepcopy(ctx)``.
    ///
    /// All curves, surfaces, and scalars inside a ``MarketContext`` are
    /// ``Arc``-shared and immutable, so a deep copy is equivalent to a shallow
    /// copy—both reference the same underlying data safely.
    ///
    /// Parameters
    /// ----------
    /// _memo : Any
    ///     Memo dictionary (unused).
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Clone of this context sharing the same immutable market data.
    fn __deepcopy__(&self, _memo: Bound<'_, PyAny>) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Check if a curve or surface with the given ID exists in the context.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Identifier to look up (curve id or surface id).
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` if a curve or surface with the given id is registered.
    fn __contains__(&self, id: &str) -> bool {
        // Check curves first, then surfaces
        self.inner.curve(id).is_some() || self.inner.get_surface(id).is_ok()
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
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
