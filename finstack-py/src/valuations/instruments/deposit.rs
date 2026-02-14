use crate::core::common::args::DayCountArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::deposit::Deposit;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Money-market deposit with simple interest accrual.
///
/// Examples:
///     >>> deposit = Deposit(
///     ...     "dep_001",
///     ...     Money("USD", 1_000_000),
///     ...     date(2024, 1, 2),
///     ...     date(2024, 2, 2),
///     ...     DayCount("act_360"),
///     ...     "usd_discount"
///     ... )
///     >>> deposit.quote_rate
///     None
#[pyclass(module = "finstack.valuations.instruments", name = "Deposit", frozen)]
#[derive(Clone, Debug)]
pub struct PyDeposit {
    pub(crate) inner: Arc<Deposit>,
}

impl PyDeposit {
    pub(crate) fn new(inner: Deposit) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DepositBuilder",
    unsendable
)]
pub struct PyDepositBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<Currency>,
    start: Option<time::Date>,
    maturity: Option<time::Date>,
    day_count: finstack_core::dates::DayCount,
    discount_curve_id: Option<CurveId>,
    quote_rate: Option<f64>,
}

impl PyDepositBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            start: None,
            maturity: None,
            day_count: finstack_core::dates::DayCount::Act360,
            discount_curve_id: None,
            quote_rate: None,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
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

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.start.is_none() {
            return Err(PyValueError::new_err(
                "Start date must be provided via start().",
            ));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err(
                "Maturity date must be provided via maturity().",
            ));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "Discount curve must be provided via disc_id().",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PyDepositBuilder {
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

    #[pyo3(text_signature = "($self, start)")]
    fn start<'py>(
        mut slf: PyRefMut<'py, Self>,
        start: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start = Some(py_to_date(&start).context("start")?);
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

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(dc) = day_count.extract().context("day_count")?;
        slf.day_count = dc;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, quote_rate=None)", signature = (quote_rate=None))]
    fn quote_rate(mut slf: PyRefMut<'_, Self>, quote_rate: Option<f64>) -> PyRefMut<'_, Self> {
        slf.quote_rate = quote_rate;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyDeposit> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "DepositBuilder internal error: missing notional after validation",
            )
        })?;
        let start = slf.start.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "DepositBuilder internal error: missing start after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "DepositBuilder internal error: missing maturity after validation",
            )
        })?;
        let discount = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "DepositBuilder internal error: missing discount curve after validation",
            )
        })?;

        Deposit::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .start_date(start)
            .maturity(maturity)
            .day_count(slf.day_count)
            .discount_curve_id(discount)
            .quote_rate_opt(
                slf.quote_rate
                    .map(|rate| rust_decimal::Decimal::try_from(rate).unwrap_or_default()),
            )
            .build()
            .map(PyDeposit::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "DepositBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyDeposit {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyDepositBuilder>> {
        let py = cls.py();
        let builder = PyDepositBuilder::new_with_id(InstrumentId::new(instrument_id));
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

    /// Underlying notional amount.
    ///
    /// Returns:
    ///     Money: Notional amount wrapped in :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Start date of the deposit period.
    ///
    /// Returns:
    ///     datetime.date: Start date for interest accrual.
    #[getter]
    fn start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// Maturity date of the deposit period.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Day-count convention used for accrual.
    ///
    /// Returns:
    ///     DayCount: Day-count convention wrapper.
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    /// Optional quoted simple rate.
    ///
    /// Returns:
    ///     float | None: Quoted rate in decimal form when supplied.
    #[getter]
    fn quote_rate(&self) -> Option<f64> {
        self.inner
            .quote_rate
            .as_ref()
            .and_then(rust_decimal::prelude::ToPrimitive::to_f64)
    }

    /// Discount curve identifier used for valuation.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.DEPOSIT``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Deposit)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Deposit(id='{}', start='{}', maturity='{}', quote_rate={:?})",
            self.inner.id, self.inner.start_date, self.inner.maturity, self.inner.quote_rate
        ))
    }
}

impl fmt::Display for PyDeposit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Deposit({}, {} -> {})",
            self.inner.id, self.inner.start_date, self.inner.maturity
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDeposit>()?;
    module.add_class::<PyDepositBuilder>()?;
    Ok(vec!["Deposit", "DepositBuilder"])
}
