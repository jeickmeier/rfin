use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::errors::PyContext;
use crate::valuations::common::{
    frequency_from_payments_per_year, to_optional_string, PyInstrumentType,
};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::credit_derivatives::cds_tranche::{CdsTranche, TrancheSide};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_tranche_side(label: Option<&str>) -> PyResult<TrancheSide> {
    match label {
        None => Ok(TrancheSide::BuyProtection),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// CDS tranche wrapper exposing a simplified constructor.
///
/// Examples:
///     >>> tranche = (
///     ...     CdsTranche.builder("itraxx_tranche")
///     ...     .index_name("iTraxx Europe")
///     ...     .series(38)
///     ...     .attach_pct(3.0)
///     ...     .detach_pct(7.0)
///     ...     .notional(Money("EUR", 10_000_000))
///     ...     .maturity(date(2029, 3, 20))
///     ...     .running_coupon_bp(500.0)
///     ...     .discount_curve("eur_discount")
///     ...     .credit_index_curve("itraxx_credit")
///     ...     .build()
///     ... )
///     >>> tranche.attach_pct
///     3.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CdsTranche",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCdsTranche {
    pub(crate) inner: Arc<CdsTranche>,
}

impl PyCdsTranche {
    pub(crate) fn new(inner: CdsTranche) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CdsTrancheBuilder",
    unsendable
)]
pub struct PyCdsTrancheBuilder {
    instrument_id: InstrumentId,
    index_name: Option<String>,
    series: Option<u16>,
    attach_pct: Option<f64>,
    detach_pct: Option<f64>,
    notional: Option<finstack_core::money::Money>,
    maturity: Option<time::Date>,
    running_coupon_bp: Option<f64>,
    discount_curve_id: Option<CurveId>,
    credit_index_id: Option<CurveId>,
    side: TrancheSide,
    payments_per_year: u32,
    day_count: DayCount,
    business_day_convention: BusinessDayConvention,
    calendar: Option<String>,
    effective_date: Option<time::Date>,
}

impl PyCdsTrancheBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            index_name: None,
            series: None,
            attach_pct: None,
            detach_pct: None,
            notional: None,
            maturity: None,
            running_coupon_bp: None,
            discount_curve_id: None,
            credit_index_id: None,
            side: TrancheSide::BuyProtection,
            payments_per_year: 4,
            day_count: DayCount::Act360,
            business_day_convention: BusinessDayConvention::Following,
            calendar: None,
            effective_date: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.index_name.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("index_name() is required."));
        }
        if self.series.is_none() {
            return Err(PyValueError::new_err("series() is required."));
        }
        let attach = self
            .attach_pct
            .ok_or_else(|| PyValueError::new_err("attach_pct() is required."))?;
        let detach = self
            .detach_pct
            .ok_or_else(|| PyValueError::new_err("detach_pct() is required."))?;
        if attach < 0.0 || detach <= attach {
            return Err(PyValueError::new_err(
                "detach_pct must be greater than attach_pct and both non-negative",
            ));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.running_coupon_bp.is_none() {
            return Err(PyValueError::new_err("running_coupon_bp() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.credit_index_id.is_none() {
            return Err(PyValueError::new_err("credit_index_curve() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCdsTrancheBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, index_name)")]
    fn index_name(mut slf: PyRefMut<'_, Self>, index_name: String) -> PyRefMut<'_, Self> {
        slf.index_name = Some(index_name);
        slf
    }

    #[pyo3(text_signature = "($self, series)")]
    fn series(mut slf: PyRefMut<'_, Self>, series: u16) -> PyRefMut<'_, Self> {
        slf.series = Some(series);
        slf
    }

    #[pyo3(text_signature = "($self, attach_pct)")]
    fn attach_pct(mut slf: PyRefMut<'_, Self>, attach_pct: f64) -> PyRefMut<'_, Self> {
        slf.attach_pct = Some(attach_pct);
        slf
    }

    #[pyo3(text_signature = "($self, detach_pct)")]
    fn detach_pct(mut slf: PyRefMut<'_, Self>, detach_pct: f64) -> PyRefMut<'_, Self> {
        slf.detach_pct = Some(detach_pct);
        slf
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, running_coupon_bp)")]
    fn running_coupon_bp(
        mut slf: PyRefMut<'_, Self>,
        running_coupon_bp: f64,
    ) -> PyRefMut<'_, Self> {
        slf.running_coupon_bp = Some(running_coupon_bp);
        slf
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(discount_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, credit_index_curve)")]
    fn credit_index_curve(
        mut slf: PyRefMut<'_, Self>,
        credit_index_curve: String,
    ) -> PyRefMut<'_, Self> {
        slf.credit_index_id = Some(CurveId::new(credit_index_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.side = parse_tranche_side(Some(side.as_str())).context("side")?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, payments_per_year)")]
    fn payments_per_year(
        mut slf: PyRefMut<'_, Self>,
        payments_per_year: u32,
    ) -> PyRefMut<'_, Self> {
        slf.payments_per_year = payments_per_year;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract()?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, business_day_convention)")]
    fn business_day_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        business_day_convention: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let BusinessDayConventionArg(value) = business_day_convention.extract()?;
        slf.business_day_convention = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, calendar=None)", signature = (calendar=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar = calendar;
        slf
    }

    #[pyo3(text_signature = "($self, effective_date=None)", signature = (effective_date=None))]
    fn effective_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        effective_date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.effective_date = if let Some(date_obj) = effective_date {
            Some(py_to_date(&date_obj)?)
        } else {
            None
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCdsTranche> {
        slf.ensure_ready()?;

        let mut builder = CdsTranche::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.index_name(slf.index_name.clone().unwrap());
        builder = builder.series(slf.series.unwrap());
        builder = builder.attach_pct(slf.attach_pct.unwrap());
        builder = builder.detach_pct(slf.detach_pct.unwrap());
        builder = builder.notional(slf.notional.unwrap());
        builder = builder.maturity(slf.maturity.unwrap());
        builder = builder.running_coupon_bp(slf.running_coupon_bp.unwrap());
        builder = builder.payment_frequency(
            frequency_from_payments_per_year(Some(slf.payments_per_year))
                .map_err(|e| PyValueError::new_err(format!("Invalid payments_per_year: {e}")))?,
        );
        builder = builder.day_count(slf.day_count);
        builder = builder.business_day_convention(slf.business_day_convention);
        builder = builder.calendar_id_opt(to_optional_string(slf.calendar.as_deref()));
        builder = builder.discount_curve_id(slf.discount_curve_id.clone().unwrap().into());
        builder = builder.credit_index_id(slf.credit_index_id.clone().unwrap().into());
        builder = builder.side(slf.side);
        builder = builder.effective_date_opt(slf.effective_date);
        builder = builder.attributes(Default::default());
        builder = builder.standard_imm_dates(true);
        builder = builder.accumulated_loss(0.0);

        let tranche = builder.build().map_err(core_to_py)?;
        Ok(PyCdsTranche::new(tranche))
    }

    fn __repr__(&self) -> String {
        "CdsTrancheBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCdsTranche {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCdsTrancheBuilder>> {
        let py = cls.py();
        let builder = PyCdsTrancheBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the tranche.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Tranche notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Attachment point percentage.
    ///
    /// Returns:
    ///     float: Attachment level in percent.
    #[getter]
    fn attach_pct(&self) -> f64 {
        self.inner.attach_pct
    }

    /// Detachment point percentage.
    ///
    /// Returns:
    ///     float: Detachment level in percent.
    #[getter]
    fn detach_pct(&self) -> f64 {
        self.inner.detach_pct
    }

    /// Running coupon in basis points.
    ///
    /// Returns:
    ///     float: Running spread paid on outstanding tranche balance.
    #[getter]
    fn running_coupon_bp(&self) -> f64 {
        self.inner.running_coupon_bp
    }

    /// Maturity date of the tranche.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Credit index curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve used for the index portfolio.
    #[getter]
    fn credit_index_curve(&self) -> String {
        self.inner.credit_index_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_TRANCHE``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSTranche)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CdsTranche(id='{}', attach={:.2}%, detach={:.2}%)",
            self.inner.id, self.inner.attach_pct, self.inner.detach_pct
        ))
    }
}

impl fmt::Display for PyCdsTranche {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CdsTranche({}, attach={:.2}%, detach={:.2}%)",
            self.inner.index_name, self.inner.attach_pct, self.inner.detach_pct
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsTranche>()?;
    module.add_class::<PyCdsTrancheBuilder>()?;
    Ok(vec!["CdsTranche", "CdsTrancheBuilder"])
}
