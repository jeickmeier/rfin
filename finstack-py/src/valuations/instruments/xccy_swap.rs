//! Cross-Currency Swap instrument bindings.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! This module only handles type conversion and builder ergonomics - no business logic
//! or financial calculations belong here.

use crate::core::common::args::{BusinessDayConventionArg, CurrencyArg, DayCountArg, TenorArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::xccy_swap::{
    LegSide, NotionalExchange, XccySwap, XccySwapLeg,
};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Cross-currency floating-for-floating swap.
///
/// Swaps floating cashflows in two currencies with optional notional exchange.
/// Each leg has its own forward curve, discount curve, and spread.
///
/// Examples
/// --------
/// Create a USD/EUR cross-currency basis swap::
///
///     from finstack import Money, Date
///     from finstack.valuations.instruments import CrossCurrencySwap
///
///     swap = (
///         CrossCurrencySwap.builder("XCCY_USD_EUR_001")
///         .start_date(Date(2024, 1, 1))
///         .maturity_date(Date(2029, 1, 1))
///         .reporting_currency("USD")
///         # First leg: Pay USD floating
///         .leg1_currency("USD")
///         .leg1_notional(10_000_000, "USD")
///         .leg1_side("pay")
///         .leg1_forward_curve("USD-SOFR")
///         .leg1_discount_curve("USD-OIS")
///         .leg1_frequency("quarterly")
///         .leg1_spread(0.0010)  # 10bp
///         # Second leg: Receive EUR floating
///         .leg2_currency("EUR")
///         .leg2_notional(9_000_000, "EUR")
///         .leg2_side("receive")
///         .leg2_forward_curve("EUR-ESTR")
///         .leg2_discount_curve("EUR-OIS")
///         .leg2_frequency("quarterly")
///         .leg2_spread(0.0)
///         .notional_exchange("initial_and_final")
///         .build()
///     )
///
/// See Also
/// --------
/// InterestRateSwap : Single-currency interest rate swap
/// BasisSwap : Single-currency basis swap
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CrossCurrencySwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCrossCurrencySwap {
    pub(crate) inner: Arc<XccySwap>,
}

impl PyCrossCurrencySwap {
    pub(crate) fn new(inner: XccySwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CrossCurrencySwapBuilder",
    unsendable
)]
pub struct PyCrossCurrencySwapBuilder {
    instrument_id: InstrumentId,
    start_date: Option<time::Date>,
    maturity_date: Option<time::Date>,
    reporting_currency: Option<Currency>,
    notional_exchange: NotionalExchange,
    stub_kind: StubKind,
    // Leg 1
    leg1_currency: Option<Currency>,
    leg1_notional_amount: Option<f64>,
    leg1_side: LegSide,
    leg1_forward_curve: Option<CurveId>,
    leg1_discount_curve: Option<CurveId>,
    leg1_frequency: Tenor,
    leg1_day_count: DayCount,
    leg1_bdc: BusinessDayConvention,
    leg1_spread: f64,
    leg1_payment_lag_days: i32,
    leg1_calendar_id: Option<String>,
    // Leg 2
    leg2_currency: Option<Currency>,
    leg2_notional_amount: Option<f64>,
    leg2_side: LegSide,
    leg2_forward_curve: Option<CurveId>,
    leg2_discount_curve: Option<CurveId>,
    leg2_frequency: Tenor,
    leg2_day_count: DayCount,
    leg2_bdc: BusinessDayConvention,
    leg2_spread: f64,
    leg2_payment_lag_days: i32,
    leg2_calendar_id: Option<String>,
}

impl PyCrossCurrencySwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            start_date: None,
            maturity_date: None,
            reporting_currency: None,
            notional_exchange: NotionalExchange::InitialAndFinal,
            stub_kind: StubKind::None,
            leg1_currency: None,
            leg1_notional_amount: None,
            leg1_side: LegSide::Pay,
            leg1_forward_curve: None,
            leg1_discount_curve: None,
            leg1_frequency: Tenor::quarterly(),
            leg1_day_count: DayCount::Act360,
            leg1_bdc: BusinessDayConvention::Following,
            leg1_spread: 0.0,
            leg1_payment_lag_days: 0,
            leg1_calendar_id: None,
            leg2_currency: None,
            leg2_notional_amount: None,
            leg2_side: LegSide::Receive,
            leg2_forward_curve: None,
            leg2_discount_curve: None,
            leg2_frequency: Tenor::quarterly(),
            leg2_day_count: DayCount::Act360,
            leg2_bdc: BusinessDayConvention::Following,
            leg2_spread: 0.0,
            leg2_payment_lag_days: 0,
            leg2_calendar_id: None,
        }
    }

    fn parse_leg_side(value: &str) -> PyResult<LegSide> {
        match value.to_lowercase().as_str() {
            "pay" => Ok(LegSide::Pay),
            "receive" | "rec" => Ok(LegSide::Receive),
            other => Err(PyValueError::new_err(format!(
                "expects 'pay' or 'receive', got '{}'",
                other
            ))),
        }
    }

    fn parse_notional_exchange(value: &str) -> PyResult<NotionalExchange> {
        match value.to_lowercase().as_str() {
            "none" => Ok(NotionalExchange::None),
            "final" => Ok(NotionalExchange::Final),
            "initial_and_final" | "both" => Ok(NotionalExchange::InitialAndFinal),
            other => Err(PyValueError::new_err(format!(
                "notional_exchange() expects 'none', 'final', or 'initial_and_final', got '{}'",
                other
            ))),
        }
    }
}

#[pymethods]
impl PyCrossCurrencySwapBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    // Common fields

    #[pyo3(text_signature = "($self, date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, date)")]
    fn maturity_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity_date = Some(py_to_date(&date).context("maturity_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn reporting_currency(
        mut slf: PyRefMut<'_, Self>,
        currency: CurrencyArg,
    ) -> PyRefMut<'_, Self> {
        slf.reporting_currency = Some(currency.0);
        slf
    }

    /// Set notional exchange convention ("none", "final", or "initial_and_final").
    #[pyo3(text_signature = "($self, exchange)")]
    fn notional_exchange<'py>(
        mut slf: PyRefMut<'py, Self>,
        exchange: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional_exchange = Self::parse_notional_exchange(exchange)?;
        Ok(slf)
    }

    // Leg 1 methods

    #[pyo3(text_signature = "($self, currency)")]
    fn leg1_currency(mut slf: PyRefMut<'_, Self>, currency: CurrencyArg) -> PyRefMut<'_, Self> {
        slf.leg1_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, amount, currency)")]
    fn leg1_notional(
        mut slf: PyRefMut<'_, Self>,
        amount: f64,
        currency: CurrencyArg,
    ) -> PyRefMut<'_, Self> {
        // Let Rust validation handle notional checks
        slf.leg1_notional_amount = Some(amount);
        slf.leg1_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn leg1_side<'py>(mut slf: PyRefMut<'py, Self>, side: &str) -> PyResult<PyRefMut<'py, Self>> {
        slf.leg1_side = Self::parse_leg_side(side)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg1_forward_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg1_forward_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg1_discount_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg1_discount_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn leg1_frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.leg1_frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn leg1_day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.leg1_day_count = day_count.0;
        slf
    }

    #[pyo3(text_signature = "($self, bdc)")]
    fn leg1_bdc(mut slf: PyRefMut<'_, Self>, bdc: BusinessDayConventionArg) -> PyRefMut<'_, Self> {
        slf.leg1_bdc = bdc.0;
        slf
    }

    /// Set leg 1 spread (decimal, e.g., 0.0001 = 1bp).
    #[pyo3(text_signature = "($self, spread)")]
    fn leg1_spread(mut slf: PyRefMut<'_, Self>, spread: f64) -> PyRefMut<'_, Self> {
        slf.leg1_spread = spread;
        slf
    }

    #[pyo3(text_signature = "($self, days)")]
    fn leg1_payment_lag_days(mut slf: PyRefMut<'_, Self>, days: i32) -> PyRefMut<'_, Self> {
        slf.leg1_payment_lag_days = days;
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id)")]
    fn leg1_calendar_id(
        mut slf: PyRefMut<'_, Self>,
        calendar_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.leg1_calendar_id = calendar_id;
        slf
    }

    // Leg 2 methods

    #[pyo3(text_signature = "($self, currency)")]
    fn leg2_currency(mut slf: PyRefMut<'_, Self>, currency: CurrencyArg) -> PyRefMut<'_, Self> {
        slf.leg2_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, amount, currency)")]
    fn leg2_notional(
        mut slf: PyRefMut<'_, Self>,
        amount: f64,
        currency: CurrencyArg,
    ) -> PyRefMut<'_, Self> {
        // Let Rust validation handle notional checks
        slf.leg2_notional_amount = Some(amount);
        slf.leg2_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn leg2_side<'py>(mut slf: PyRefMut<'py, Self>, side: &str) -> PyResult<PyRefMut<'py, Self>> {
        slf.leg2_side = Self::parse_leg_side(side)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg2_forward_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg2_forward_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg2_discount_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg2_discount_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn leg2_frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.leg2_frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn leg2_day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.leg2_day_count = day_count.0;
        slf
    }

    #[pyo3(text_signature = "($self, bdc)")]
    fn leg2_bdc(mut slf: PyRefMut<'_, Self>, bdc: BusinessDayConventionArg) -> PyRefMut<'_, Self> {
        slf.leg2_bdc = bdc.0;
        slf
    }

    /// Set leg 2 spread (decimal, e.g., 0.0001 = 1bp).
    #[pyo3(text_signature = "($self, spread)")]
    fn leg2_spread(mut slf: PyRefMut<'_, Self>, spread: f64) -> PyRefMut<'_, Self> {
        slf.leg2_spread = spread;
        slf
    }

    #[pyo3(text_signature = "($self, days)")]
    fn leg2_payment_lag_days(mut slf: PyRefMut<'_, Self>, days: i32) -> PyRefMut<'_, Self> {
        slf.leg2_payment_lag_days = days;
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id)")]
    fn leg2_calendar_id(
        mut slf: PyRefMut<'_, Self>,
        calendar_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.leg2_calendar_id = calendar_id;
        slf
    }

    /// Build the CrossCurrencySwap instrument.
    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCrossCurrencySwap> {
        let start_date = slf
            .start_date
            .ok_or_else(|| PyValueError::new_err("start_date() must be provided"))?;

        let maturity_date = slf
            .maturity_date
            .ok_or_else(|| PyValueError::new_err("maturity_date() must be provided"))?;

        let reporting_currency = slf
            .reporting_currency
            .ok_or_else(|| PyValueError::new_err("reporting_currency() must be provided"))?;

        // Build leg 1
        let leg1_currency = slf
            .leg1_currency
            .ok_or_else(|| PyValueError::new_err("leg1_currency() must be provided"))?;
        let leg1_notional_amount = slf
            .leg1_notional_amount
            .ok_or_else(|| PyValueError::new_err("leg1_notional() must be provided"))?;
        let leg1_forward_curve = slf
            .leg1_forward_curve
            .clone()
            .ok_or_else(|| PyValueError::new_err("leg1_forward_curve() must be provided"))?;
        let leg1_discount_curve = slf
            .leg1_discount_curve
            .clone()
            .ok_or_else(|| PyValueError::new_err("leg1_discount_curve() must be provided"))?;

        let leg1 = XccySwapLeg {
            currency: leg1_currency,
            notional: Money::new(leg1_notional_amount, leg1_currency),
            side: slf.leg1_side,
            forward_curve_id: leg1_forward_curve,
            discount_curve_id: leg1_discount_curve,
            frequency: slf.leg1_frequency,
            day_count: slf.leg1_day_count,
            bdc: slf.leg1_bdc,
            spread: slf.leg1_spread,
            payment_lag_days: slf.leg1_payment_lag_days,
            calendar_id: slf.leg1_calendar_id.clone(),
            allow_calendar_fallback: false,
        };

        // Build leg 2
        let leg2_currency = slf
            .leg2_currency
            .ok_or_else(|| PyValueError::new_err("leg2_currency() must be provided"))?;
        let leg2_notional_amount = slf
            .leg2_notional_amount
            .ok_or_else(|| PyValueError::new_err("leg2_notional() must be provided"))?;
        let leg2_forward_curve = slf
            .leg2_forward_curve
            .clone()
            .ok_or_else(|| PyValueError::new_err("leg2_forward_curve() must be provided"))?;
        let leg2_discount_curve = slf
            .leg2_discount_curve
            .clone()
            .ok_or_else(|| PyValueError::new_err("leg2_discount_curve() must be provided"))?;

        let leg2 = XccySwapLeg {
            currency: leg2_currency,
            notional: Money::new(leg2_notional_amount, leg2_currency),
            side: slf.leg2_side,
            forward_curve_id: leg2_forward_curve,
            discount_curve_id: leg2_discount_curve,
            frequency: slf.leg2_frequency,
            day_count: slf.leg2_day_count,
            bdc: slf.leg2_bdc,
            spread: slf.leg2_spread,
            payment_lag_days: slf.leg2_payment_lag_days,
            calendar_id: slf.leg2_calendar_id.clone(),
            allow_calendar_fallback: false,
        };

        let swap = XccySwap {
            id: slf.instrument_id.clone(),
            start_date,
            maturity_date,
            leg1,
            leg2,
            notional_exchange: slf.notional_exchange,
            reporting_currency,
            stub_kind: slf.stub_kind,
            attributes: Attributes::new(),
        };

        Ok(PyCrossCurrencySwap::new(swap))
    }
}

#[pymethods]
impl PyCrossCurrencySwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyCrossCurrencySwapBuilder {
        PyCrossCurrencySwapBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    #[getter]
    fn maturity_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity_date)
    }

    #[getter]
    fn reporting_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.reporting_currency)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::XccySwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CrossCurrencySwap(id='{}', {}/{}, maturity='{}')",
            self.inner.id,
            self.inner.leg1.currency,
            self.inner.leg2.currency,
            self.inner.maturity_date
        ))
    }
}

impl fmt::Display for PyCrossCurrencySwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CrossCurrencySwap({})", self.inner.id)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCrossCurrencySwap>()?;
    module.add_class::<PyCrossCurrencySwapBuilder>()?;
    Ok(vec!["CrossCurrencySwap", "CrossCurrencySwapBuilder"])
}
