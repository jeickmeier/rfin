use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::calendar::PyBusinessDayConvention;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::{PyFrequency, PyStubKind};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::intern_calendar_id_opt;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::irs::{
    FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive,
};
use pyo3::basic::CompareOp;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    Bound, Py, PyRef, PyRefMut,
};
use std::fmt;

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
    pub(crate) inner: InterestRateSwap,
}

impl PyInterestRateSwap {
    pub(crate) fn new(inner: InterestRateSwap) -> Self {
        Self { inner }
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

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects Currency or str"))
        }
    }

    fn parse_frequency(value: &Bound<'_, PyAny>) -> PyResult<Tenor> {
        if let Ok(py_freq) = value.extract::<PyRef<PyFrequency>>() {
            return Ok((*py_freq).inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            let normalized = name.to_lowercase();
            return match normalized.as_str() {
                "annual" | "1y" | "yearly" => Ok(Tenor::annual()),
                "semiannual" | "semi_annual" | "semi" | "6m" => Ok(Tenor::semi_annual()),
                "quarterly" | "qtr" | "3m" => Ok(Tenor::quarterly()),
                "monthly" | "1m" => Ok(Tenor::monthly()),
                "biweekly" | "2w" => Ok(Tenor::biweekly()),
                "weekly" | "1w" => Ok(Tenor::weekly()),
                "daily" | "1d" => Ok(Tenor::daily()),
                other => Tenor::from_payments_per_year(other.parse::<u32>().map_err(|_| {
                    PyValueError::new_err("frequency expects Tenor, name, or payments per year")
                })?)
                .map_err(|msg| PyValueError::new_err(msg.to_string())),
            };
        }
        if let Ok(payments) = value.extract::<u32>() {
            return Tenor::from_payments_per_year(payments)
                .map_err(|msg| PyValueError::new_err(msg.to_string()));
        }
        Err(PyTypeError::new_err(
            "frequency expects Tenor, str, or int payments_per_year",
        ))
    }

    fn parse_day_count(value: &Bound<'_, PyAny>) -> PyResult<DayCount> {
        if let Ok(py_dc) = value.extract::<PyRef<PyDayCount>>() {
            return Ok(py_dc.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "act_360" | "act/360" => Ok(DayCount::Act360),
                "act_365f" | "act/365f" | "act365f" => Ok(DayCount::Act365F),
                "act_act" | "act/act" | "actact" => Ok(DayCount::ActAct),
                "thirty_360" | "30/360" | "30e/360" => Ok(DayCount::Thirty360),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported day count '{other}'"
                ))),
            };
        }
        Err(PyTypeError::new_err("day_count expects DayCount or str"))
    }

    fn parse_bdc(value: &Bound<'_, PyAny>) -> PyResult<BusinessDayConvention> {
        if let Ok(py_bdc) = value.extract::<PyRef<PyBusinessDayConvention>>() {
            return Ok(py_bdc.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "following" => Ok(BusinessDayConvention::Following),
                "modified_following" | "mod_following" | "modifiedfollowing" => {
                    Ok(BusinessDayConvention::ModifiedFollowing)
                }
                "preceding" => Ok(BusinessDayConvention::Preceding),
                "modified_preceding" | "mod_preceding" | "modifiedpreceding" => {
                    Ok(BusinessDayConvention::ModifiedPreceding)
                }
                "unadjusted" => Ok(BusinessDayConvention::Unadjusted),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported business day convention '{other}'"
                ))),
            };
        }
        Err(PyTypeError::new_err(
            "bdc expects BusinessDayConvention or str",
        ))
    }

    fn parse_stub(value: &Bound<'_, PyAny>) -> PyResult<StubKind> {
        if let Ok(py_stub) = value.extract::<PyRef<PyStubKind>>() {
            return Ok(py_stub.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "none" => Ok(StubKind::None),
                "short_front" => Ok(StubKind::ShortFront),
                "short_back" => Ok(StubKind::ShortBack),
                "long_front" => Ok(StubKind::LongFront),
                "long_back" => Ok(StubKind::LongBack),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported stub kind '{other}'"
                ))),
            };
        }
        Err(PyTypeError::new_err("stub expects StubKind or str"))
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
        Err(PyTypeError::new_err("side expects PayReceive or str label"))
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

    #[pyo3(text_signature = "($self, side)")]
    fn side<'py>(
        mut slf: PyRefMut<'py, Self>,
        side: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.side = Some(Self::parse_side(&side)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, rate)")]
    fn fixed_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyResult<PyRefMut<'_, Self>> {
        if rate < 0.0 {
            return Err(PyValueError::new_err("fixed_rate must be non-negative"));
        }
        slf.fixed_rate = Some(rate);
        Ok(slf)
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
    fn fixed_frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.fixed_frequency = Self::parse_frequency(&frequency)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn float_frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.float_frequency = Self::parse_frequency(&frequency)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let parsed = Self::parse_frequency(&frequency)?;
        slf.fixed_frequency = parsed;
        slf.float_frequency = parsed;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn fixed_day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.fixed_day_count = Self::parse_day_count(&day_count)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn float_day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.float_day_count = Self::parse_day_count(&day_count)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, bdc)")]
    fn bdc<'py>(
        mut slf: PyRefMut<'py, Self>,
        bdc: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.bdc = Self::parse_bdc(&bdc)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, stub)")]
    fn stub<'py>(
        mut slf: PyRefMut<'py, Self>,
        stub: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.stub = Self::parse_stub(&stub)?;
        Ok(slf)
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
        let notional = slf
            .notional_money()
            .expect("notional validated by ensure_ready");
        let side = slf.side.expect("validated by ensure_ready");
        let fixed_rate = slf.fixed_rate.expect("validated by ensure_ready");
        let start = slf.start.expect("validated by ensure_ready");
        let end = slf.end.expect("validated by ensure_ready");
        let discount = slf
            .discount_curve
            .clone()
            .expect("validated by ensure_ready");
        let forward = slf
            .forward_curve
            .clone()
            .expect("validated by ensure_ready");
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
            spread_bp: rust_decimal::Decimal::from_f64_retain(slf.float_spread_bp).unwrap_or_default(),
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
    #[pyo3(text_signature = "(cls, instrument_id, notional, fixed_rate, start, end)")]
    /// Create a USD SOFR swap where the caller pays fixed and receives floating.
    ///
    /// Note: Uses USD market conventions (fixed: semi-annual 30/360; float: quarterly Act/360).
    ///       For explicit curve and convention control, use ``builder()``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate in decimal form.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///
    /// Returns:
    ///     InterestRateSwap: Configured swap with USD-OIS discount and USD-SOFR-3M forward.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn usd_pay_fixed(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        use finstack_valuations::instruments::common::parameters::PayReceive;
        let swap = InterestRateSwap::create_usd_swap(
            id,
            amt,
            fixed_rate,
            start_date,
            end_date,
            PayReceive::PayFixed,
        )
        .map_err(core_to_py)?;
        Ok(Self::new(swap))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, fixed_rate, start, end)")]
    /// Create a USD SOFR swap where the caller receives fixed.
    ///
    /// Note: Uses USD market conventions (fixed: semi-annual 30/360; float: quarterly Act/360).
    ///       For explicit curve and convention control, use ``builder()``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate in decimal form.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///
    /// Returns:
    ///     InterestRateSwap: Configured swap with USD-OIS discount and USD-SOFR-3M forward.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn usd_receive_fixed(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        use finstack_valuations::instruments::common::parameters::PayReceive;
        let swap = InterestRateSwap::create_usd_swap(
            id,
            amt,
            fixed_rate,
            start_date,
            end_date,
            PayReceive::ReceiveFixed,
        )
        .map_err(core_to_py)?;
        Ok(Self::new(swap))
    }

    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional=None,
            fixed_rate=None,
            start=None,
            end=None,
            side=None,
            discount_curve=None,
            forward_curve=None,
            *,
            fixed_frequency=None,
            float_frequency=None,
            fixed_day_count=None,
            float_day_count=None,
            business_day_convention=None,
            float_spread_bp=None,
            reset_lag_days=None,
            calendar=None,
            stub=None
        ),
        text_signature = "(cls, instrument_id, notional=None, fixed_rate=None, start=None, end=None, side=None, discount_curve=None, forward_curve=None, /, *, fixed_frequency='semi_annual', float_frequency='quarterly', fixed_day_count='thirty_360', float_day_count='act_360', business_day_convention='modified_following', float_spread_bp=None, reset_lag_days=None, calendar=None, stub='none')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a fully customizable interest rate swap with explicit curves and conventions.
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: Bound<'py, PyAny>,
        notional: Option<Bound<'py, PyAny>>,
        fixed_rate: Option<f64>,
        start: Option<Bound<'py, PyAny>>,
        end: Option<Bound<'py, PyAny>>,
        side: Option<Bound<'py, PyAny>>,
        discount_curve: Option<Bound<'py, PyAny>>,
        forward_curve: Option<Bound<'py, PyAny>>,
        fixed_frequency: Option<PyFrequency>,
        float_frequency: Option<PyFrequency>,
        fixed_day_count: Option<Bound<'py, PyAny>>,
        float_day_count: Option<Bound<'py, PyAny>>,
        business_day_convention: Option<Bound<'py, PyAny>>,
        float_spread_bp: Option<f64>,
        reset_lag_days: Option<i32>,
        calendar: Option<&str>,
        stub: Option<PyStubKind>,
    ) -> PyResult<Py<PyAny>> {
        use crate::errors::PyContext;

        let py = cls.py();
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);

        let wants_builder = notional.is_none()
            && fixed_rate.is_none()
            && start.is_none()
            && end.is_none()
            && side.is_none()
            && discount_curve.is_none()
            && forward_curve.is_none()
            && fixed_frequency.is_none()
            && float_frequency.is_none()
            && fixed_day_count.is_none()
            && float_day_count.is_none()
            && business_day_convention.is_none()
            && float_spread_bp.is_none()
            && reset_lag_days.is_none()
            && calendar.is_none()
            && stub.is_none();

        if wants_builder {
            let builder = PyInterestRateSwapBuilder::new_with_id(id);
            let handle = Py::new(py, builder)?;
            return Ok(handle.into());
        }

        let notional = notional.ok_or_else(|| {
            PyValueError::new_err(
                "notional is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let fixed_rate = fixed_rate.ok_or_else(|| {
            PyValueError::new_err(
                "fixed_rate is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let start = start.ok_or_else(|| {
            PyValueError::new_err(
                "start date is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let end = end.ok_or_else(|| {
            PyValueError::new_err(
                "end date is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let side = side.ok_or_else(|| {
            PyValueError::new_err(
                "side is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let discount_curve = discount_curve.ok_or_else(|| {
            PyValueError::new_err(
                "discount_curve is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;
        let forward_curve = forward_curve.ok_or_else(|| {
            PyValueError::new_err(
                "forward_curve is required when calling InterestRateSwap.builder with full arguments",
            )
        })?;

        let amt = extract_money(&notional).context("notional")?;
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let forward_curve_id =
            CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);

        let side_value = PyInterestRateSwapBuilder::parse_side(&side)?;

        let fixed_freq = fixed_frequency
            .map(|f| f.inner)
            .unwrap_or_else(Tenor::semi_annual);
        let float_freq = float_frequency
            .map(|f| f.inner)
            .unwrap_or_else(Tenor::quarterly);

        let fixed_dc = if let Some(obj) = fixed_day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Thirty360
        };
        let float_dc = if let Some(obj) = float_day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Act360
        };

        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) = obj.extract()?;
            value
        } else {
            BusinessDayConvention::ModifiedFollowing
        };

        let stub_kind = stub.map(|s| s.inner).unwrap_or(StubKind::None);
        let cal_id_opt = intern_calendar_id_opt(calendar).map(|s| s.to_string());

        let fixed_leg = FixedLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            rate: rust_decimal::Decimal::from_f64_retain(fixed_rate).unwrap_or_default(),
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: cal_id_opt.clone(),
            stub: stub_kind,
            start: start_date,
            end: end_date,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
        };

        let float_leg = FloatLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            forward_curve_id,
            spread_bp: rust_decimal::Decimal::from_f64_retain(float_spread_bp.unwrap_or(0.0)).unwrap_or_default(),
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: cal_id_opt.clone(),
            fixing_calendar_id: cal_id_opt,
            stub: stub_kind,
            reset_lag_days: reset_lag_days.unwrap_or(2),
            start: start_date,
            end: end_date,
            compounding: Default::default(),
            payment_delay_days: 0,
        };

        let swap = InterestRateSwap::builder()
            .id(id)
            .notional(amt)
            .side(side_value)
            .fixed(fixed_leg)
            .float(float_leg)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        let handle = Py::new(py, Self::new(swap))?;
        Ok(handle.into())
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
