use crate::core::common::args::DayCountArg;
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::deposit::Deposit;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use pyo3::Bound;
use std::fmt;

/// Money-market deposit with simple interest accrual.
#[pyclass(module = "finstack.valuations.instruments", name = "Deposit", frozen)]
#[derive(Clone, Debug)]
pub struct PyDeposit {
    pub(crate) inner: Deposit,
}

impl PyDeposit {
    pub(crate) fn new(inner: Deposit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDeposit {
    #[new]
    #[pyo3(
        text_signature = "(instrument_id, notional, start, end, day_count, discount_curve, quote_rate=None)"
    )]
    /// Create a deposit with explicit start/end dates and optional quoted rate.
    fn ctor(
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        day_count: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        quote_rate: Option<f64>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        let DayCountArg(dc) = day_count.extract()?;
        let disc = extract_curve_id(&discount_curve)?;
        Deposit::builder()
            .id(id)
            .notional(amt)
            .start(start_date)
            .end(end_date)
            .day_count(dc)
            .disc_id(disc)
            .quote_rate_opt(quote_rate)
            .build()
            .map(Self::new)
            .map_err(core_to_py)
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Start date of the deposit period.
    #[getter]
    fn start(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start)
    }

    /// End date of the deposit period.
    #[getter]
    fn end(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end)
    }

    /// Day-count convention used for accrual.
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    /// Optional quoted simple rate.
    #[getter]
    fn quote_rate(&self) -> Option<f64> {
        self.inner.quote_rate
    }

    /// Discount curve identifier used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.DEPOSIT``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Deposit)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Deposit(id='{}', start='{}', end='{}', quote_rate={:?})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.quote_rate
        ))
    }
}

impl fmt::Display for PyDeposit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Deposit({}, {} -> {})",
            self.inner.id, self.inner.start, self.inner.end
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDeposit>()?;
    Ok(vec!["Deposit"])
}
