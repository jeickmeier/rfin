use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
// use crate::core::money::PyMoney; // not used in this module
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::equity::Equity;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;
use std::sync::Arc;

/// Spot equity position with optional share count and price override.
///
/// Examples:
///     >>> equity = Equity.create(
///     ...     "eq_us_apple",
///     ...     "AAPL",
///     ...     "USD",
///     ...     shares=100,
///     ...     price=185.5
///     ... )
///     >>> equity.shares
///     100.0
#[pyclass(module = "finstack.valuations.instruments", name = "Equity", frozen)]
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

#[pymethods]
impl PyEquity {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, currency, *, shares=None, price=None, price_id=None, div_yield_id=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            ticker,
            currency,
            *,
            shares=None,
            price=None,
            price_id=None,
            div_yield_id=None
        )
    )]
    /// Create an equity instrument optionally specifying share count and price.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     ticker: Equity ticker symbol (e.g., ``"AAPL"``).
    ///     currency: Currency for quotation, supplied as a currency wrapper or code.
    ///     shares: Optional number of shares held.
    ///     price: Optional price override per share.
    ///     price_id: Optional market data identifier resolving spot price.
    ///     div_yield_id: Optional market data identifier for dividend yield.
    ///
    /// Returns:
    ///     Equity: Configured equity instrument ready for pricing.
    ///
    /// Raises:
    ///     ValueError: If identifiers or currency inputs cannot be parsed.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        currency: Bound<'_, PyAny>,
        shares: Option<f64>,
        price: Option<f64>,
        price_id: Option<&str>,
        div_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let mut equity = Equity::new(id.into_string(), ticker, ccy);
        if let Some(qty) = shares {
            equity = equity.with_shares(qty);
        }
        if let Some(px) = price {
            equity = equity.with_price(px);
        }
        if let Some(pid) = price_id {
            equity = equity.with_price_id(pid);
        }
        if let Some(did) = div_yield_id {
            equity = equity.with_dividend_yield_id(did);
        }
        Ok(Self::new(equity))
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
    Ok(vec!["Equity"])
}
