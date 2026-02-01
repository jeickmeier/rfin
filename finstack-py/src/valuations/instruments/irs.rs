//! Interest Rate Swap instrument bindings.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! This module only handles type conversion and builder ergonomics - no business logic
//! or financial calculations belong here.

use crate::core::common::args::{
    BusinessDayConventionArg, CurrencyArg, DayCountArg, StubKindArg, TenorArg,
};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::irs::{
    FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive,
};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Pay/receive direction for swap fixed-leg cashflows.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PayReceive",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPayReceive {
    pub(crate) inner: PayReceive,
}

impl PyPayReceive {
    pub(crate) const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            PayReceive::PayFixed => "pay_fixed",
            PayReceive::ReceiveFixed => "receive_fixed",
        }
    }
}

#[pymethods]
impl PyPayReceive {
    #[classattr]
    const PAY_FIXED: Self = Self::new(PayReceive::PayFixed);
    #[classattr]
    const RECEIVE_FIXED: Self = Self::new(PayReceive::ReceiveFixed);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse ``"pay_fixed"`` or ``"receive_fixed"``.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<PayReceive>()
            .map(Self::new)
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("PayReceive('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = if let Ok(value) = other.extract::<PyRef<Self>>() {
            Some(value.inner)
        } else {
            None
        };
        crate::core::common::pycmp::richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyPayReceive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Plain-vanilla interest rate swap with fixed-for-floating legs.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateSwap {
    pub(crate) inner: Arc<InterestRateSwap>,
}

impl PyInterestRateSwap {
    pub(crate) fn new(inner: InterestRateSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateSwapBuilder",
    unsendable
)]
pub struct PyInterestRateSwapBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<Currency>,
    side: Option<PayReceive>,
    fixed_rate: Option<f64>,
    float_spread_bp: f64,
    start: Option<time::Date>,
    end: Option<time::Date>,
    discount_curve: Option<CurveId>,
    forward_curve: Option<CurveId>,
    fixed_frequency: Tenor,
    float_frequency: Tenor,
    fixed_day_count: DayCount,
    float_day_count: DayCount,
    bdc: BusinessDayConvention,
    calendar_id: Option<String>,
    stub: StubKind,
    reset_lag_days: i32,
}

impl PyInterestRateSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            side: None,
            fixed_rate: None,
            float_spread_bp: 0.0,
            start: None,
            end: None,
            discount_curve: None,
            forward_curve: None,
            fixed_frequency: Tenor::semi_annual(),
            float_frequency: Tenor::quarterly(),
            fixed_day_count: DayCount::Thirty360,
            float_day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&mut self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.side.is_none() {
            self.side = Some(PayReceive::PayFixed);
        }
        if self.fixed_rate.is_none() {
            return Err(PyValueError::new_err(
                "Fixed rate must be provided via fixed_rate().",
            ));
        }
        let end = self.end.ok_or_else(|| {
            PyValueError::new_err("Maturity date must be provided via maturity().")
        })?;
        if self.start.is_none() {
            let fallback = end.checked_sub(time::Duration::days(365)).unwrap_or(end);
            self.start = Some(fallback);
        }
        if self.discount_curve.is_none() {
            return Err(PyValueError::new_err(
                "Discount curve must be provided via disc_id().",
            ));
        }
        if self.forward_curve.is_none() {
            return Err(PyValueError::new_err(
                "Forward curve must be provided via fwd_id().",
            ));
        }
        Ok(())
    }

    fn parse_side(value: &Bound<'_, PyAny>) -> PyResult<PayReceive> {
        if let Ok(py_side) = value.extract::<PyRef<PyPayReceive>>() {
            return Ok(py_side.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return name
                .parse::<PayReceive>()
                .map_err(|e| PyValueError::new_err(e));
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "side expects PayReceive or str label",
        ))
    }
}

#[pymethods]
impl PyInterestRateSwapBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyRefMut<'_, Self> {
        // Let Rust validation in InterestRateSwap::builder().build() handle validation
        slf.pending_notional_amount = Some(amount);
        slf
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency(mut slf: PyRefMut<'_, Self>, currency: CurrencyArg) -> PyRefMut<'_, Self> {
        slf.pending_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side<'py>(
        mut slf: PyRefMut<'py, Self>,
        side: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.side = Some(Self::parse_side(&side)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, rate)")]
    fn fixed_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyRefMut<'_, Self> {
        // Let Rust validation handle negative rate checks
        slf.fixed_rate = Some(rate);
        slf
    }

    #[pyo3(text_signature = "($self, spread_bp)")]
    fn float_spread_bp(mut slf: PyRefMut<'_, Self>, spread_bp: f64) -> PyRefMut<'_, Self> {
        slf.float_spread_bp = spread_bp;
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
        slf.end = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn fwd_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn fixed_frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.fixed_frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn float_frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.float_frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.fixed_frequency = frequency.0;
        slf.float_frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn fixed_day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.fixed_day_count = day_count.0;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn float_day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.float_day_count = day_count.0;
        slf
    }

    #[pyo3(text_signature = "($self, bdc)")]
    fn bdc(mut slf: PyRefMut<'_, Self>, bdc: BusinessDayConventionArg) -> PyRefMut<'_, Self> {
        slf.bdc = bdc.0;
        slf
    }

    #[pyo3(text_signature = "($self, stub)")]
    fn stub(mut slf: PyRefMut<'_, Self>, stub: StubKindArg) -> PyRefMut<'_, Self> {
        slf.stub = stub.0;
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id=None)", signature = (calendar_id=None))]
    fn calendar<'py>(
        mut slf: PyRefMut<'py, Self>,
        calendar_id: Option<&str>,
    ) -> PyRefMut<'py, Self> {
        slf.calendar_id = calendar_id.map(|c| c.to_string());
        slf
    }

    #[pyo3(text_signature = "($self, days)")]
    fn reset_lag_days(mut slf: PyRefMut<'_, Self>, days: i32) -> PyRefMut<'_, Self> {
        slf.reset_lag_days = days;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyInterestRateSwap> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing notional after validation",
            )
        })?;
        if notional.amount() <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        let side = slf.side.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing side after validation",
            )
        })?;
        let fixed_rate = slf.fixed_rate.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing fixed_rate after validation",
            )
        })?;
        let start = slf.start.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing start date after validation",
            )
        })?;
        let end = slf.end.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing end date after validation",
            )
        })?;
        let discount = slf.discount_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing discount curve after validation",
            )
        })?;
        let forward = slf.forward_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateSwapBuilder internal error: missing forward curve after validation",
            )
        })?;
        let calendar = slf.calendar_id.clone();

        let fixed_leg = FixedLegSpec {
            discount_curve_id: discount.clone(),
            rate: rust_decimal::Decimal::from_f64_retain(fixed_rate).unwrap_or_default(),
            freq: slf.fixed_frequency,
            dc: slf.fixed_day_count,
            bdc: slf.bdc,
            calendar_id: calendar.clone(),
            stub: slf.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
        };

        let float_leg = FloatLegSpec {
            discount_curve_id: discount,
            forward_curve_id: forward,
            spread_bp: rust_decimal::Decimal::from_f64_retain(slf.float_spread_bp)
                .unwrap_or_default(),
            freq: slf.float_frequency,
            dc: slf.float_day_count,
            bdc: slf.bdc,
            calendar_id: calendar.clone(),
            stub: slf.stub,
            reset_lag_days: slf.reset_lag_days,
            start,
            end,
            compounding: Default::default(),
            fixing_calendar_id: calendar,
            payment_delay_days: 0,
        };

        InterestRateSwap::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .side(side)
            .fixed(fixed_leg)
            .float(float_leg)
            .attributes(Default::default())
            .build()
            .map(PyInterestRateSwap::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "InterestRateSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyInterestRateSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyInterestRateSwapBuilder>> {
        let py = cls.py();
        let builder = PyInterestRateSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional amount shared by both legs.
    ///
    /// Returns:
    ///     Any: Notional amount shared by both legs.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Pay/receive direction of the fixed leg.
    ///
    /// Returns:
    ///     Any: Pay/receive direction of the fixed leg.
    #[getter]
    fn side(&self) -> PyPayReceive {
        PyPayReceive::new(self.inner.side)
    }

    /// Fixed leg coupon rate.
    ///
    /// Returns:
    ///     Any: Fixed leg coupon rate.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.fixed.rate.to_f64().unwrap_or(0.0)
    }

    /// Floating leg spread in basis points.
    ///
    /// Returns:
    ///     Any: Floating leg spread in basis points.
    #[getter]
    fn float_spread_bp(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.float.spread_bp.to_f64().unwrap_or(0.0)
    }

    /// Effective start date (from fixed leg spec).
    ///
    /// Returns:
    ///     Any: Effective start date (from fixed leg spec).
    #[getter]
    fn start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.fixed.start)
    }

    /// Effective end date (from fixed leg spec).
    ///
    /// Returns:
    ///     Any: Effective end date (from fixed leg spec).
    #[getter]
    fn end(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.fixed.end)
    }

    /// Discount curve identifier used by the fixed leg.
    ///
    /// Returns:
    ///     Any: Discount curve identifier used by the fixed leg.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.fixed.discount_curve_id.as_str().to_string()
    }

    /// Floating forward curve identifier.
    ///
    /// Returns:
    ///     Any: Floating forward curve identifier.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.float.forward_curve_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.IRS``).
    ///
    /// Returns:
    ///     Any: Instrument type enum (``InstrumentType.IRS``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::IRS)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InterestRateSwap(id='{}', notional={}, side='{}')",
            self.inner.id,
            self.inner.notional,
            PyPayReceive::new(self.inner.side).label()
        ))
    }
}

impl fmt::Display for PyInterestRateSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IRS({}, rate={:.4}, side={})",
            self.inner.id,
            self.inner.fixed.rate,
            PyPayReceive::new(self.inner.side).label()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPayReceive>()?;
    module.add_class::<PyInterestRateSwap>()?;
    module.add_class::<PyInterestRateSwapBuilder>()?;
    Ok(vec![
        "PayReceive",
        "InterestRateSwap",
        "InterestRateSwapBuilder",
    ])
}
