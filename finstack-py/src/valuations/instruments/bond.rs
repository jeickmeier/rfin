use crate::core::currency::PyCurrency;
use crate::core::dates::calendar::PyBusinessDayConvention;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::{PyFrequency, PyStubKind};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::cashflow::specs::PyAmortizationSpec;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::AmortizationSpec as ValAmortizationSpec;
use finstack_valuations::cashflow::builder::CashFlowSchedule;
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

/// Fixed-income bond instrument (builder-only API).
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
    custom_cashflows: Option<CashFlowSchedule>,
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
            custom_cashflows: None,
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
        if self.custom_cashflows.is_some() {
            if self.discount_curve.is_none() {
                return Err(PyValueError::new_err(
                    "Discount curve must be provided via disc_id().",
                ));
            }
            return Ok(());
        }
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

    #[pyo3(text_signature = "($self, schedule)")]
    fn cashflows<'py>(
        mut slf: PyRefMut<'py, Self>,
        schedule: PyRef<'py, PyCashFlowSchedule>,
    ) -> PyRefMut<'py, Self> {
        slf.custom_cashflows = Some(schedule.inner_clone());
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

        if let Some(schedule) = slf.custom_cashflows.clone() {
            let discount = slf.discount_curve.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "BondBuilder internal error: missing discount curve after validation",
                )
            })?;

            let mut bond = Bond::from_cashflows(
                slf.instrument_id.clone(),
                schedule,
                discount,
                slf.quoted_clean_price,
            )
            .map_err(core_to_py)?;

            if let Some(credit) = slf.credit_curve.clone() {
                bond.credit_curve_id = Some(credit);
            }

            if let Some(call_put) = slf.call_put.clone() {
                if call_put.has_options() {
                    bond.call_put = Some(call_put);
                }
            }

            if let Some(fwd) = slf.forward_curve.clone() {
                let freq = bond.cashflow_spec.frequency();
                let dc = bond.cashflow_spec.day_count();
                bond.cashflow_spec = CashflowSpec::Floating(FloatingCouponSpec {
                    rate_spec: FloatingRateSpec {
                        index_id: fwd,
                        spread_bp: rust_decimal::Decimal::from_f64_retain(slf.float_margin_bp)
                            .unwrap_or_default(),
                        gearing: rust_decimal::Decimal::from_f64_retain(slf.float_gearing)
                            .unwrap_or(rust_decimal::Decimal::ONE),
                        gearing_includes_spread: true,
                        floor_bp: None,
                        all_in_floor_bp: None,
                        cap_bp: None,
                        index_cap_bp: None,
                        reset_freq: freq,
                        reset_lag_days: slf.float_reset_lag_days,
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

            return Ok(PyBond::new(bond));
        }

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
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (``Bond.builder(\"ID\")``).
    fn builder<'py>(cls: &Bound<'py, PyType>, instrument_id: &str) -> PyResult<Py<PyBondBuilder>> {
        let py = cls.py();
        let builder = PyBondBuilder::new_with_id(InstrumentId::new(instrument_id));
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
