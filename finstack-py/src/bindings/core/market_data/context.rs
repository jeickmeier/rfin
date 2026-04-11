//! Python bindings for [`finstack_core::market_data::context::MarketContext`].

use std::sync::Arc;

use finstack_core::market_data::context::MarketContext;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

use crate::errors::core_to_py;

use super::curves::{
    PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyPriceCurve, PyVolatilityIndexCurve,
};
use super::fx::PyFxMatrix;

// ---------------------------------------------------------------------------
// PyMarketContext
// ---------------------------------------------------------------------------

/// Unified market data container for curves, surfaces, and FX.
///
/// Wraps [`MarketContext`] from `finstack-core`. Curves are stored behind
/// `Arc` and the context is cheap to clone.
#[pyclass(
    name = "MarketContext",
    module = "finstack.core.market_data.context",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarketContext {
    /// Underlying Rust context.
    inner: MarketContext,
}

#[pymethods]
impl PyMarketContext {
    /// Create an empty market context.
    #[new]
    fn new() -> Self {
        Self {
            inner: MarketContext::new(),
        }
    }

    /// Insert a curve into the context (fluent, returns ``self``).
    ///
    /// Accepts any curve type: ``DiscountCurve``, ``ForwardCurve``,
    /// ``HazardCurve``, ``PriceCurve``, or ``VolatilityIndexCurve``.
    #[pyo3(text_signature = "(self, curve)")]
    fn insert<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(dc) = curve.extract::<PyRef<'_, PyDiscountCurve>>() {
            slf.inner = std::mem::take(&mut slf.inner).insert(Arc::clone(&dc.inner));
            return Ok(slf);
        }
        if let Ok(fc) = curve.extract::<PyRef<'_, PyForwardCurve>>() {
            slf.inner = std::mem::take(&mut slf.inner).insert(Arc::clone(&fc.inner));
            return Ok(slf);
        }
        if let Ok(hc) = curve.extract::<PyRef<'_, PyHazardCurve>>() {
            slf.inner = std::mem::take(&mut slf.inner).insert(Arc::clone(&hc.inner));
            return Ok(slf);
        }
        if let Ok(pc) = curve.extract::<PyRef<'_, PyPriceCurve>>() {
            slf.inner = std::mem::take(&mut slf.inner).insert(Arc::clone(&pc.inner));
            return Ok(slf);
        }
        if let Ok(vc) = curve.extract::<PyRef<'_, PyVolatilityIndexCurve>>() {
            slf.inner = std::mem::take(&mut slf.inner).insert(Arc::clone(&vc.inner));
            return Ok(slf);
        }
        Err(PyTypeError::new_err(
            "insert() expects a DiscountCurve, ForwardCurve, HazardCurve, PriceCurve, or VolatilityIndexCurve",
        ))
    }

    /// Insert an FX matrix into the context.
    #[pyo3(text_signature = "(self, fx)")]
    fn insert_fx(&mut self, fx: &PyFxMatrix) {
        self.inner = std::mem::take(&mut self.inner).insert_fx(Arc::clone(&fx.inner));
    }

    /// Retrieve a discount curve by identifier.
    ///
    /// Raises ``ValueError`` if the curve does not exist or is not a discount curve.
    #[pyo3(text_signature = "(self, id)")]
    fn get_discount(&self, id: &str) -> PyResult<PyDiscountCurve> {
        let arc = self.inner.get_discount(id).map_err(core_to_py)?;
        Ok(PyDiscountCurve::from_inner(arc))
    }

    /// Retrieve a forward curve by identifier.
    ///
    /// Raises ``ValueError`` if the curve does not exist or is not a forward curve.
    #[pyo3(text_signature = "(self, id)")]
    fn get_forward(&self, id: &str) -> PyResult<PyForwardCurve> {
        let arc = self.inner.get_forward(id).map_err(core_to_py)?;
        Ok(PyForwardCurve::from_inner(arc))
    }

    /// Retrieve a hazard curve by identifier.
    ///
    /// Raises ``ValueError`` if the curve does not exist or is not a hazard curve.
    #[pyo3(text_signature = "(self, id)")]
    fn get_hazard(&self, id: &str) -> PyResult<PyHazardCurve> {
        let arc = self.inner.get_hazard(id).map_err(core_to_py)?;
        Ok(PyHazardCurve::from_inner(arc))
    }

    /// Retrieve a price curve by identifier.
    ///
    /// Raises ``ValueError`` if the curve does not exist or is not a price curve.
    #[pyo3(text_signature = "(self, id)")]
    fn get_price_curve(&self, id: &str) -> PyResult<PyPriceCurve> {
        let arc = self.inner.get_price_curve(id).map_err(core_to_py)?;
        Ok(PyPriceCurve::from_inner(arc))
    }

    /// Retrieve a volatility index curve by identifier.
    ///
    /// Raises ``ValueError`` if the curve does not exist or is not a vol-index curve.
    #[pyo3(text_signature = "(self, id)")]
    fn get_vol_index_curve(&self, id: &str) -> PyResult<PyVolatilityIndexCurve> {
        let arc = self.inner.get_vol_index_curve(id).map_err(core_to_py)?;
        Ok(PyVolatilityIndexCurve::from_inner(arc))
    }

    /// Access the FX matrix (returns ``None`` if not set).
    #[getter]
    fn fx(&self) -> Option<PyFxMatrix> {
        self.inner.fx().map(|arc_fx| {
            let provider = Arc::new(finstack_core::money::fx::SimpleFxProvider::new());
            PyFxMatrix {
                provider,
                inner: Arc::clone(arc_fx),
            }
        })
    }

    fn __repr__(&self) -> String {
        "MarketContext(...)".to_string()
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register the `finstack.core.market_data.context` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "context")?;
    m.setattr(
        "__doc__",
        "Market data context container bindings (finstack-core).",
    )?;

    m.add_class::<PyMarketContext>()?;

    let all = PyList::new(py, ["MarketContext"])?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core.market_data".to_string(),
        },
        Err(_) => "finstack.core.market_data".to_string(),
    };
    let qual = format!("{pkg}.context");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
