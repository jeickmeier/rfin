use crate::core::common::args::DayCountArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::legs::PayReceive;
use finstack_valuations::instruments::rates::fra::ForwardRateAgreement;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Forward Rate Agreement binding exposing standard FRA parameters.
///
/// Examples:
///     >>> fra = (
///     ...     ForwardRateAgreement.builder("fra_3x6")
///     ...     .money(Money("USD", 5_000_000))
///     ...     .fixed_rate(0.035)
///     ...     .fixing_date(date(2024, 3, 15))
///     ...     .start_date(date(2024, 6, 17))
///     ...     .end_date(date(2024, 9, 16))
///     ...     .disc_id("usd_discount")
///     ...     .fwd_id("usd_libor_3m")
///     ...     .build()
///     ... )
///     >>> fra.fixed_rate
///     0.035
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ForwardRateAgreement",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyForwardRateAgreement {
    pub(crate) inner: Arc<ForwardRateAgreement>,
}

impl PyForwardRateAgreement {
    pub(crate) fn new(inner: ForwardRateAgreement) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ForwardRateAgreementBuilder",
    unsendable
)]
pub struct PyForwardRateAgreementBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<Currency>,
    fixed_rate: Option<f64>,
    fixing_date: Option<time::Date>,
    start_date: Option<time::Date>,
    end_date: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    forward_curve_id: Option<CurveId>,
    day_count: finstack_core::dates::DayCount,
    reset_lag: i32,
    receive_fixed: bool,
}

impl PyForwardRateAgreementBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            fixed_rate: None,
            fixing_date: None,
            start_date: None,
            end_date: None,
            discount_curve_id: None,
            forward_curve_id: None,
            day_count: finstack_core::dates::DayCount::Act360,
            reset_lag: 2,
            receive_fixed: true,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.fixed_rate.is_none() {
            return Err(PyValueError::new_err(
                "Fixed rate must be provided via fixed_rate().",
            ));
        }
        if self.fixing_date.is_none() {
            return Err(PyValueError::new_err(
                "Fixing date must be provided via fixing_date().",
            ));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err(
                "Start date must be provided via start_date().",
            ));
        }
        if self.end_date.is_none() {
            return Err(PyValueError::new_err(
                "End date must be provided via end_date().",
            ));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "Discount curve must be provided via disc_id().",
            ));
        }
        if self.forward_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "Forward curve must be provided via fwd_id().",
            ));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyForwardRateAgreementBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pending_currency = Some(Self::parse_currency(currency)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, rate)")]
    fn fixed_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyResult<PyRefMut<'_, Self>> {
        if rate < 0.0 {
            return Err(PyValueError::new_err("fixed_rate must be non-negative"));
        }
        slf.fixed_rate = Some(rate);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixing_date)")]
    fn fixing_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        fixing_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.fixing_date = Some(py_to_date(&fixing_date).context("fixing_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, start_date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        start_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&start_date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, end_date)")]
    fn end_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        end_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.end_date = Some(py_to_date(&end_date).context("end_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn fwd_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(dc) = day_count.extract().context("day_count")?;
        slf.day_count = dc;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, reset_lag)")]
    fn reset_lag(mut slf: PyRefMut<'_, Self>, reset_lag: i32) -> PyRefMut<'_, Self> {
        slf.reset_lag = reset_lag;
        slf
    }

    /// Set the FRA direction: True = receive fixed rate, False = pay fixed rate.
    #[pyo3(text_signature = "($self, receive_fixed)")]
    fn receive_fixed(mut slf: PyRefMut<'_, Self>, receive_fixed: bool) -> PyRefMut<'_, Self> {
        slf.receive_fixed = receive_fixed;
        slf
    }

    #[pyo3(text_signature = "($self, pay_fixed)")]
    fn pay_fixed(mut slf: PyRefMut<'_, Self>, pay_fixed: bool) -> PyRefMut<'_, Self> {
        slf.receive_fixed = !pay_fixed;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyForwardRateAgreement> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing notional after validation",
            )
        })?;
        let fixed_rate = slf.fixed_rate.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing fixed_rate after validation",
            )
        })?;
        let fixing_date = slf.fixing_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing fixing_date after validation",
            )
        })?;
        let start_date = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing start_date after validation",
            )
        })?;
        let end_date = slf.end_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing end_date after validation",
            )
        })?;
        let discount = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing discount curve after validation",
            )
        })?;
        let forward = slf.forward_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "ForwardRateAgreementBuilder internal error: missing forward curve after validation",
            )
        })?;

        ForwardRateAgreement::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .fixed_rate(rust_decimal::Decimal::try_from(fixed_rate).unwrap_or_default())
            .fixing_date(fixing_date)
            .start_date(start_date)
            .maturity(end_date)
            .day_count(slf.day_count)
            .reset_lag(slf.reset_lag)
            .discount_curve_id(discount)
            .forward_curve_id(forward)
            .side(if slf.receive_fixed {
                PayReceive::ReceiveFixed
            } else {
                PayReceive::PayFixed
            })
            .build()
            .map(PyForwardRateAgreement::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "ForwardRateAgreementBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyForwardRateAgreement {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyForwardRateAgreementBuilder>> {
        let py = cls.py();
        let builder = PyForwardRateAgreementBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// FRA fixed rate as decimal.
    ///
    /// Returns:
    ///     float: Fixed rate paid or received on the FRA.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.fixed_rate).unwrap_or_default()
    }

    /// Day-count convention used for accrual.
    ///
    /// Returns:
    ///     DayCount: Day-count convention wrapper.
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    /// Reset lag in business days.
    ///
    /// Returns:
    ///     int: Number of business days prior to start when the rate fixes.
    #[getter]
    fn reset_lag(&self) -> i32 {
        self.inner.reset_lag
    }

    /// Whether the FRA receives fixed rate / pays floating rate.
    ///
    /// Returns:
    ///     bool: ``True`` when the FRA position receives fixed rate.
    #[getter]
    fn receive_fixed(&self) -> bool {
        self.inner.side.is_receiver()
    }

    #[getter]
    fn pay_fixed(&self) -> bool {
        !self.inner.side.is_receiver()
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Forward curve identifier.
    ///
    /// Returns:
    ///     str: Forward curve used for projecting floating rates.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    /// Fixing date for the reference rate.
    ///
    /// Returns:
    ///     Optional[datetime.date]: Date on which the floating rate is observed,
    ///     or None if inferred from start_date - reset_lag.
    #[getter]
    fn fixing_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.fixing_date {
            Some(date) => date_to_py(py, date).map(Some),
            None => Ok(None),
        }
    }

    /// Start date of the accrual period.
    ///
    /// Returns:
    ///     datetime.date: Accrual start date converted to Python.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date of the accrual period.
    ///
    /// Returns:
    ///     datetime.date: Accrual end date converted to Python.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Notional amount for the FRA.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Instrument type enum (``InstrumentType.FRA``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.FRA``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FRA)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "ForwardRateAgreement(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        ))
    }
}

impl fmt::Display for PyForwardRateAgreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FRA({}, rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyForwardRateAgreement>()?;
    module.add_class::<PyForwardRateAgreementBuilder>()?;
    Ok(vec!["ForwardRateAgreement", "ForwardRateAgreementBuilder"])
}
