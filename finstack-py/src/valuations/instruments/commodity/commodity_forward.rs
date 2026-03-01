//! Python bindings for CommodityForward instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::{
    CommodityForward, Position, SettlementType,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Commodity forward or futures contract.
///
/// Represents a commitment to buy or sell a commodity at a specified future
/// date at a predetermined price. Can be physically settled (delivery) or
/// cash settled (price difference).
///
/// Examples:
///     >>> forward = (
///     ...     CommodityForward.builder("WTI-FWD-2025M03")
///     ...     .commodity_type("Energy")
///     ...     .ticker("CL")
///     ...     .quantity(1000.0)
///     ...     .unit("BBL")
///     ...     .maturity(Date(2025, 3, 15))
///     ...     .currency("USD")
///     ...     .forward_curve_id("WTI-FORWARD")
///     ...     .discount_curve_id("USD-OIS")
///     ...     .build()
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityForward",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommodityForward {
    pub(crate) inner: Arc<CommodityForward>,
}

impl PyCommodityForward {
    pub(crate) fn new(inner: CommodityForward) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityForwardBuilder",
    unsendable
)]
pub struct PyCommodityForwardBuilder {
    instrument_id: InstrumentId,
    commodity_type: Option<String>,
    ticker: Option<String>,
    quantity: Option<f64>,
    unit: Option<String>,
    maturity: Option<time::Date>,
    currency: Option<finstack_core::currency::Currency>,
    forward_curve_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    multiplier: f64,
    quoted_price: Option<f64>,
    spot_id: Option<String>,
    settlement: Option<SettlementType>,
    exchange: Option<String>,
    contract_month: Option<String>,
    position: Option<Position>,
    contract_price: Option<f64>,
}

impl PyCommodityForwardBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            commodity_type: None,
            ticker: None,
            quantity: None,
            unit: None,
            maturity: None,
            currency: None,
            forward_curve_id: None,
            discount_curve_id: None,
            multiplier: 1.0,
            quoted_price: None,
            spot_id: None,
            settlement: None,
            exchange: None,
            contract_month: None,
            position: None,
            contract_price: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.commodity_type.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("commodity_type() is required."));
        }
        if self.ticker.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("ticker() is required."));
        }
        if self.quantity.is_none() {
            return Err(PyValueError::new_err("quantity() is required."));
        }
        if self.unit.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("unit() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.forward_curve_id.is_none() {
            return Err(PyValueError::new_err("forward_curve_id() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCommodityForwardBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, commodity_type)")]
    fn commodity_type(mut slf: PyRefMut<'_, Self>, commodity_type: String) -> PyRefMut<'_, Self> {
        slf.commodity_type = Some(commodity_type);
        slf
    }

    #[pyo3(text_signature = "($self, ticker)")]
    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    #[pyo3(text_signature = "($self, quantity)")]
    fn quantity(mut slf: PyRefMut<'_, Self>, quantity: f64) -> PyResult<PyRefMut<'_, Self>> {
        if quantity <= 0.0 {
            return Err(PyValueError::new_err("quantity must be positive"));
        }
        slf.quantity = Some(quantity);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, unit)")]
    fn unit(mut slf: PyRefMut<'_, Self>, unit: String) -> PyRefMut<'_, Self> {
        slf.unit = Some(unit);
        slf
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
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

    #[pyo3(text_signature = "($self, curve_id)")]
    fn forward_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, multiplier)")]
    fn multiplier(mut slf: PyRefMut<'_, Self>, multiplier: f64) -> PyRefMut<'_, Self> {
        slf.multiplier = multiplier;
        slf
    }

    #[pyo3(text_signature = "($self, quoted_price=None)", signature = (quoted_price=None))]
    fn quoted_price(mut slf: PyRefMut<'_, Self>, quoted_price: Option<f64>) -> PyRefMut<'_, Self> {
        slf.quoted_price = quoted_price;
        slf
    }

    #[pyo3(text_signature = "($self, spot_id=None)", signature = (spot_id=None))]
    fn spot_id(mut slf: PyRefMut<'_, Self>, spot_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.spot_id = spot_id;
        slf
    }

    #[pyo3(text_signature = "($self, settlement_type=None)", signature = (settlement_type=None))]
    fn settlement_type(
        mut slf: PyRefMut<'_, Self>,
        settlement_type: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.settlement = match settlement_type.as_deref() {
            Some("physical") | Some("Physical") => Some(SettlementType::Physical),
            Some("cash") | Some("Cash") => Some(SettlementType::Cash),
            None => None,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Invalid settlement_type: '{other}'. Must be 'physical' or 'cash'",
                )))
            }
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, exchange=None)", signature = (exchange=None))]
    fn exchange(mut slf: PyRefMut<'_, Self>, exchange: Option<String>) -> PyRefMut<'_, Self> {
        slf.exchange = exchange;
        slf
    }

    #[pyo3(text_signature = "($self, contract_month=None)", signature = (contract_month=None))]
    fn contract_month(
        mut slf: PyRefMut<'_, Self>,
        contract_month: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.contract_month = contract_month;
        slf
    }

    /// Set position direction: "long" or "short" (defaults to "long").
    #[pyo3(text_signature = "($self, position=None)", signature = (position=None))]
    fn position(
        mut slf: PyRefMut<'_, Self>,
        position: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.position = match position.as_deref() {
            Some("long") | Some("Long") | Some("LONG") => Some(Position::Long),
            Some("short") | Some("Short") | Some("SHORT") => Some(Position::Short),
            None => None,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Invalid position: '{other}'. Must be 'long' or 'short'",
                )))
            }
        };
        Ok(slf)
    }

    /// Set contract entry price (for mark-to-market). If None, treated as at-market.
    #[pyo3(text_signature = "($self, contract_price=None)", signature = (contract_price=None))]
    fn contract_price(
        mut slf: PyRefMut<'_, Self>,
        contract_price: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.contract_price = contract_price;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommodityForward> {
        slf.ensure_ready()?;

        let commodity_type = slf.commodity_type.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing commodity_type after validation",
            )
        })?;
        let ticker = slf.ticker.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing ticker after validation",
            )
        })?;
        let quantity = slf.quantity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing quantity after validation",
            )
        })?;
        let unit = slf.unit.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing unit after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing maturity after validation",
            )
        })?;
        let currency = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing currency after validation",
            )
        })?;
        let forward_curve_id = slf.forward_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing forward_curve_id after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityForwardBuilder internal error: missing discount_curve_id after validation",
            )
        })?;

        let mut builder = CommodityForward::builder()
            .id(slf.instrument_id.clone())
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .quantity(quantity)
            .multiplier(slf.multiplier)
            .maturity(maturity)
            .forward_curve_id(forward_curve_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new());

        builder = builder.settlement(slf.settlement.unwrap_or(SettlementType::Cash));
        if let Some(qp) = slf.quoted_price {
            builder = builder.quoted_price_opt(Some(qp));
        }
        if let Some(sp) = slf.spot_id.clone() {
            builder = builder.spot_id_opt(Some(sp));
        }
        if let Some(ex) = slf.exchange.clone() {
            builder = builder.exchange_opt(Some(ex));
        }
        if let Some(cm) = slf.contract_month.clone() {
            builder = builder.contract_month_opt(Some(cm));
        }
        if let Some(pos) = slf.position {
            builder = builder.position(pos);
        }
        if let Some(cp) = slf.contract_price {
            builder = builder.contract_price_opt(Some(cp));
        }

        let forward = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e}")))?;

        Ok(PyCommodityForward::new(forward))
    }

    fn __repr__(&self) -> String {
        "CommodityForwardBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCommodityForward {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommodityForwardBuilder>> {
        let py = cls.py();
        let builder = PyCommodityForwardBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commodity type (e.g., "Energy", "Metal").
    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.underlying.commodity_type
    }

    /// Ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying.ticker
    }

    /// Contract quantity.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.underlying.unit
    }

    /// Contract multiplier.
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Maturity date.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.underlying.currency)
    }

    /// Optional quoted forward price.
    #[getter]
    fn quoted_price(&self) -> Option<f64> {
        self.inner.quoted_price
    }

    /// Forward curve ID.
    #[getter]
    fn forward_curve_id(&self) -> &str {
        self.inner.forward_curve_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Optional exchange.
    #[getter]
    fn exchange(&self) -> Option<&str> {
        self.inner.exchange.as_deref()
    }

    /// Optional contract month.
    #[getter]
    fn contract_month(&self) -> Option<&str> {
        self.inner.contract_month.as_deref()
    }

    /// Position direction: "long" or "short".
    #[getter]
    fn position(&self) -> &str {
        match self.inner.position {
            Position::Long => "long",
            Position::Short => "short",
        }
    }

    /// Contract entry price (for mark-to-market). None if at-market.
    #[getter]
    fn contract_price(&self) -> Option<f64> {
        self.inner.contract_price
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommodityForward(id='{}', ticker='{}', quantity={}, maturity='{}')",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.quantity,
            self.inner.maturity
        )
    }
}

impl fmt::Display for PyCommodityForward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommodityForward({}, {}, {})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.quantity
        )
    }
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityForward>()?;
    parent.add_class::<PyCommodityForwardBuilder>()?;
    Ok(())
}
