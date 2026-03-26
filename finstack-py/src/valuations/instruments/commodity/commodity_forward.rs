//! Python bindings for CommodityForward instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::{
    CommodityForward, Position, SettlementType,
};
use finstack_valuations::instruments::common::parameters::CommodityConvention;
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
    convention: Option<CommodityConvention>,
    settlement_lag_days: Option<u32>,
    settlement_calendar_id: Option<String>,
    settlement_bdc: Option<BusinessDayConvention>,
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
            convention: None,
            settlement_lag_days: None,
            settlement_calendar_id: None,
            settlement_bdc: None,
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

    #[pyo3(text_signature = "($self, convention=None)", signature = (convention=None))]
    fn convention(
        mut slf: PyRefMut<'_, Self>,
        convention: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.convention = match convention.map(|s| normalize_label(&s)).as_deref() {
            Some("wticrude") | Some("wti") => Some(CommodityConvention::WTICrude),
            Some("brentcrude") | Some("brent") => Some(CommodityConvention::BrentCrude),
            Some("naturalgas") | Some("ng") => Some(CommodityConvention::NaturalGas),
            Some("gold") => Some(CommodityConvention::Gold),
            Some("silver") => Some(CommodityConvention::Silver),
            Some("copper") => Some(CommodityConvention::Copper),
            Some("agricultural") | Some("ag") => Some(CommodityConvention::Agricultural),
            Some("power") => Some(CommodityConvention::Power),
            None => None,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Invalid convention: '{other}'. Must be one of: wti, brent, naturalgas, gold, silver, copper, agricultural, power"
                )))
            }
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, lag_days=None)", signature = (lag_days=None))]
    fn settlement_lag_days(
        mut slf: PyRefMut<'_, Self>,
        lag_days: Option<u32>,
    ) -> PyRefMut<'_, Self> {
        slf.settlement_lag_days = lag_days;
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id=None)", signature = (calendar_id=None))]
    fn settlement_calendar_id(
        mut slf: PyRefMut<'_, Self>,
        calendar_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.settlement_calendar_id = calendar_id;
        slf
    }

    #[pyo3(text_signature = "($self, bdc=None)", signature = (bdc=None))]
    fn settlement_bdc(
        mut slf: PyRefMut<'_, Self>,
        bdc: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.settlement_bdc = parse_bdc_option(bdc.as_deref())?;
        Ok(slf)
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
        if let Some(conv) = slf.convention {
            builder = builder.convention_opt(Some(conv));
        }
        if let Some(lag) = slf.settlement_lag_days {
            builder = builder.settlement_lag_days_opt(Some(lag));
        }
        if let Some(cal) = slf.settlement_calendar_id.clone() {
            builder = builder.settlement_calendar_id_opt(Some(cal));
        }
        if let Some(bdc) = slf.settlement_bdc {
            builder = builder.settlement_bdc_opt(Some(bdc));
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

    /// Commodity convention (e.g., "wti", "brent", "gold"). None if not set.
    #[getter]
    fn convention(&self) -> Option<&str> {
        self.inner.convention.map(convention_to_str)
    }

    /// Settlement lag in business days, if explicitly set.
    #[getter]
    fn settlement_lag_days(&self) -> Option<u32> {
        self.inner.settlement_lag_days
    }

    /// Settlement calendar ID, if explicitly set.
    #[getter]
    fn settlement_calendar_id(&self) -> Option<&str> {
        self.inner.settlement_calendar_id.as_deref()
    }

    /// Settlement business day convention, if explicitly set.
    #[getter]
    fn settlement_bdc(&self) -> Option<&str> {
        self.inner.settlement_bdc.map(bdc_to_str)
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    /// True if this forward has no contract price set (NPV ~ 0 at inception).
    fn is_at_market(&self) -> bool {
        self.inner.is_at_market()
    }

    /// Effective settlement lag in business days (resolves convention defaults).
    fn effective_settlement_lag(&self) -> u32 {
        self.inner.effective_settlement_lag()
    }

    /// Effective settlement calendar ID (resolves convention defaults).
    fn effective_settlement_calendar(&self) -> Option<&str> {
        self.inner.effective_settlement_calendar()
    }

    /// Effective settlement business day convention (resolves convention defaults).
    fn effective_settlement_bdc(&self) -> &str {
        bdc_to_str(self.inner.effective_settlement_bdc())
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

fn convention_to_str(c: CommodityConvention) -> &'static str {
    match c {
        CommodityConvention::WTICrude => "wti",
        CommodityConvention::BrentCrude => "brent",
        CommodityConvention::NaturalGas => "naturalgas",
        CommodityConvention::Gold => "gold",
        CommodityConvention::Silver => "silver",
        CommodityConvention::Copper => "copper",
        CommodityConvention::Agricultural => "agricultural",
        CommodityConvention::Power => "power",
        _ => "unknown",
    }
}

fn bdc_to_str(bdc: BusinessDayConvention) -> &'static str {
    match bdc {
        BusinessDayConvention::Following => "following",
        BusinessDayConvention::ModifiedFollowing => "modified_following",
        BusinessDayConvention::Preceding => "preceding",
        BusinessDayConvention::ModifiedPreceding => "modified_preceding",
        BusinessDayConvention::Unadjusted => "unadjusted",
        _ => "unknown",
    }
}

fn parse_bdc_option(bdc: Option<&str>) -> PyResult<Option<BusinessDayConvention>> {
    match bdc {
        Some("following") | Some("Following") => Ok(Some(BusinessDayConvention::Following)),
        Some("modified_following") | Some("ModifiedFollowing") => {
            Ok(Some(BusinessDayConvention::ModifiedFollowing))
        }
        Some("preceding") | Some("Preceding") => Ok(Some(BusinessDayConvention::Preceding)),
        Some("modified_preceding") | Some("ModifiedPreceding") => {
            Ok(Some(BusinessDayConvention::ModifiedPreceding))
        }
        Some("unadjusted") | Some("Unadjusted") | Some("none") | Some("None") => {
            Ok(Some(BusinessDayConvention::Unadjusted))
        }
        None => Ok(None),
        Some(other) => Err(PyValueError::new_err(format!(
            "Invalid bdc: '{other}'. Must be 'following', 'modified_following', 'preceding', 'modified_preceding', or 'unadjusted'"
        ))),
    }
}

/// Export module items for registration.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityForward>()?;
    parent.add_class::<PyCommodityForwardBuilder>()?;
    Ok(())
}
