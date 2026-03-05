#![allow(clippy::unwrap_used)]

//! Python bindings for Dollar Roll instruments.

use crate::core::common::args::CurrencyArg;
use crate::core::dates::utils::py_to_date;
use crate::errors::PyContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::dollar_roll::DollarRoll;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyProgram;
use finstack_valuations::instruments::fixed_income::tba::TbaTerm;
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::sync::Arc;

use super::mbs_passthrough::PyAgencyProgram;
use super::tba::PyTbaTerm;

// =============================================================================
// Dollar Roll
// =============================================================================

/// Dollar roll between TBA settlement months.
///
/// A dollar roll involves selling TBA for near-month settlement
/// and buying TBA for far-month settlement.
///
/// Examples:
///     >>> roll = DollarRoll.builder("FN30-4.0-ROLL-0324-0424").agency(AgencyProgram.Fnma).coupon(0.04).term(TbaTerm.ThirtyYear).notional(10_000_000.0).currency("USD").front_settlement_year(2024).front_settlement_month(3).back_settlement_year(2024).back_settlement_month(4).front_price(98.5).back_price(98.0).discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DollarRoll",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDollarRoll {
    pub(crate) inner: Arc<DollarRoll>,
}

impl PyDollarRoll {
    pub(crate) fn new(inner: DollarRoll) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DollarRollBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyDollarRollBuilder {
    instrument_id: InstrumentId,
    agency: Option<AgencyProgram>,
    coupon: Option<f64>,
    term: Option<TbaTerm>,
    notional: Option<f64>,
    currency: Option<finstack_core::currency::Currency>,
    front_settlement_year: Option<i32>,
    front_settlement_month: Option<u8>,
    back_settlement_year: Option<i32>,
    back_settlement_month: Option<u8>,
    front_price: Option<f64>,
    back_price: Option<f64>,
    discount_curve_id: Option<String>,
    trade_date: Option<time::Date>,
    repo_curve_id: Option<String>,
}

impl PyDollarRollBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            agency: None,
            coupon: None,
            term: None,
            notional: None,
            currency: None,
            front_settlement_year: None,
            front_settlement_month: None,
            back_settlement_year: None,
            back_settlement_month: None,
            front_price: None,
            back_price: None,
            discount_curve_id: None,
            trade_date: None,
            repo_curve_id: None,
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
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.front_settlement_year.is_none() {
            return Err(PyValueError::new_err(
                "front_settlement_year() is required.",
            ));
        }
        if self.front_settlement_month.is_none() {
            return Err(PyValueError::new_err(
                "front_settlement_month() is required.",
            ));
        }
        if self.back_settlement_year.is_none() {
            return Err(PyValueError::new_err("back_settlement_year() is required."));
        }
        if self.back_settlement_month.is_none() {
            return Err(PyValueError::new_err(
                "back_settlement_month() is required.",
            ));
        }
        if self.front_price.is_none() {
            return Err(PyValueError::new_err("front_price() is required."));
        }
        if self.back_price.is_none() {
            return Err(PyValueError::new_err("back_price() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyDollarRollBuilder {
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

    fn front_settlement_year(
        mut slf: PyRefMut<'_, Self>,
        front_settlement_year: i32,
    ) -> PyRefMut<'_, Self> {
        slf.front_settlement_year = Some(front_settlement_year);
        slf
    }

    fn front_settlement_month(
        mut slf: PyRefMut<'_, Self>,
        front_settlement_month: u8,
    ) -> PyRefMut<'_, Self> {
        slf.front_settlement_month = Some(front_settlement_month);
        slf
    }

    fn back_settlement_year(
        mut slf: PyRefMut<'_, Self>,
        back_settlement_year: i32,
    ) -> PyRefMut<'_, Self> {
        slf.back_settlement_year = Some(back_settlement_year);
        slf
    }

    fn back_settlement_month(
        mut slf: PyRefMut<'_, Self>,
        back_settlement_month: u8,
    ) -> PyRefMut<'_, Self> {
        slf.back_settlement_month = Some(back_settlement_month);
        slf
    }

    fn front_price(mut slf: PyRefMut<'_, Self>, front_price: f64) -> PyRefMut<'_, Self> {
        slf.front_price = Some(front_price);
        slf
    }

    fn back_price(mut slf: PyRefMut<'_, Self>, back_price: f64) -> PyRefMut<'_, Self> {
        slf.back_price = Some(back_price);
        slf
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    /// Set repo/financing curve identifier.
    ///
    /// When set, this curve is used for financing/carry calculations instead of
    /// the general discount curve, capturing repo specials.
    fn repo_curve_id(mut slf: PyRefMut<'_, Self>, repo_curve_id: String) -> PyRefMut<'_, Self> {
        slf.repo_curve_id = Some(repo_curve_id);
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

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyDollarRoll> {
        slf.ensure_ready()?;
        let ccy = slf.currency.unwrap();

        let mut builder = DollarRoll::builder()
            .id(slf.instrument_id.clone())
            .agency(slf.agency.unwrap())
            .coupon(slf.coupon.unwrap())
            .term(slf.term.unwrap())
            .notional(Money::new(slf.notional.unwrap(), ccy))
            .front_settlement_year(slf.front_settlement_year.unwrap())
            .front_settlement_month(slf.front_settlement_month.unwrap())
            .back_settlement_year(slf.back_settlement_year.unwrap())
            .back_settlement_month(slf.back_settlement_month.unwrap())
            .front_price(slf.front_price.unwrap())
            .back_price(slf.back_price.unwrap())
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().unwrap()))
            .attributes(Attributes::new());

        if let Some(td) = slf.trade_date {
            builder = builder.trade_date_opt(Some(td));
        }

        if let Some(ref repo_id) = slf.repo_curve_id {
            builder = builder.repo_curve_id_opt(Some(CurveId::new(repo_id)));
        }

        let roll = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(PyDollarRoll::new(roll))
    }
}

#[pymethods]
impl PyDollarRoll {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyDollarRollBuilder>> {
        let py = cls.py();
        let builder = PyDollarRollBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example dollar roll for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(DollarRoll::example())
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
    fn term(&self) -> PyTbaTerm {
        self.inner.term.into()
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Front-month settlement year.
    #[getter]
    fn front_settlement_year(&self) -> i32 {
        self.inner.front_settlement_year
    }

    /// Front-month settlement month.
    #[getter]
    fn front_settlement_month(&self) -> u8 {
        self.inner.front_settlement_month
    }

    /// Back-month settlement year.
    #[getter]
    fn back_settlement_year(&self) -> i32 {
        self.inner.back_settlement_year
    }

    /// Back-month settlement month.
    #[getter]
    fn back_settlement_month(&self) -> u8 {
        self.inner.back_settlement_month
    }

    /// Front-month price (sell price).
    #[getter]
    fn front_price(&self) -> f64 {
        self.inner.front_price
    }

    /// Back-month price (buy price).
    #[getter]
    fn back_price(&self) -> f64 {
        self.inner.back_price
    }

    /// Get the drop (price difference between front and back month).
    fn drop_value(&self) -> f64 {
        self.inner.as_ref().drop()
    }

    /// Get the drop in 32nds.
    fn drop_32nds(&self) -> f64 {
        self.inner.drop_32nds()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "DollarRoll(id='{}', drop={:.3})",
            self.inner.id.as_str(),
            self.inner.as_ref().drop()
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
    parent.add_class::<PyDollarRoll>()?;
    parent.add_class::<PyDollarRollBuilder>()?;

    Ok(vec!["DollarRoll", "DollarRollBuilder"])
}
