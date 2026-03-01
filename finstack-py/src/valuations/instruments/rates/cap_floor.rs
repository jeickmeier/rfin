use crate::core::common::args::DayCountArg;
// use crate::errors::core_to_py; // not used in this module currently
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::PyContext;
use crate::valuations::common::{frequency_from_payments_per_year, PyInstrumentType};
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::cap_floor::InterestRateOption;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn extract_day_count(dc: Option<Bound<'_, PyAny>>) -> PyResult<DayCount> {
    if let Some(bound) = dc {
        let DayCountArg(inner) = bound.extract()?;
        Ok(inner)
    } else {
        Ok(DayCount::Act360)
    }
}

/// Interest rate cap/floor instruments using Black pricing.
///
/// Examples:
///     >>> cap = (
///     ...     InterestRateOption.builder("cap_1")
///     ...     .kind("cap")
///     ...     .money(Money("USD", 5_000_000))
///     ...     .strike(0.035)
///     ...     .start_date(date(2024, 1, 1))
///     ...     .end_date(date(2027, 1, 1))
///     ...     .disc_id("usd_discount")
///     ...     .fwd_id("usd_libor_3m")
///     ...     .vol_surface("usd_cap_vol")
///     ...     .build()
///     ... )
///     >>> cap.strike
///     0.035
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateOption {
    pub(crate) inner: Arc<InterestRateOption>,
}

impl PyInterestRateOption {
    pub(crate) fn new(inner: InterestRateOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateOptionBuilder",
    unsendable
)]
pub struct PyInterestRateOptionBuilder {
    instrument_id: InstrumentId,
    is_cap: bool,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    start_date: Option<time::Date>,
    end_date: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    forward_curve_id: Option<CurveId>,
    vol_surface_id: Option<String>,
    payments_per_year: u32,
    day_count: DayCount,
}

impl PyInterestRateOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            is_cap: true,
            pending_notional_amount: None,
            pending_currency: None,
            strike: None,
            start_date: None,
            end_date: None,
            discount_curve_id: None,
            forward_curve_id: None,
            vol_surface_id: None,
            payments_per_year: 4,
            day_count: DayCount::Act360,
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
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.end_date.is_none() {
            return Err(PyValueError::new_err("end_date() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        if self.forward_curve_id.is_none() {
            return Err(PyValueError::new_err("fwd_id() is required."));
        }
        if self.vol_surface_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("vol_surface() is required."));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::currency::Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<finstack_core::currency::Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyInterestRateOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, kind)")]
    fn kind(mut slf: PyRefMut<'_, Self>, kind: String) -> PyResult<PyRefMut<'_, Self>> {
        match kind.to_lowercase().as_str() {
            "cap" => slf.is_cap = true,
            "floor" => slf.is_cap = false,
            other => {
                return Err(PyValueError::new_err(format!(
                    "kind must be 'cap' or 'floor' (got '{other}')"
                )))
            }
        }
        Ok(slf)
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

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
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

    #[pyo3(text_signature = "($self, vol_surface)")]
    fn vol_surface(mut slf: PyRefMut<'_, Self>, vol_surface: String) -> PyRefMut<'_, Self> {
        slf.vol_surface_id = Some(vol_surface);
        slf
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
        let dc = extract_day_count(Some(day_count))?;
        slf.day_count = dc;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyInterestRateOption> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing notional after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let start = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing start_date after validation",
            )
        })?;
        let end = slf.end_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing end_date after validation",
            )
        })?;
        let disc = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing discount curve after validation",
            )
        })?;
        let fwd = slf.forward_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing forward curve after validation",
            )
        })?;
        let vol_surface_id = slf.vol_surface_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateOptionBuilder internal error: missing vol surface after validation",
            )
        })?;
        let freq = frequency_from_payments_per_year(Some(slf.payments_per_year))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let option = if slf.is_cap {
            InterestRateOption::new_cap(
                slf.instrument_id.clone(),
                notional,
                strike,
                start,
                end,
                freq,
                slf.day_count,
                disc,
                fwd,
                vol_surface_id.as_str(),
            )
        } else {
            InterestRateOption::new_floor(
                slf.instrument_id.clone(),
                notional,
                strike,
                start,
                end,
                freq,
                slf.day_count,
                disc,
                fwd,
                vol_surface_id.as_str(),
            )
        };
        Ok(PyInterestRateOption::new(option))
    }

    fn __repr__(&self) -> String {
        "InterestRateOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyInterestRateOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyInterestRateOptionBuilder>> {
        let py = cls.py();
        let builder = PyInterestRateOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the cap/floor.
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

    /// Strike rate in decimal form.
    ///
    /// Returns:
    ///     float: Strike rate of the instrument.
    #[getter]
    fn strike(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.strike).unwrap_or_default()
    }

    /// Start date for accrual.
    ///
    /// Returns:
    ///     datetime.date: Start date converted to Python.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date for accrual.
    ///
    /// Returns:
    ///     datetime.date: End date converted to Python.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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

    /// Forward curve identifier.
    ///
    /// Returns:
    ///     str: Forward curve used for rate projections.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface used for option pricing.
    #[getter]
    fn vol_surface(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CAP_FLOOR``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CapFloor)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InterestRateOption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike
        ))
    }
}

impl fmt::Display for PyInterestRateOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InterestRateOption({}, strike={:.4})",
            self.inner.id, self.inner.strike
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInterestRateOption>()?;
    module.add_class::<PyInterestRateOptionBuilder>()?;
    Ok(vec!["InterestRateOption", "InterestRateOptionBuilder"])
}
