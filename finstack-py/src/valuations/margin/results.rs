use crate::core::currency::PyCurrency;
use crate::core::dates::utils::date_to_py;
use crate::core::money::PyMoney;
use finstack_margin::{ImResult, InstrumentMarginResult, SimmSensitivities, VmResult};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::csa::PyImMethodology;
use super::helpers::parse_currency;

/// Variation margin calculation result.
#[pyclass(
    name = "VmResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVmResult {
    pub(crate) inner: VmResult,
}

impl PyVmResult {
    pub(crate) fn new(inner: VmResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVmResult {
    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }

    #[getter]
    fn gross_exposure(&self) -> PyMoney {
        PyMoney::new(self.inner.gross_exposure)
    }

    #[getter]
    fn net_exposure(&self) -> PyMoney {
        PyMoney::new(self.inner.net_exposure)
    }

    #[getter]
    fn delivery_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.delivery_amount)
    }

    #[getter]
    fn return_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.return_amount)
    }

    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    fn net_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.net_margin())
    }

    fn requires_call(&self) -> bool {
        self.inner.requires_call()
    }

    fn __repr__(&self) -> String {
        format!(
            "VmResult(date={}, delivery={}, return={})",
            self.inner.date, self.inner.delivery_amount, self.inner.return_amount
        )
    }
}

/// Initial margin calculation result.
#[pyclass(
    name = "ImResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyImResult {
    pub(crate) inner: ImResult,
}

impl PyImResult {
    #[allow(dead_code)]
    pub(crate) fn new(inner: ImResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyImResult {
    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    #[getter]
    fn methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.methodology)
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    #[getter]
    fn breakdown(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        for (key, &value) in &self.inner.breakdown {
            d.set_item(key, PyMoney::new(value).into_pyobject(py)?)?;
        }
        Ok(d.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ImResult(amount={}, methodology={}, mpor_days={})",
            self.inner.amount, self.inner.methodology, self.inner.mpor_days
        )
    }
}

/// Per-instrument margin calculation result.
#[pyclass(
    name = "InstrumentMarginResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInstrumentMarginResult {
    pub(crate) inner: InstrumentMarginResult,
}

impl PyInstrumentMarginResult {
    #[allow(dead_code)]
    pub(crate) fn new(inner: InstrumentMarginResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInstrumentMarginResult {
    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn initial_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.initial_margin)
    }

    #[getter]
    fn variation_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.variation_margin)
    }

    #[getter]
    fn total_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_margin)
    }

    #[getter]
    fn im_methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.im_methodology)
    }

    #[getter]
    fn is_cleared(&self) -> bool {
        self.inner.is_cleared
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentMarginResult(id={}, total={})",
            self.inner.instrument_id, self.inner.total_margin
        )
    }
}

/// SIMM sensitivity inputs organized by risk class.
#[pyclass(
    name = "SimmSensitivities",
    module = "finstack.valuations.margin",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySimmSensitivities {
    pub(crate) inner: SimmSensitivities,
}

impl PySimmSensitivities {
    pub(crate) fn new(inner: SimmSensitivities) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmSensitivities {
    #[new]
    fn ctor(base_currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(SimmSensitivities::new(parse_currency(
            base_currency,
        )?)))
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    fn add_ir_delta(
        &mut self,
        currency: &Bound<'_, PyAny>,
        tenor: String,
        delta: f64,
    ) -> PyResult<()> {
        self.inner
            .add_ir_delta(parse_currency(currency)?, tenor, delta);
        Ok(())
    }

    fn add_ir_vega(
        &mut self,
        currency: &Bound<'_, PyAny>,
        tenor: String,
        vega: f64,
    ) -> PyResult<()> {
        self.inner
            .add_ir_vega(parse_currency(currency)?, tenor, vega);
        Ok(())
    }

    fn add_credit_delta(&mut self, name: String, qualifying: bool, tenor: String, delta: f64) {
        self.inner.add_credit_delta(name, qualifying, tenor, delta);
    }

    fn add_equity_delta(&mut self, underlier: String, delta: f64) {
        self.inner.add_equity_delta(underlier, delta);
    }

    fn add_equity_vega(&mut self, underlier: String, vega: f64) {
        self.inner.add_equity_vega(underlier, vega);
    }

    fn add_fx_delta(&mut self, currency: &Bound<'_, PyAny>, delta: f64) -> PyResult<()> {
        self.inner.add_fx_delta(parse_currency(currency)?, delta);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn total_ir_delta(&self) -> f64 {
        self.inner.total_ir_delta()
    }

    fn total_credit_delta(&self) -> f64 {
        self.inner.total_credit_delta()
    }

    fn total_equity_delta(&self) -> f64 {
        self.inner.total_equity_delta()
    }

    fn merge(&mut self, other: &PySimmSensitivities) {
        self.inner.merge(&other.inner);
    }

    fn __repr__(&self) -> String {
        format!(
            "SimmSensitivities(base_currency={}, empty={})",
            self.inner.base_currency,
            self.inner.is_empty()
        )
    }
}
