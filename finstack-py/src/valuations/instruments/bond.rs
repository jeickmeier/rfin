use crate::core::currency::PyCurrency;
use crate::core::dates::calendar::PyBusinessDayConvention;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::{PyFrequency, PyStubKind};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::cashflow::specs::PyAmortizationSpec;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::AmortizationSpec as ValAmortizationSpec;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::fixed_income::bond::{
    CallPut, CallPutSchedule, CashflowSpec,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::PricingOverrides;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

use finstack_valuations::cashflow::builder::specs::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
};

/// Fixed-income bond instrument with convenience constructors.
#[pyclass(module = "finstack.valuations.instruments", name = "Bond", frozen)]
#[derive(Clone, Debug)]
pub struct PyBond {
    pub(crate) inner: Arc<Bond>,
}

impl PyBond {
    pub(crate) fn new(inner: Bond) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BondBuilder",
    unsendable
)]
pub struct PyBondBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<Currency>,
    issue: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve: Option<CurveId>,
    credit_curve: Option<CurveId>,
    frequency: Tenor,
    day_count: DayCount,
    bdc: BusinessDayConvention,
    calendar_id: Option<String>,
    stub: StubKind,
    amortization: Option<ValAmortizationSpec>,
    call_put: Option<CallPutSchedule>,
    coupon_rate: f64,
    quoted_clean_price: Option<f64>,
    forward_curve: Option<CurveId>,
    float_margin_bp: f64,
    float_gearing: f64,
    float_reset_lag_days: i32,
}

impl PyBondBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            issue: None,
            maturity: None,
            discount_curve: None,
            credit_curve: None,
            frequency: Tenor::semi_annual(),
            day_count: DayCount::Thirty360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            amortization: None,
            call_put: None,
            coupon_rate: 0.0,
            quoted_clean_price: None,
            forward_curve: None,
            float_margin_bp: 0.0,
            float_gearing: 1.0,
            float_reset_lag_days: 2,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn make_cashflow_spec(&self) -> CashflowSpec {
        let calendar_id = self.calendar_id.clone();
        if let Some(fwd) = &self.forward_curve {
            CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: fwd.clone(),
                    spread_bp: rust_decimal::Decimal::from_f64_retain(self.float_margin_bp)
                        .unwrap_or_default(),
                    gearing: rust_decimal::Decimal::from_f64_retain(self.float_gearing)
                        .unwrap_or(rust_decimal::Decimal::ONE),
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: self.frequency,
                    reset_lag_days: self.float_reset_lag_days,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: calendar_id.clone(),
                    fixing_calendar_id: calendar_id,
                },
                coupon_type: CouponType::Cash,
                freq: self.frequency,
                stub: self.stub,
            })
        } else {
            let base = CashflowSpec::Fixed(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: rust_decimal::Decimal::from_f64_retain(self.coupon_rate).unwrap_or_default(),
                freq: self.frequency,
                dc: self.day_count,
                bdc: self.bdc,
                calendar_id: self.calendar_id.clone(),
                stub: self.stub,
            });
            if let Some(amort) = &self.amortization {
                CashflowSpec::amortizing(base, amort.clone())
            } else {
                base
            }
        }
    }

    fn ensure_ready(&mut self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        let maturity = self.maturity.ok_or_else(|| {
            PyValueError::new_err("Maturity date must be provided via maturity().")
        })?;
        if self.issue.is_none() {
            let fallback = maturity
                .checked_sub(time::Duration::days(365))
                .unwrap_or(maturity);
            self.issue = Some(fallback);
        }
        if self.discount_curve.is_none() {
            return Err(PyValueError::new_err(
                "Discount curve must be provided via disc_id().",
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
            Err(PyTypeError::new_err("currency() expects str or Currency"))
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
                "semiannual" | "semi_annual" | "2y" | "6m" | "semi" => Ok(Tenor::semi_annual()),
                "quarterly" | "qtr" | "3m" => Ok(Tenor::quarterly()),
                "monthly" | "1m" => Ok(Tenor::monthly()),
                "biweekly" | "2w" => Ok(Tenor::biweekly()),
                "weekly" | "1w" => Ok(Tenor::weekly()),
                "daily" | "1d" => Ok(Tenor::daily()),
                other => Tenor::from_payments_per_year(other.parse::<u32>().map_err(|_| {
                    PyValueError::new_err("frequency() expects Tenor, name, or payments per year")
                })?)
                .map_err(|msg| PyValueError::new_err(msg.to_string())),
            };
        }
        if let Ok(payments) = value.extract::<u32>() {
            return Tenor::from_payments_per_year(payments)
                .map_err(|msg| PyValueError::new_err(msg.to_string()));
        }
        Err(PyTypeError::new_err(
            "frequency() expects Tenor, str, or int payments_per_year",
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
        Err(PyTypeError::new_err("day_count() expects DayCount or str"))
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
            "bdc() expects BusinessDayConvention or str",
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
        Err(PyTypeError::new_err("stub() expects StubKind or str"))
    }

    fn set_calendar(&mut self, value: Option<String>) {
        self.calendar_id = value;
    }

    fn call_put_mut(&mut self) -> &mut CallPutSchedule {
        self.call_put.get_or_insert_with(CallPutSchedule::default)
    }
}

#[pymethods]
impl PyBondBuilder {
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
        let parsed = Self::parse_currency(currency)?;
        slf.pending_currency = Some(parsed);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, issue)")]
    fn issue<'py>(
        mut slf: PyRefMut<'py, Self>,
        issue: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let issue_date = py_to_date(&issue).context("issue")?;
        slf.issue = Some(issue_date);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        slf.maturity = Some(maturity_date);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(
        text_signature = "($self, curve_id=None)",
        signature = (curve_id=None)
    )]
    fn credit_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: Option<String>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.credit_curve = curve_id.map(|id| CurveId::new(&id));
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, rate)")]
    fn coupon_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyResult<PyRefMut<'_, Self>> {
        if rate < 0.0 {
            return Err(PyValueError::new_err("coupon_rate must be non-negative"));
        }
        slf.coupon_rate = rate;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.frequency = Self::parse_frequency(&frequency)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.day_count = Self::parse_day_count(&day_count)?;
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

    #[pyo3(
        text_signature = "($self, calendar_id=None)",
        signature = (calendar_id=None)
    )]
    fn calendar<'py>(
        mut slf: PyRefMut<'py, Self>,
        calendar_id: Option<String>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.set_calendar(calendar_id);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, amortization=None)")]
    fn amortization(
        mut slf: PyRefMut<'_, Self>,
        amortization: Option<PyAmortizationSpec>,
    ) -> PyRefMut<'_, Self> {
        slf.amortization = amortization.map(|spec| spec.inner);
        slf
    }

    #[pyo3(text_signature = "($self, schedule)")]
    fn call_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        schedule: Vec<(Bound<'py, PyAny>, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut calls = Vec::with_capacity(schedule.len());
        for (date, price) in schedule {
            calls.push(CallPut {
                date: py_to_date(&date)?,
                price_pct_of_par: price,
            });
        }
        slf.call_put_mut().calls = calls;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, schedule)")]
    fn put_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        schedule: Vec<(Bound<'py, PyAny>, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut puts = Vec::with_capacity(schedule.len());
        for (date, price) in schedule {
            puts.push(CallPut {
                date: py_to_date(&date)?,
                price_pct_of_par: price,
            });
        }
        slf.call_put_mut().puts = puts;
        Ok(slf)
    }

    #[pyo3(
        text_signature = "($self, price=None)",
        signature = (price=None)
    )]
    fn quoted_clean_price(mut slf: PyRefMut<'_, Self>, price: Option<f64>) -> PyRefMut<'_, Self> {
        slf.quoted_clean_price = price;
        slf
    }

    #[pyo3(
        text_signature = "($self, curve_id=None)",
        signature = (curve_id=None)
    )]
    fn forward_curve(
        mut slf: PyRefMut<'_, Self>,
        curve_id: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.forward_curve = curve_id.map(|id| CurveId::new(&id));
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, margin_bp)")]
    fn float_margin_bp(mut slf: PyRefMut<'_, Self>, margin_bp: f64) -> PyRefMut<'_, Self> {
        slf.float_margin_bp = margin_bp;
        slf
    }

    #[pyo3(text_signature = "($self, gearing)")]
    fn float_gearing(mut slf: PyRefMut<'_, Self>, gearing: f64) -> PyRefMut<'_, Self> {
        slf.float_gearing = gearing;
        slf
    }

    #[pyo3(text_signature = "($self, lag_days)")]
    fn float_reset_lag_days(mut slf: PyRefMut<'_, Self>, lag_days: i32) -> PyRefMut<'_, Self> {
        slf.float_reset_lag_days = lag_days;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyBond> {
        slf.ensure_ready()?;
        let money = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BondBuilder internal error: missing notional after validation",
            )
        })?;
        let issue = slf.issue.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BondBuilder internal error: missing issue date after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BondBuilder internal error: missing maturity date after validation",
            )
        })?;
        let discount = slf.discount_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BondBuilder internal error: missing discount curve after validation",
            )
        })?;

        let mut builder = Bond::builder()
            .id(slf.instrument_id.clone())
            .notional(money)
            .issue(issue)
            .maturity(maturity)
            .discount_curve_id(discount)
            .cashflow_spec(slf.make_cashflow_spec())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .ex_coupon_calendar_id_opt(None);

        if let Some(credit) = slf.credit_curve.clone() {
            builder = builder.credit_curve_id_opt(Some(credit));
        }

        if let Some(px) = slf.quoted_clean_price {
            builder = builder.pricing_overrides(PricingOverrides::default().with_clean_price(px));
        }

        if let Some(schedule) = slf.call_put.clone() {
            if schedule.has_options() {
                builder = builder.call_put_opt(Some(schedule));
            }
        }

        builder.build().map(PyBond::new).map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "BondBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyBond {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, coupon_rate, issue, maturity, discount_curve)"
    )]
    /// Create a semi-annual fixed-rate bond with 30/360 day count and Following BDC.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     coupon_rate: Annual coupon in decimal form.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///     discount_curve: Discount curve identifier for valuation.
    ///
    /// Returns:
    ///     Bond: Configured fixed-rate bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///
    /// Examples:
    ///     >>> bond = Bond.fixed_semiannual(
    ///     ...     "corp_1",
    ///     ...     Money("USD", 1_000_000),
    ///     ...     0.045,
    ///     ...     date(2023, 1, 1),
    ///     ...     date(2028, 1, 1),
    ///     ...     "usd_discount"
    ///     ... )
    ///     >>> bond.coupon
    ///     0.045
    fn fixed_semiannual(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        Ok(Self::new(
            Bond::fixed(id, amt, coupon_rate, issue_date, maturity_date, disc)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
        ))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, coupon_rate, issue, maturity)")]
    /// Create a U.S. Treasury-style bond with annual coupons and Act/Act ISMA day count.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     coupon_rate: Annual coupon in decimal form.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///
    /// Returns:
    ///     Bond: Configured Treasury-style bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///
    /// Examples:
    ///     >>> Bond.treasury("ust_5y", Money("USD", 1_000), 0.03, date(2024, 1, 1), date(2029, 1, 1))
    ///     Bond(id='ust_5y', coupon=0.03, maturity='2029-01-01')
    fn treasury(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        use finstack_valuations::instruments::BondConvention;
        Ok(Self::new(
            Bond::with_convention(
                id,
                amt,
                coupon_rate,
                issue_date,
                maturity_date,
                BondConvention::USTreasury,
                "USD-TREASURY",
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
        ))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve)")]
    /// Create a zero-coupon bond discounted off ``discount_curve``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Redemption amount as :class:`finstack.core.money.Money`.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///     discount_curve: Discount curve identifier for valuation.
    ///
    /// Returns:
    ///     Bond: Configured zero-coupon bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn zero_coupon(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        Ok(Self::new(
            Bond::fixed(
                id,
                amt,
                0.0, // Zero coupon
                issue_date,
                maturity_date,
                disc,
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
        ))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional=None, issue=None, maturity=None, discount_curve=None, /, *, coupon_rate=None, frequency=None, day_count=None, bdc=None, calendar_id=None, stub=None, amortization=None, call_schedule=None, put_schedule=None, quoted_clean_price=None, forward_curve=None, float_margin_bp=None, float_gearing=None, float_reset_lag_days=None)",
        signature = (
            instrument_id,
            notional=None,
            issue=None,
            maturity=None,
            discount_curve=None,
            *,
            coupon_rate=None,
            frequency=None,
            day_count=None,
            bdc=None,
            calendar_id=None,
            stub=None,
            amortization=None,
            call_schedule=None,
            put_schedule=None,
            quoted_clean_price=None,
            forward_curve=None,
            float_margin_bp=None,
            float_gearing=None,
            float_reset_lag_days=None,
        )
    )]
    #[allow(clippy::too_many_arguments)]
    /// Start a fluent builder (``Bond.builder(\"ID\")``) or build immediately using
    /// the legacy full-argument signature.
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: Bound<'py, PyAny>,
        notional: Option<Bound<'py, PyAny>>,
        issue: Option<Bound<'py, PyAny>>,
        maturity: Option<Bound<'py, PyAny>>,
        discount_curve: Option<Bound<'py, PyAny>>,
        coupon_rate: Option<f64>,
        frequency: Option<PyFrequency>,
        day_count: Option<PyDayCount>,
        bdc: Option<PyBusinessDayConvention>,
        calendar_id: Option<&str>,
        stub: Option<PyStubKind>,
        amortization: Option<PyAmortizationSpec>,
        call_schedule: Option<Vec<(Bound<'py, PyAny>, f64)>>,
        put_schedule: Option<Vec<(Bound<'py, PyAny>, f64)>>,
        quoted_clean_price: Option<f64>,
        forward_curve: Option<Bound<'py, PyAny>>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
    ) -> PyResult<Py<PyAny>> {
        use crate::errors::PyContext;

        let py = cls.py();
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);

        let wants_builder = notional.is_none()
            && issue.is_none()
            && maturity.is_none()
            && discount_curve.is_none()
            && coupon_rate.is_none()
            && frequency.is_none()
            && day_count.is_none()
            && bdc.is_none()
            && calendar_id.is_none()
            && stub.is_none()
            && amortization.is_none()
            && call_schedule.is_none()
            && put_schedule.is_none()
            && quoted_clean_price.is_none()
            && forward_curve.is_none()
            && float_margin_bp.is_none()
            && float_gearing.is_none()
            && float_reset_lag_days.is_none();

        if wants_builder {
            let builder = PyBondBuilder::new_with_id(id);
            let handle = Py::new(py, builder)?;
            return Ok(handle.into());
        }

        let notional = notional.ok_or_else(|| {
            PyValueError::new_err(
                "notional is required when calling Bond.builder with full arguments",
            )
        })?;
        let issue = issue.ok_or_else(|| {
            PyValueError::new_err(
                "issue date is required when calling Bond.builder with full arguments",
            )
        })?;
        let maturity = maturity.ok_or_else(|| {
            PyValueError::new_err(
                "maturity date is required when calling Bond.builder with full arguments",
            )
        })?;
        let discount_curve = discount_curve.ok_or_else(|| {
            PyValueError::new_err(
                "discount_curve is required when calling Bond.builder with full arguments",
            )
        })?;

        let amt = extract_money(&notional).context("notional")?;
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);

        let freq = frequency.map(|f| f.inner).unwrap_or(Tenor::semi_annual());
        let dc = day_count.map(|d| d.inner).unwrap_or(DayCount::Thirty360);
        let bdc_val = bdc
            .map(|b| b.inner)
            .unwrap_or(BusinessDayConvention::Following);
        let stub_val = stub.map(|s| s.inner).unwrap_or(StubKind::None);
        let calendar_str = calendar_id.map(|s| s.to_string());

        let cashflow_spec = if let Some(fwd) = forward_curve {
            let forward_curve_id = CurveId::new(fwd.extract::<&str>()?);
            CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: forward_curve_id,
                    spread_bp: rust_decimal::Decimal::from_f64_retain(
                        float_margin_bp.unwrap_or(0.0),
                    )
                    .unwrap_or_default(),
                    gearing: rust_decimal::Decimal::from_f64_retain(float_gearing.unwrap_or(1.0))
                        .unwrap_or(rust_decimal::Decimal::ONE),
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: freq,
                    reset_lag_days: float_reset_lag_days.unwrap_or(2),
                    dc,
                    bdc: bdc_val,
                    calendar_id: calendar_str.clone(),
                    fixing_calendar_id: calendar_str.clone(),
                },
                coupon_type: CouponType::Cash,
                freq,
                stub: stub_val,
            })
        } else {
            let rate = rust_decimal::Decimal::from_f64_retain(coupon_rate.unwrap_or(0.0))
                .unwrap_or_default();
            if let Some(am) = amortization.as_ref() {
                CashflowSpec::amortizing(
                    CashflowSpec::Fixed(FixedCouponSpec {
                        coupon_type: CouponType::Cash,
                        rate,
                        freq,
                        dc,
                        bdc: bdc_val,
                        calendar_id: calendar_str.clone(),
                        stub: stub_val,
                    }),
                    am.inner.clone(),
                )
            } else {
                CashflowSpec::Fixed(FixedCouponSpec {
                    coupon_type: CouponType::Cash,
                    rate,
                    freq,
                    dc,
                    bdc: bdc_val,
                    calendar_id: calendar_str.clone(),
                    stub: stub_val,
                })
            }
        };

        let mut builder = Bond::builder()
            .id(id)
            .notional(amt)
            .issue(issue_date)
            .maturity(maturity_date)
            .discount_curve_id(disc)
            .cashflow_spec(cashflow_spec);

        if let Some(px) = quoted_clean_price {
            builder = builder.pricing_overrides(PricingOverrides::default().with_clean_price(px));
        }

        if call_schedule.is_some() || put_schedule.is_some() {
            let mut cps = CallPutSchedule::default();
            if let Some(calls) = call_schedule {
                for (d, pct) in calls {
                    cps.calls.push(CallPut {
                        date: py_to_date(&d)?,
                        price_pct_of_par: pct,
                    });
                }
            }
            if let Some(puts) = put_schedule {
                for (d, pct) in puts {
                    cps.puts.push(CallPut {
                        date: py_to_date(&d)?,
                        price_pct_of_par: pct,
                    });
                }
            }
            builder = builder.call_put_opt(Some(cps));
        }

        let bond = builder.build().map_err(core_to_py)?;
        let handle = Py::new(py, PyBond::new(bond))?;
        Ok(handle.into())
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, schedule, discount_curve, quoted_clean=None, forward_curve=None, float_margin_bp=None, float_gearing=None, float_reset_lag_days=None)",
        signature = (
            instrument_id,
            schedule,
            discount_curve,
            quoted_clean=None,
            forward_curve=None,
            float_margin_bp=None,
            float_gearing=None,
            float_reset_lag_days=None,
        )
    )]
    /// Create a bond from a pre-built CashFlowSchedule (supports PIK, amort, custom coupons).
    /// Optionally attach a floating-rate spec (forward curve + margin) so ASW metrics
    /// can build a custom-swap on the same schedule.
    fn from_cashflows(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        schedule: PyRef<PyCashFlowSchedule>,
        discount_curve: Bound<'_, PyAny>,
        quoted_clean: Option<f64>,
        forward_curve: Option<Bound<'_, PyAny>>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let mut bond = finstack_valuations::instruments::fixed_income::bond::Bond::from_cashflows(
            id,
            schedule.inner_clone(),
            disc,
            quoted_clean,
        )
        .map_err(core_to_py)?;

        if let Some(fwd) = forward_curve {
            // Update cashflow_spec to floating if forward curve is provided
            use finstack_valuations::cashflow::builder::specs::{
                CouponType, FloatingCouponSpec, FloatingRateSpec,
            };

            let forward_curve_id = CurveId::new(fwd.extract::<&str>().context("forward_curve")?);
            let freq = bond.cashflow_spec.frequency();
            let dc = bond.cashflow_spec.day_count();

            bond.cashflow_spec = CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: forward_curve_id,
                    spread_bp: rust_decimal::Decimal::from_f64_retain(
                        float_margin_bp.unwrap_or(0.0),
                    )
                    .unwrap_or_default(),
                    gearing: rust_decimal::Decimal::from_f64_retain(float_gearing.unwrap_or(1.0))
                        .unwrap_or(rust_decimal::Decimal::ONE),
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: freq,
                    reset_lag_days: float_reset_lag_days.unwrap_or(2),
                    dc,
                    bdc: finstack_core::dates::BusinessDayConvention::Following,
                    calendar_id: None,
                    fixing_calendar_id: None,
                },
                coupon_type: CouponType::Cash,
                freq,
                stub: finstack_core::dates::StubKind::None,
            });
        }

        Ok(Self::new(bond))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve, forward_curve, margin_bp)"
    )]
    /// Create a floating-rate note (FRN) using SOFR-like conventions.
    fn floating(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        margin_bp: f64,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let fwd = CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);

        use finstack_core::dates::{DayCount, Tenor};

        let bond = finstack_valuations::instruments::fixed_income::bond::Bond::floating(
            id,
            amt,
            fwd,
            margin_bp,
            issue_date,
            maturity_date,
            Tenor::quarterly(),
            DayCount::Act360,
            disc,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self::new(bond))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Annual coupon rate in decimal form (for fixed bonds only).
    ///
    /// Returns:
    ///     float: Annual coupon rate, or 0.0 for non-fixed bonds.
    #[getter]
    fn coupon(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            CashflowSpec::Amortizing { base, .. } => match &**base {
                CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    /// Issue date for the bond.
    ///
    /// Returns:
    ///     datetime.date: Issue date converted to Python.
    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.issue)
    }

    /// Maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier for the discount curve.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Optional hazard curve identifier enabling credit-sensitive pricing.
    ///
    /// Returns:
    ///     str | None: Hazard curve identifier when provided.
    #[getter]
    fn hazard_curve(&self) -> Option<String> {
        self.inner
            .credit_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    /// Instrument type enum (``InstrumentType.BOND``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Bond)
    }

    fn __repr__(&self) -> PyResult<String> {
        use rust_decimal::prelude::ToPrimitive;
        let coupon_rate = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            _ => 0.0,
        };
        Ok(format!(
            "Bond(id='{}', coupon={:.4}, maturity='{}')",
            self.inner.id, coupon_rate, self.inner.maturity
        ))
    }
}

impl fmt::Display for PyBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use rust_decimal::prelude::ToPrimitive;
        let coupon_rate = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            _ => 0.0,
        };
        write!(f, "Bond({}, coupon={:.4})", self.inner.id, coupon_rate)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBond>()?;
    module.add_class::<PyBondBuilder>()?;
    Ok(vec!["Bond", "BondBuilder"])
}
