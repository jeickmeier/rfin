//! Python bindings for CommoditySwap instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, Tenor, TenorUnit};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_swap::CommoditySwap;
use finstack_valuations::instruments::legs::PayReceive;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Commodity swap (fixed-for-floating commodity price exchange).
///
/// One party pays a fixed price per unit, the other pays a floating price
/// determined by an index or average of spot prices over the period.
///
/// Examples:
///     >>> swap = (
///     ...     CommoditySwap.builder("NG-SWAP-2025")
///     ...     .commodity_type("Energy")
///     ...     .ticker("NG")
///     ...     .unit("MMBTU")
///     ...     .currency("USD")
///     ...     .quantity(10000.0)
///     ...     .fixed_price(3.50)
///     ...     .floating_index_id("NG-SPOT-AVG")
///     ...     .pay_fixed(True)
///     ...     .start_date(Date(2025, 1, 1))
///     ...     .end_date(Date(2025, 12, 31))
///     ...     .payment_frequency("1M")
///     ...     .discount_curve_id("USD-OIS")
///     ...     .build()
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySwap",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommoditySwap {
    pub(crate) inner: Arc<CommoditySwap>,
}

impl PyCommoditySwap {
    pub(crate) fn new(inner: CommoditySwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySwapBuilder",
    unsendable
)]
pub struct PyCommoditySwapBuilder {
    instrument_id: InstrumentId,
    commodity_type: Option<String>,
    ticker: Option<String>,
    unit: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    quantity: Option<f64>,
    fixed_price: Option<f64>,
    floating_index_id: Option<CurveId>,
    pay_fixed: Option<bool>,
    start_date: Option<time::Date>,
    end_date: Option<time::Date>,
    payment_frequency: Option<Tenor>,
    discount_curve_id: Option<CurveId>,
    calendar_id: Option<String>,
    bdc: Option<BusinessDayConvention>,
    index_lag_days: Option<i32>,
}

impl PyCommoditySwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            commodity_type: None,
            ticker: None,
            unit: None,
            currency: None,
            quantity: None,
            fixed_price: None,
            floating_index_id: None,
            pay_fixed: None,
            start_date: None,
            end_date: None,
            payment_frequency: None,
            discount_curve_id: None,
            calendar_id: None,
            bdc: None,
            index_lag_days: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.commodity_type.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("commodity_type() is required."));
        }
        if self.ticker.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("ticker() is required."));
        }
        if self.unit.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("unit() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.quantity.is_none() {
            return Err(PyValueError::new_err("quantity() is required."));
        }
        if self.fixed_price.is_none() {
            return Err(PyValueError::new_err("fixed_price() is required."));
        }
        if self.floating_index_id.is_none() {
            return Err(PyValueError::new_err("floating_index_id() is required."));
        }
        if self.pay_fixed.is_none() {
            return Err(PyValueError::new_err("pay_fixed() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.end_date.is_none() {
            return Err(PyValueError::new_err("end_date() is required."));
        }
        if self.payment_frequency.is_none() {
            return Err(PyValueError::new_err("payment_frequency() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCommoditySwapBuilder {
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

    #[pyo3(text_signature = "($self, unit)")]
    fn unit(mut slf: PyRefMut<'_, Self>, unit: String) -> PyRefMut<'_, Self> {
        slf.unit = Some(unit);
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

    #[pyo3(text_signature = "($self, quantity)")]
    fn quantity(mut slf: PyRefMut<'_, Self>, quantity: f64) -> PyResult<PyRefMut<'_, Self>> {
        if quantity <= 0.0 {
            return Err(PyValueError::new_err("quantity must be positive"));
        }
        slf.quantity = Some(quantity);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixed_price)")]
    fn fixed_price(mut slf: PyRefMut<'_, Self>, fixed_price: f64) -> PyRefMut<'_, Self> {
        slf.fixed_price = Some(fixed_price);
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn floating_index_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.floating_index_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, pay_fixed)")]
    fn pay_fixed(mut slf: PyRefMut<'_, Self>, pay_fixed: bool) -> PyRefMut<'_, Self> {
        slf.pay_fixed = Some(pay_fixed);
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

    #[pyo3(text_signature = "($self, payment_frequency)")]
    fn payment_frequency(
        mut slf: PyRefMut<'_, Self>,
        payment_frequency: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.payment_frequency = Some(parse_tenor(&payment_frequency).map_err(|e| {
            PyValueError::new_err(format!(
                "Invalid payment_frequency '{}': {}",
                payment_frequency, e
            ))
        })?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id=None)", signature = (calendar_id=None))]
    fn calendar_id(mut slf: PyRefMut<'_, Self>, calendar_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar_id = calendar_id;
        slf
    }

    #[pyo3(text_signature = "($self, bdc=None)", signature = (bdc=None))]
    fn bdc(mut slf: PyRefMut<'_, Self>, bdc: Option<String>) -> PyResult<PyRefMut<'_, Self>> {
        slf.bdc = match bdc.as_deref() {
            Some("following") | Some("Following") => Some(BusinessDayConvention::Following),
            Some("modified_following") | Some("ModifiedFollowing") => {
                Some(BusinessDayConvention::ModifiedFollowing)
            }
            Some("preceding") | Some("Preceding") => Some(BusinessDayConvention::Preceding),
            Some("modified_preceding") | Some("ModifiedPreceding") => {
                Some(BusinessDayConvention::ModifiedPreceding)
            }
            Some("unadjusted") | Some("Unadjusted") | Some("none") | Some("None") => {
                Some(BusinessDayConvention::Unadjusted)
            }
            None => None,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Invalid bdc: '{other}'. Must be 'following', 'modified_following', 'preceding', 'modified_preceding', or 'unadjusted'",
                )))
            }
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, index_lag_days=None)", signature = (index_lag_days=None))]
    fn index_lag_days(
        mut slf: PyRefMut<'_, Self>,
        index_lag_days: Option<i32>,
    ) -> PyRefMut<'_, Self> {
        slf.index_lag_days = index_lag_days;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommoditySwap> {
        slf.ensure_ready()?;

        let commodity_type = slf.commodity_type.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing commodity_type after validation",
            )
        })?;
        let ticker = slf.ticker.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing ticker after validation",
            )
        })?;
        let unit = slf.unit.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing unit after validation",
            )
        })?;
        let currency = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing currency after validation",
            )
        })?;
        let quantity = slf.quantity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing quantity after validation",
            )
        })?;
        let fixed_price = slf.fixed_price.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing fixed_price after validation",
            )
        })?;
        let floating_index_id = slf.floating_index_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing floating_index_id after validation",
            )
        })?;
        let pay_fixed = slf.pay_fixed.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing pay_fixed after validation",
            )
        })?;
        let start_date = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing start_date after validation",
            )
        })?;
        let end_date = slf.end_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing end_date after validation",
            )
        })?;
        let payment_frequency = slf.payment_frequency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing payment_frequency after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommoditySwapBuilder internal error: missing discount_curve_id after validation",
            )
        })?;

        let mut builder = CommoditySwap::builder()
            .id(slf.instrument_id.clone())
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .quantity(quantity)
            .fixed_price(rust_decimal::Decimal::try_from(fixed_price).unwrap_or_default())
            .floating_index_id(floating_index_id)
            .side(if pay_fixed {
                PayReceive::PayFixed
            } else {
                PayReceive::ReceiveFixed
            })
            .start_date(start_date)
            .maturity(end_date)
            .frequency(payment_frequency)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new());

        if let Some(cal) = slf.calendar_id.clone() {
            builder = builder.calendar_id_opt(Some(cal.into()));
        }
        if let Some(b) = slf.bdc {
            builder = builder.bdc(b);
        }
        if let Some(lag) = slf.index_lag_days {
            builder = builder.index_lag_days_opt(Some(lag));
        }

        let swap = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e}")))?;

        Ok(PyCommoditySwap::new(swap))
    }

    fn __repr__(&self) -> String {
        "CommoditySwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCommoditySwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommoditySwapBuilder>> {
        let py = cls.py();
        let builder = PyCommoditySwapBuilder::new_with_id(InstrumentId::new(instrument_id));
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

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.underlying.unit
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.underlying.currency)
    }

    /// Quantity per period.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Fixed price per unit.
    #[getter]
    fn fixed_price(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.fixed_price).unwrap_or_default()
    }

    /// Whether paying fixed (receiving floating).
    #[getter]
    fn pay_fixed(&self) -> bool {
        self.inner.side.is_payer()
    }

    /// Start date.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Floating index ID.
    #[getter]
    fn floating_index_id(&self) -> &str {
        self.inner.floating_index_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommoditySwap(id='{}', ticker='{}', fixed_price={}, pay_fixed={})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.fixed_price,
            self.inner.side.is_payer()
        )
    }
}

impl fmt::Display for PyCommoditySwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommoditySwap({}, {}, {})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.fixed_price
        )
    }
}

/// Parse a tenor string like "1M", "3M", "1Y" into a Tenor.
fn parse_tenor(s: &str) -> Result<Tenor, String> {
    let s = s.trim().to_uppercase();
    if s.is_empty() {
        return Err("Empty tenor string".to_string());
    }

    // Find the split point between number and unit
    let unit_start = s.find(|c: char| c.is_alphabetic()).ok_or("No unit found")?;
    let count_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let count: u32 = count_str
        .parse()
        .map_err(|_| format!("Invalid count: {}", count_str))?;

    let unit = match unit_str {
        "D" => TenorUnit::Days,
        "W" => TenorUnit::Weeks,
        "M" => TenorUnit::Months,
        "Y" => TenorUnit::Years,
        _ => return Err(format!("Unknown unit: {}", unit_str)),
    };

    Ok(Tenor::new(count, unit))
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommoditySwap>()?;
    parent.add_class::<PyCommoditySwapBuilder>()?;
    Ok(())
}
