//! Python bindings for Agency TBA instruments.

use crate::core::common::args::CurrencyArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyProgram;
use finstack_valuations::instruments::fixed_income::tba::{AgencyTba, TbaSettlement, TbaTerm};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::sync::Arc;

use super::mbs_passthrough::PyAgencyProgram;

// =============================================================================
// TBA Term Enum
// =============================================================================

/// TBA original loan term.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TbaTerm",
    eq,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum PyTbaTerm {
    /// 15-year original term.
    FifteenYear,
    /// 20-year original term.
    TwentyYear,
    /// 30-year original term.
    ThirtyYear,
}

impl From<PyTbaTerm> for TbaTerm {
    fn from(py: PyTbaTerm) -> Self {
        match py {
            PyTbaTerm::FifteenYear => TbaTerm::FifteenYear,
            PyTbaTerm::TwentyYear => TbaTerm::TwentyYear,
            PyTbaTerm::ThirtyYear => TbaTerm::ThirtyYear,
        }
    }
}

impl TryFrom<TbaTerm> for PyTbaTerm {
    type Error = &'static str;

    fn try_from(rust: TbaTerm) -> Result<Self, Self::Error> {
        match rust {
            TbaTerm::FifteenYear => Ok(PyTbaTerm::FifteenYear),
            TbaTerm::TwentyYear => Ok(PyTbaTerm::TwentyYear),
            TbaTerm::ThirtyYear => Ok(PyTbaTerm::ThirtyYear),
            _ => Err("unknown TbaTerm variant"),
        }
    }
}

#[pymethods]
impl PyTbaTerm {
    fn __repr__(&self) -> String {
        format!("TbaTerm.{:?}", self)
    }
}

// =============================================================================
// TBA Settlement
// =============================================================================

/// TBA settlement information.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TbaSettlement",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTbaSettlement {
    pub(crate) inner: TbaSettlement,
}

#[pymethods]
impl PyTbaSettlement {
    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    #[getter]
    fn notification_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.notification_date)
    }

    fn __repr__(&self) -> String {
        format!(
            "TbaSettlement(settlement={}, notification={})",
            self.inner.settlement_date, self.inner.notification_date
        )
    }
}

// =============================================================================
// Agency TBA
// =============================================================================

/// Agency To-Be-Announced (TBA) forward contract.
///
/// Examples:
///     >>> tba = AgencyTba.builder("FN30-4.0-202403").agency(AgencyProgram.Fnma).coupon(0.04).term(TbaTerm.ThirtyYear).settlement_year(2024).settlement_month(3).notional(10_000_000.0).currency("USD").trade_price(98.5).discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyTba",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAgencyTba {
    pub(crate) inner: Arc<AgencyTba>,
}

impl PyAgencyTba {
    pub(crate) fn new(inner: AgencyTba) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyTbaBuilder",
    skip_from_py_object
)]
pub struct PyAgencyTbaBuilder {
    instrument_id: InstrumentId,
    agency: Option<AgencyProgram>,
    coupon: Option<f64>,
    term: Option<TbaTerm>,
    settlement_year: Option<i32>,
    settlement_month: Option<u8>,
    notional: Option<f64>,
    currency: Option<finstack_core::currency::Currency>,
    trade_price: Option<f64>,
    discount_curve_id: Option<String>,
    trade_date: Option<time::Date>,
}

impl PyAgencyTbaBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            agency: None,
            coupon: None,
            term: None,
            settlement_year: None,
            settlement_month: None,
            notional: None,
            currency: None,
            trade_price: None,
            discount_curve_id: None,
            trade_date: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.agency.is_none() {
            return Err(PyValueError::new_err("agency() is required."));
        }
        if self.coupon.is_none() {
            return Err(PyValueError::new_err("coupon() is required."));
        }
        if self.term.is_none() {
            return Err(PyValueError::new_err("term() is required."));
        }
        if self.settlement_year.is_none() {
            return Err(PyValueError::new_err("settlement_year() is required."));
        }
        if self.settlement_month.is_none() {
            return Err(PyValueError::new_err("settlement_month() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.trade_price.is_none() {
            return Err(PyValueError::new_err("trade_price() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyAgencyTbaBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn agency(mut slf: PyRefMut<'_, Self>, agency: PyAgencyProgram) -> PyRefMut<'_, Self> {
        slf.agency = Some(agency.into());
        slf
    }

    fn coupon(mut slf: PyRefMut<'_, Self>, coupon: f64) -> PyRefMut<'_, Self> {
        slf.coupon = Some(coupon);
        slf
    }

    fn term(mut slf: PyRefMut<'_, Self>, term: PyTbaTerm) -> PyRefMut<'_, Self> {
        slf.term = Some(term.into());
        slf
    }

    fn settlement_year(mut slf: PyRefMut<'_, Self>, settlement_year: i32) -> PyRefMut<'_, Self> {
        slf.settlement_year = Some(settlement_year);
        slf
    }

    fn settlement_month(mut slf: PyRefMut<'_, Self>, settlement_month: u8) -> PyRefMut<'_, Self> {
        slf.settlement_month = Some(settlement_month);
        slf
    }

    fn notional(mut slf: PyRefMut<'_, Self>, notional: f64) -> PyRefMut<'_, Self> {
        slf.notional = Some(notional);
        slf
    }

    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    fn trade_price(mut slf: PyRefMut<'_, Self>, trade_price: f64) -> PyRefMut<'_, Self> {
        slf.trade_price = Some(trade_price);
        slf
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    #[pyo3(signature = (trade_date=None))]
    fn trade_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        trade_date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.trade_date = trade_date.map(|d| py_to_date(&d)).transpose()?;
        Ok(slf)
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyAgencyTba> {
        slf.ensure_ready()?;
        let ccy = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "AgencyTbaBuilder internal error: missing currency after validation",
            )
        })?;

        let mut builder = AgencyTba::builder()
            .id(slf.instrument_id.clone())
            .agency(slf.agency.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing agency after validation"))?)
            .coupon(slf.coupon.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing coupon after validation"))?)
            .term(slf.term.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing term after validation"))?)
            .settlement_year(slf.settlement_year.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing settlement_year after validation"))?)
            .settlement_month(slf.settlement_month.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing settlement_month after validation"))?)
            .notional(Money::new(slf.notional.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing notional after validation"))?, ccy))
            .trade_price(slf.trade_price.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing trade_price after validation"))?)
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyTbaBuilder internal error: missing discount_curve_id after validation"))?))
            .attributes(Attributes::new());

        if let Some(td) = slf.trade_date {
            builder = builder.trade_date_opt(Some(td));
        }

        let tba = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(PyAgencyTba::new(tba))
    }
}

#[pymethods]
impl PyAgencyTba {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyAgencyTbaBuilder>> {
        let py = cls.py();
        let builder = PyAgencyTbaBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example TBA for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> PyResult<Self> {
        AgencyTba::example()
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("{e}")))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Agency program.
    #[getter]
    fn agency(&self) -> PyAgencyProgram {
        self.inner.agency.into()
    }

    /// Coupon rate.
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    /// Term.
    #[getter]
    fn term(&self) -> PyResult<PyTbaTerm> {
        PyTbaTerm::try_from(self.inner.term).map_err(crate::errors::InternalError::new_err)
    }

    /// Settlement year.
    #[getter]
    fn settlement_year(&self) -> i32 {
        self.inner.settlement_year
    }

    /// Settlement month.
    #[getter]
    fn settlement_month(&self) -> u8 {
        self.inner.settlement_month
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Trade price.
    #[getter]
    fn trade_price(&self) -> f64 {
        self.inner.trade_price
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgencyTba(id='{}', coupon={:.2}%, settlement={}/{:02})",
            self.inner.id.as_str(),
            self.inner.coupon * 100.0,
            self.inner.settlement_year,
            self.inner.settlement_month
        )
    }
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register(
    _py: Python<'_>,
    parent: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyTbaTerm>()?;
    parent.add_class::<PyTbaSettlement>()?;
    parent.add_class::<PyAgencyTba>()?;
    parent.add_class::<PyAgencyTbaBuilder>()?;

    Ok(vec![
        "TbaTerm",
        "TbaSettlement",
        "AgencyTba",
        "AgencyTbaBuilder",
    ])
}
