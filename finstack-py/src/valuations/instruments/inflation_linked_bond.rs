#![allow(clippy::unwrap_used)]

use crate::core::common::args::DayCountArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor, TenorUnit};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::InflationLinkedBondParams;
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_indexation_method(label: Option<&str>) -> PyResult<IndexationMethod> {
    match label {
        None => Ok(IndexationMethod::TIPS),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

fn parse_deflation_protection(label: Option<&str>) -> PyResult<DeflationProtection> {
    match label {
        None => Ok(DeflationProtection::MaturityOnly),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Inflation-linked bond binding with a convenience constructor.
///
/// Examples:
///     >>> ilb = (
///     ...     InflationLinkedBond.builder("tips_2032")
///     ...     .notional(Money("USD", 1_000_000))
///     ...     .real_coupon(0.01)
///     ...     .issue(date(2022, 1, 15))
///     ...     .maturity(date(2032, 1, 15))
///     ...     .base_index(260.0)
///     ...     .discount_curve("usd_discount")
///     ...     .inflation_curve("us_cpi")
///     ...     .build()
///     ... )
///     >>> ilb.real_coupon
///     0.01
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationLinkedBond",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInflationLinkedBond {
    pub(crate) inner: Arc<InflationLinkedBond>,
}

impl PyInflationLinkedBond {
    pub(crate) fn new(inner: InflationLinkedBond) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationLinkedBondBuilder",
    unsendable
)]
pub struct PyInflationLinkedBondBuilder {
    instrument_id: InstrumentId,
    notional: Option<finstack_core::money::Money>,
    real_coupon: Option<f64>,
    issue: Option<time::Date>,
    maturity: Option<time::Date>,
    base_index: Option<f64>,
    discount_curve: Option<CurveId>,
    inflation_curve: Option<CurveId>,
    indexation: IndexationMethod,
    frequency: Tenor,
    day_count: DayCount,
    deflation_protection: DeflationProtection,
    calendar: Option<String>,
}

impl PyInflationLinkedBondBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            notional: None,
            real_coupon: None,
            issue: None,
            maturity: None,
            base_index: None,
            discount_curve: None,
            inflation_curve: None,
            indexation: IndexationMethod::TIPS,
            frequency: crate::valuations::common::parse_frequency_label(Some("semi_annual"))
                .unwrap_or_else(|_| Tenor::new(6, TenorUnit::Months)),
            day_count: DayCount::ActAct,
            deflation_protection: DeflationProtection::MaturityOnly,
            calendar: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.real_coupon.is_none() {
            return Err(PyValueError::new_err("real_coupon() is required."));
        }
        if self.issue.is_none() {
            return Err(PyValueError::new_err("issue() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.base_index.is_none() {
            return Err(PyValueError::new_err("base_index() is required."));
        }
        if self.discount_curve.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.inflation_curve.is_none() {
            return Err(PyValueError::new_err("inflation_curve() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyInflationLinkedBondBuilder {
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
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, real_coupon)")]
    fn real_coupon(mut slf: PyRefMut<'_, Self>, real_coupon: f64) -> PyRefMut<'_, Self> {
        slf.real_coupon = Some(real_coupon);
        slf
    }

    #[pyo3(text_signature = "($self, issue)")]
    fn issue<'py>(
        mut slf: PyRefMut<'py, Self>,
        issue: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.issue = Some(py_to_date(&issue).context("issue")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, base_index)")]
    fn base_index(mut slf: PyRefMut<'_, Self>, base_index: f64) -> PyRefMut<'_, Self> {
        slf.base_index = Some(base_index);
        slf
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(discount_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, inflation_curve)")]
    fn inflation_curve(mut slf: PyRefMut<'_, Self>, inflation_curve: String) -> PyRefMut<'_, Self> {
        slf.inflation_curve = Some(CurveId::new(inflation_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, indexation)")]
    fn indexation(mut slf: PyRefMut<'_, Self>, indexation: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.indexation = parse_indexation_method(Some(indexation.as_str()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency(mut slf: PyRefMut<'_, Self>, frequency: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.frequency = crate::valuations::common::parse_frequency_label(Some(frequency.as_str()))
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract().map_err(|e| {
            pyo3::exceptions::PyTypeError::new_err(format!("day_count expects DayCount: {e}"))
        })?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, deflation_protection)")]
    fn deflation_protection(
        mut slf: PyRefMut<'_, Self>,
        deflation_protection: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.deflation_protection = parse_deflation_protection(Some(deflation_protection.as_str()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, calendar=None)", signature = (calendar=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar = calendar;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyInflationLinkedBond> {
        slf.ensure_ready()?;
        let params = InflationLinkedBondParams::new(
            slf.notional.unwrap(),
            slf.real_coupon.unwrap(),
            slf.issue.unwrap(),
            slf.maturity.unwrap(),
            slf.base_index.unwrap(),
            slf.frequency,
            slf.day_count,
        );

        let mut builder = InflationLinkedBond::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.notional(params.notional);
        builder = builder.real_coupon(params.real_coupon);
        builder = builder.frequency(params.frequency);
        builder = builder.day_count(params.day_count);
        builder = builder.issue_date(params.issue);
        builder = builder.maturity(params.maturity);
        builder = builder.base_index(params.base_index);
        builder = builder.base_date(params.issue);
        builder = builder.indexation_method(slf.indexation);
        builder = builder.lag(slf.indexation.standard_lag());
        builder = builder.deflation_protection(slf.deflation_protection);
        builder = builder.bdc(BusinessDayConvention::Following);
        builder = builder.stub(StubKind::None);
        builder = builder.calendar_id_opt(slf.calendar.clone());
        builder = builder.discount_curve_id(slf.discount_curve.clone().unwrap());
        builder = builder.inflation_index_id(slf.inflation_curve.clone().unwrap());
        builder = builder.attributes(Default::default());

        let bond = builder.build().map_err(core_to_py)?;
        Ok(PyInflationLinkedBond::new(bond))
    }

    fn __repr__(&self) -> String {
        "InflationLinkedBondBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyInflationLinkedBond {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyInflationLinkedBondBuilder>> {
        let py = cls.py();
        let builder = PyInflationLinkedBondBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the bond.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Real coupon rate in decimal form.
    ///
    /// Returns:
    ///     float: Real coupon rate.
    #[getter]
    fn real_coupon(&self) -> f64 {
        self.inner.real_coupon
    }

    /// Maturity date of the bond.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
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

    /// Inflation curve identifier.
    ///
    /// Returns:
    ///     str: Inflation curve used for indexation.
    #[getter]
    fn inflation_curve(&self) -> String {
        self.inner.inflation_index_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.INFLATION_LINKED_BOND``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::InflationLinkedBond)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InflationLinkedBond(id='{}', coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        ))
    }
}

impl fmt::Display for PyInflationLinkedBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InflationLinkedBond({}, coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInflationLinkedBond>()?;
    module.add_class::<PyInflationLinkedBondBuilder>()?;
    Ok(vec!["InflationLinkedBond", "InflationLinkedBondBuilder"])
}
