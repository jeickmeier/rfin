//! Rust source: `finstack/valuations/src/instruments/equity/spot/`
//! Python uses the instrument name (`equity`) instead of the Rust submodule (`spot`).

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::Equity;
use finstack_valuations::prelude::Instrument;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Spot equity position with optional share count and price override.
///
/// Examples:
///     >>> equity = Equity.builder("eq_us_apple").ticker("AAPL").currency("USD").shares(100).price(185.5).build()
///     >>> equity.shares
///     100.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Equity",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEquity {
    pub(crate) inner: Arc<Equity>,
}

impl PyEquity {
    pub(crate) fn new(inner: Equity) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

/// Fluent builder for Equity (builder-only API).
#[pyclass(module = "finstack.valuations.instruments", name = "EquityBuilder")]
pub struct PyEquityBuilder {
    instrument_id: InstrumentId,
    ticker: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    shares: Option<f64>,
    price: Option<f64>,
    price_id: Option<String>,
    div_yield_id: Option<String>,
    discount_curve_id: Option<CurveId>,
}

impl PyEquityBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            ticker: None,
            currency: None,
            shares: None,
            price: None,
            price_id: None,
            div_yield_id: None,
            discount_curve_id: None,
        }
    }
}

#[pymethods]
impl PyEquityBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, ticker)")]
    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, shares)")]
    fn shares(mut slf: PyRefMut<'_, Self>, shares: f64) -> PyRefMut<'_, Self> {
        slf.shares = Some(shares);
        slf
    }

    #[pyo3(text_signature = "($self, price)")]
    fn price(mut slf: PyRefMut<'_, Self>, price: f64) -> PyRefMut<'_, Self> {
        slf.price = Some(price);
        slf
    }

    #[pyo3(text_signature = "($self, price_id)")]
    fn price_id(mut slf: PyRefMut<'_, Self>, price_id: String) -> PyRefMut<'_, Self> {
        slf.price_id = Some(price_id);
        slf
    }

    #[pyo3(text_signature = "($self, div_yield_id)")]
    fn div_yield_id(mut slf: PyRefMut<'_, Self>, div_yield_id: String) -> PyRefMut<'_, Self> {
        slf.div_yield_id = Some(div_yield_id);
        slf
    }

    #[pyo3(text_signature = "($self, discount_curve_id)")]
    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(discount_curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyEquity> {
        let ticker = slf.ticker.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("EquityBuilder: ticker() is required")
        })?;
        let currency = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("EquityBuilder: currency() is required")
        })?;

        let mut equity = Equity::new(slf.instrument_id.clone().into_string(), ticker, currency);
        if let Some(qty) = slf.shares {
            equity = equity.with_shares(qty);
        }
        if let Some(px) = slf.price {
            equity = equity.with_price(px);
        }
        if let Some(pid) = slf.price_id.as_deref() {
            equity = equity.with_price_id(pid);
        }
        if let Some(did) = slf.div_yield_id.as_deref() {
            equity = equity.with_dividend_yield_id(did);
        }
        if let Some(ref dc_id) = slf.discount_curve_id {
            equity.discount_curve_id = dc_id.clone();
        }

        Ok(PyEquity::new(equity))
    }

    fn __repr__(&self) -> String {
        "EquityBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyEquity {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyEquityBuilder>> {
        let py = cls.py();
        let builder = PyEquityBuilder::new_with_id(InstrumentId::new(instrument_id));
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

    /// Equity ticker symbol.
    ///
    /// Returns:
    ///     str: Listing symbol of the underlying equity.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.ticker
    }

    /// Quotation currency.
    ///
    /// Returns:
    ///     Currency: Currency wrapper representing the quotation currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Number of shares (defaults to 1 when unspecified).
    ///
    /// Returns:
    ///     float: Share count used for valuation.
    #[getter]
    fn shares(&self) -> f64 {
        self.inner.effective_shares()
    }

    /// Explicit price quote if provided.
    ///
    /// Returns:
    ///     float | None: Price override per share when supplied.
    #[getter]
    fn price_quote(&self) -> Option<f64> {
        self.inner.price_quote
    }

    /// Preferred market data identifier for spot resolution when provided.
    ///
    /// Returns:
    ///     str | None: Market data key for retrieving spot price.
    #[getter]
    fn price_id(&self) -> Option<&str> {
        self.inner.price_id.as_deref()
    }

    /// Preferred market data identifier for dividend yield resolution when provided.
    ///
    /// Returns:
    ///     str | None: Market data key for retrieving dividend yield.
    #[getter]
    fn div_yield_id(&self) -> Option<&str> {
        self.inner.div_yield_id.as_deref()
    }

    /// Instrument type enum (``InstrumentType.EQUITY``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Equity)
    }

    #[pyo3(signature = (market, as_of))]
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    #[pyo3(signature = (market, as_of))]
    fn price_per_share(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.price_per_share(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    #[pyo3(signature = (market))]
    fn dividend_yield(&self, py: Python<'_>, market: &PyMarketContext) -> PyResult<f64> {
        py.detach(|| self.inner.dividend_yield(&market.inner))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of, t))]
    fn forward_price_per_share(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        t: f64,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.forward_price_per_share(&market.inner, date, t))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    #[pyo3(signature = (market, as_of, t))]
    fn forward_value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        t: f64,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.forward_value(&market.inner, date, t))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Equity(id='{}', ticker='{}', shares={})",
            self.inner.id,
            self.inner.ticker,
            self.shares()
        ))
    }
}

impl fmt::Display for PyEquity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Equity({}, ticker={}, shares={})",
            self.inner.id,
            self.inner.ticker,
            self.shares()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyEquity>()?;
    module.add_class::<PyEquityBuilder>()?;
    Ok(vec!["Equity", "EquityBuilder"])
}
