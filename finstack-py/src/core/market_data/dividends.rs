//! Dividends: immutable events, schedules, and a fluent builder for Python.
//!
//! Exposes `DividendEvent` (cash, yield, stock), `DividendSchedule` (ordered
//! events for an equity/index), and `DividendScheduleBuilder` for ergonomic
//! construction. Cash events carry `Money` with a `Currency`; yield and stock
//! variants carry numeric values. Schedules are immutable once built.
//!
//! Example
//! -------
//! ```text
//! builder = DividendScheduleBuilder("AAPL")
//! builder.currency(Currency("USD"))
//! builder.cash(date(2025, 2, 15), Money(0.24, "USD"))
//! schedule = builder.build()
//! ```
use crate::core::currency::PyCurrency;
use crate::errors::core_to_py;
use crate::core::money::PyMoney;
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::market_data::dividends::{
    DividendEvent, DividendKind, DividendSchedule, DividendScheduleBuilder,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;
use time::Date;

/// Immutable dividend event exposed to Python.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Payment or ex-dividend date for the event.
/// kind : {'cash', 'yield', 'stock'}
///     Dividend type including cash amount, yield, or stock ratio.
///
/// Returns
/// -------
/// DividendEvent
///     Dividend record used within :class:`DividendSchedule`.
#[pyclass(
    module = "finstack.core.market_data.dividends",
    name = "DividendEvent",
    frozen
)]
#[derive(Clone)]
pub struct PyDividendEvent {
    pub(crate) date: Date,
    pub(crate) kind: DividendKind,
}

impl PyDividendEvent {
    pub(crate) fn new(event: &DividendEvent) -> Self {
        Self {
            date: event.date,
            kind: event.kind.clone(),
        }
    }
}

#[pymethods]
impl PyDividendEvent {
    #[getter]
    /// Event date (payment or ex-dividend depending on kind).
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Date associated with the dividend.
    fn date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.date)
    }

    #[getter]
    /// Dividend kind identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     One of ``"cash"``, ``"yield"`` or ``"stock"``.
    fn kind(&self) -> &'static str {
        match self.kind {
            DividendKind::Cash(_) => "cash",
            DividendKind::Yield(_) => "yield",
            DividendKind::Stock { .. } => "stock",
        }
    }

    #[getter]
    /// Cash amount for cash dividends.
    ///
    /// Returns
    /// -------
    /// Money or None
    ///     Cash dividend amount when ``kind == "cash"``.
    fn cash_amount(&self) -> Option<PyMoney> {
        match &self.kind {
            DividendKind::Cash(m) => Some(PyMoney::new(*m)),
            _ => None,
        }
    }

    #[getter]
    /// Dividend yield for yield-based dividends.
    ///
    /// Returns
    /// -------
    /// float or None
    ///     Yield in decimal form when applicable.
    fn dividend_yield(&self) -> Option<f64> {
        match self.kind {
            DividendKind::Yield(v) => Some(v),
            _ => None,
        }
    }

    #[getter]
    /// Stock ratio for stock dividends.
    ///
    /// Returns
    /// -------
    /// float or None
    ///     Ratio of shares distributed when ``kind == "stock"``.
    fn stock_ratio(&self) -> Option<f64> {
        match self.kind {
            DividendKind::Stock { ratio } => Some(ratio),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.kind {
            DividendKind::Cash(m) => {
                format!("DividendEvent(date={:?}, cash={})", self.date, m)
            }
            DividendKind::Yield(v) => {
                format!("DividendEvent(date={:?}, yield={v})", self.date)
            }
            DividendKind::Stock { ratio } => {
                format!("DividendEvent(date={:?}, stock_ratio={ratio})", self.date)
            }
        }
    }
}

/// Sequence of dividend events for an equity or index.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the dividend schedule.
/// underlying : str, optional
///     Underlying symbol associated with the events.
/// currency : Currency, optional
///     Currency for cash dividends.
///
/// Returns
/// -------
/// DividendSchedule
///     Dividend schedule containing events and helper views.
#[pyclass(
    module = "finstack.core.market_data.dividends",
    name = "DividendSchedule",
    unsendable
)]
#[derive(Clone)]
pub struct PyDividendSchedule {
    pub(crate) inner: Arc<DividendSchedule>,
}

impl PyDividendSchedule {
    pub(crate) fn new(schedule: DividendSchedule) -> Self {
        Self {
            inner: Arc::new(schedule),
        }
    }
}

#[pymethods]
impl PyDividendSchedule {
    #[getter]
    /// Schedule identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Unique schedule id.
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    /// Underlying instrument identifier if provided.
    ///
    /// Returns
    /// -------
    /// str or None
    ///     Underlying code.
    fn underlying(&self) -> Option<String> {
        self.inner.underlying.clone()
    }

    #[getter]
    /// Currency for cash dividends when available.
    ///
    /// Returns
    /// -------
    /// Currency or None
    ///     Cash dividend currency.
    fn currency(&self) -> Option<PyCurrency> {
        self.inner.currency.map(PyCurrency::new)
    }

    #[getter]
    /// List of dividend events in chronological order.
    ///
    /// Returns
    /// -------
    /// list[DividendEvent]
    ///     Events included in the schedule.
    fn events(&self) -> Vec<PyDividendEvent> {
        self.inner.events.iter().map(PyDividendEvent::new).collect()
    }

    #[getter]
    /// Convenience view containing only cash events.
    ///
    /// Returns
    /// -------
    /// list[tuple[datetime.date, Money]]
    ///     Cash dividend payments.
    fn cash_events(&self, py: Python<'_>) -> PyResult<Vec<(PyObject, PyMoney)>> {
        self.inner
            .cash_events()
            .map(|(date, money)| Ok((date_to_py(py, date)?, PyMoney::new(*money))))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "DividendSchedule(id='{}', events={})",
            self.inner.id,
            self.inner.events.len()
        )
    }
}

/// Builder used to construct dividend schedules incrementally.
///
/// Parameters
/// ----------
/// id : str
///     Schedule identifier provided to :py:meth:`DividendScheduleBuilder.new`.
///
/// Returns
/// -------
/// DividendScheduleBuilder
///     Mutable builder that emits :class:`DividendSchedule` via :py:meth:`DividendScheduleBuilder.build`.
#[pyclass(
    module = "finstack.core.market_data.dividends",
    name = "DividendScheduleBuilder",
    unsendable
)]
pub struct PyDividendScheduleBuilder {
    inner: Option<DividendScheduleBuilder>,
}

#[pymethods]
impl PyDividendScheduleBuilder {
    #[new]
    #[pyo3(text_signature = "(id)")]
    /// Create a new dividend schedule builder.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Identifier for the resulting schedule.
    ///
    /// Returns
    /// -------
    /// DividendScheduleBuilder
    ///     Builder ready for chaining.
    fn new(id: &str) -> Self {
        Self {
            inner: Some(DividendScheduleBuilder::new(id)),
        }
    }

    #[pyo3(text_signature = "(self, underlying)")]
    /// Set the underlying identifier associated with the schedule.
    ///
    /// Parameters
    /// ----------
    /// underlying : str
    ///     Underlying symbol.
    ///
    /// Returns
    /// -------
    /// None
    fn underlying(&mut self, underlying: &str) {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.underlying(underlying));
        }
    }

    #[pyo3(text_signature = "(self, currency)")]
    /// Set the cash dividend currency.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency
    ///     Currency for subsequent cash events.
    ///
    /// Returns
    /// -------
    /// None
    fn currency(&mut self, currency: &PyCurrency) {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.currency(currency.inner));
        }
    }

    #[pyo3(text_signature = "(self, date, amount)")]
    /// Add a cash dividend event.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Payment date.
    /// amount : Money
    ///     Cash amount.
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> builder.cash(date(2024, 2, 15), Money(0.24, "USD"))
    fn cash(&mut self, date: Bound<'_, PyAny>, amount: &PyMoney) -> PyResult<()> {
        let d = py_to_date(&date)?;
        let builder = self
            .inner
            .take()
            .expect("builder should exist during chaining");
        self.inner = Some(builder.cash(d, amount.inner));
        Ok(())
    }

    #[pyo3(text_signature = "(self, date, yield_value)")]
    /// Add a dividend expressed as forward yield.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Ex-dividend date.
    /// yield_value : float
    ///     Dividend yield in decimal form.
    ///
    /// Returns
    /// -------
    /// None
    fn yield_div(&mut self, date: Bound<'_, PyAny>, yield_value: f64) -> PyResult<()> {
        let d = py_to_date(&date)?;
        let builder = self
            .inner
            .take()
            .expect("builder should exist during chaining");
        self.inner = Some(builder.yield_div(d, yield_value));
        Ok(())
    }

    #[pyo3(text_signature = "(self, date, ratio)")]
    /// Add a stock dividend specified by share ratio.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Ex-dividend date.
    /// ratio : float
    ///     Share distribution ratio.
    ///
    /// Returns
    /// -------
    /// None
    fn stock(&mut self, date: Bound<'_, PyAny>, ratio: f64) -> PyResult<()> {
        let d = py_to_date(&date)?;
        let builder = self
            .inner
            .take()
            .expect("builder should exist during chaining");
        self.inner = Some(builder.stock(d, ratio));
        Ok(())
    }

    #[pyo3(text_signature = "(self)")]
    /// Finalize the builder and return a dividend schedule.
    ///
    /// Returns
    /// -------
    /// DividendSchedule
    ///     Immutable schedule containing all accumulated events.
    fn build(&mut self) -> PyResult<PyDividendSchedule> {
        let builder = self
            .inner
            .take()
            .expect("builder should exist during chaining");
        let schedule = builder.build().map_err(core_to_py)?;
        self.inner = Some(DividendScheduleBuilder::new(schedule.id.clone()));
        Ok(PyDividendSchedule::new(schedule))
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "dividends")?;
    module.setattr(
        "__doc__",
        "Dividend schedules shared across equity valuations.",
    )?;
    module.add_class::<PyDividendEvent>()?;
    module.add_class::<PyDividendSchedule>()?;
    module.add_class::<PyDividendScheduleBuilder>()?;
    let exports = [
        "DividendEvent",
        "DividendSchedule",
        "DividendScheduleBuilder",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
