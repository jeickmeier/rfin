//! Python bindings for fixed-income index total return swaps.
//!
//! Rust source: `finstack/valuations/src/instruments/fixed_income/fi_trs/`
//! Separated from the equity TRS module for asset-class clarity.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use crate::valuations::instruments::equity::trs::{
    PyFinancingLegSpec, PyIndexUnderlyingParams, PyTrsScheduleSpec, PyTrsSide,
};
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::{FinancingLegSpec, IndexUnderlyingParams};
use finstack_valuations::instruments::{TrsScheduleSpec, TrsSide};
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use std::fmt;
use std::sync::Arc;

/// Fixed income index TRS wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FiIndexTotalReturnSwap",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFiIndexTotalReturnSwap {
    pub(crate) inner: Arc<FIIndexTotalReturnSwap>,
}

impl PyFiIndexTotalReturnSwap {
    pub(crate) fn new(inner: FIIndexTotalReturnSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FiIndexTotalReturnSwapBuilder",
    skip_from_py_object
)]
pub struct PyFiIndexTotalReturnSwapBuilder {
    instrument_id: InstrumentId,
    notional: Option<finstack_core::money::Money>,
    underlying: Option<IndexUnderlyingParams>,
    financing: Option<FinancingLegSpec>,
    schedule: Option<TrsScheduleSpec>,
    side: Option<TrsSide>,
    initial_level: Option<f64>,
}

impl PyFiIndexTotalReturnSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            notional: None,
            underlying: None,
            financing: None,
            schedule: None,
            side: None,
            initial_level: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.underlying.is_none() {
            return Err(PyValueError::new_err("underlying() is required."));
        }
        if self.financing.is_none() {
            return Err(PyValueError::new_err("financing() is required."));
        }
        if self.schedule.is_none() {
            return Err(PyValueError::new_err("schedule() is required."));
        }
        if self.side.is_none() {
            return Err(PyValueError::new_err("side() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyFiIndexTotalReturnSwapBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, underlying)")]
    fn underlying<'py>(
        mut slf: PyRefMut<'py, Self>,
        underlying: &PyIndexUnderlyingParams,
    ) -> PyRefMut<'py, Self> {
        slf.underlying = Some(underlying.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, financing)")]
    fn financing<'py>(
        mut slf: PyRefMut<'py, Self>,
        financing: &PyFinancingLegSpec,
    ) -> PyRefMut<'py, Self> {
        slf.financing = Some(financing.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, schedule)")]
    fn schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        schedule: &PyTrsScheduleSpec,
    ) -> PyRefMut<'py, Self> {
        slf.schedule = Some(schedule.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: PyTrsSide) -> PyRefMut<'_, Self> {
        slf.side = Some(side.inner);
        slf
    }

    #[pyo3(text_signature = "($self, initial_level=None)", signature = (initial_level=None))]
    fn initial_level(
        mut slf: PyRefMut<'_, Self>,
        initial_level: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.initial_level = initial_level;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFiIndexTotalReturnSwap> {
        slf.ensure_ready()?;
        let inner = FIIndexTotalReturnSwap {
            id: slf.instrument_id.clone(),
            notional: slf.notional.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FiIndexTotalReturnSwapBuilder internal error: missing notional after validation",
                )
            })?,
            underlying: slf.underlying.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FiIndexTotalReturnSwapBuilder internal error: missing underlying after validation",
                )
            })?,
            financing: slf.financing.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FiIndexTotalReturnSwapBuilder internal error: missing financing after validation",
                )
            })?,
            schedule: slf.schedule.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FiIndexTotalReturnSwapBuilder internal error: missing schedule after validation",
                )
            })?,
            side: slf.side.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FiIndexTotalReturnSwapBuilder internal error: missing side after validation",
                )
            })?,
            initial_level: slf.initial_level,
            pricing_overrides: finstack_valuations::instruments::PricingOverrides::default(),
            attributes: Attributes::new(),
            margin_spec: None,
        };
        Ok(PyFiIndexTotalReturnSwap::new(inner))
    }

    fn __repr__(&self) -> String {
        "FiIndexTotalReturnSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyFiIndexTotalReturnSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyFiIndexTotalReturnSwapBuilder>> {
        let py = cls.py();
        let builder =
            PyFiIndexTotalReturnSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FIIndexTotalReturnSwap)
    }

    #[getter]
    fn side(&self) -> &'static str {
        match self.inner.side {
            TrsSide::ReceiveTotalReturn => "receive_total_return",
            TrsSide::PayTotalReturn => "pay_total_return",
        }
    }

    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_total_return_leg(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.pv_total_return_leg(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_financing_leg(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.pv_financing_leg(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn financing_annuity(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        self.inner
            .financing_annuity(&market.inner, date)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "FiIndexTotalReturnSwap(id='{}', notional={}, side='{}')",
            self.inner.id,
            self.inner.notional,
            self.side()
        ))
    }
}

impl fmt::Display for PyFiIndexTotalReturnSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FiIndexTotalReturnSwap({}, side={})",
            self.inner.id,
            self.side()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyFiIndexTotalReturnSwap>()?;
    module.add_class::<PyFiIndexTotalReturnSwapBuilder>()?;
    Ok(vec![
        "FiIndexTotalReturnSwap",
        "FiIndexTotalReturnSwapBuilder",
    ])
}
