//! Bond instrument bindings.
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
use crate::core::market_data::PyMarketContext;
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
use finstack_valuations::cashflow::AccrualMethod;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::fixed_income::bond::BondSettlementConvention;
use finstack_valuations::instruments::fixed_income::bond::MakeWholeSpec;
use finstack_valuations::instruments::fixed_income::bond::{
    CallPut, CallPutSchedule, CashflowSpec,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::PricingOverrides;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

use finstack_valuations::cashflow::builder::specs::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
};

/// Bond settlement and ex-coupon conventions.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BondSettlementConvention",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBondSettlementConvention {
    pub(crate) inner: BondSettlementConvention,
}

#[pymethods]
impl PyBondSettlementConvention {
    #[new]
    #[pyo3(signature = (settlement_days, ex_coupon_days=0, ex_coupon_calendar_id=None))]
    fn new_py(
        settlement_days: u32,
        ex_coupon_days: u32,
        ex_coupon_calendar_id: Option<String>,
    ) -> Self {
        Self {
            inner: BondSettlementConvention {
                settlement_days,
                ex_coupon_days,
                ex_coupon_calendar_id,
            },
        }
    }

    #[getter]
    fn settlement_days(&self) -> u32 {
        self.inner.settlement_days
    }
    #[getter]
    fn ex_coupon_days(&self) -> u32 {
        self.inner.ex_coupon_days
    }
    #[getter]
    fn ex_coupon_calendar_id(&self) -> Option<String> {
        self.inner.ex_coupon_calendar_id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "BondSettlementConvention(settlement_days={}, ex_coupon_days={})",
            self.inner.settlement_days, self.inner.ex_coupon_days
        )
    }
}

/// Accrual method for bond interest calculation.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AccrualMethod",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAccrualMethod {
    pub(crate) inner: AccrualMethod,
}

#[pymethods]
impl PyAccrualMethod {
    #[classattr]
    const LINEAR: Self = Self {
        inner: AccrualMethod::Linear,
    };
    #[classattr]
    const COMPOUNDED: Self = Self {
        inner: AccrualMethod::Compounded,
    };

    fn __repr__(&self) -> String {
        match &self.inner {
            AccrualMethod::Linear => "AccrualMethod.LINEAR".to_string(),
            AccrualMethod::Compounded => "AccrualMethod.COMPOUNDED".to_string(),
            _ => "AccrualMethod.UNKNOWN".to_string(),
        }
    }
}

/// Make-whole call specification.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MakeWholeSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMakeWholeSpec {
    pub(crate) inner: MakeWholeSpec,
}

#[pymethods]
impl PyMakeWholeSpec {
    #[new]
    #[pyo3(text_signature = "(reference_curve_id, spread_bps)")]
    fn new_py(reference_curve_id: String, spread_bps: f64) -> Self {
        Self {
            inner: MakeWholeSpec {
                reference_curve_id: CurveId::new(&reference_curve_id),
                spread_bps,
            },
        }
    }

    #[getter]
    fn reference_curve_id(&self) -> String {
        self.inner.reference_curve_id.as_str().to_string()
    }
    #[getter]
    fn spread_bps(&self) -> f64 {
        self.inner.spread_bps
    }

    fn __repr__(&self) -> String {
        format!(
            "MakeWholeSpec(curve='{}', spread_bps={})",
            self.inner.reference_curve_id, self.inner.spread_bps
        )
    }
}

/// Call or put option on a bond.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CallPut",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCallPut {
    pub(crate) inner: CallPut,
}

#[pymethods]
impl PyCallPut {
    #[new]
    #[pyo3(signature = (date, price_pct_of_par, end_date=None, make_whole=None))]
    fn new_py(
        date: Bound<'_, PyAny>,
        price_pct_of_par: f64,
        end_date: Option<Bound<'_, PyAny>>,
        make_whole: Option<&PyMakeWholeSpec>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: CallPut {
                date: py_to_date(&date)?,
                price_pct_of_par,
                end_date: end_date.map(|d| py_to_date(&d)).transpose()?,
                make_whole: make_whole.map(|mw| mw.inner.clone()),
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn price_pct_of_par(&self) -> f64 {
        self.inner.price_pct_of_par
    }
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner.end_date.map(|d| date_to_py(py, d)).transpose()
    }

    fn __repr__(&self) -> String {
        format!(
            "CallPut(date={}, price_pct={})",
            self.inner.date, self.inner.price_pct_of_par
        )
    }
}

/// Schedule of call and put options for a bond.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CallPutSchedule",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCallPutSchedule {
    pub(crate) inner: CallPutSchedule,
}

#[pymethods]
impl PyCallPutSchedule {
    #[new]
    #[pyo3(signature = (calls=None, puts=None))]
    fn new_py(
        calls: Option<Vec<PyRef<'_, PyCallPut>>>,
        puts: Option<Vec<PyRef<'_, PyCallPut>>>,
    ) -> Self {
        Self {
            inner: CallPutSchedule {
                calls: calls
                    .unwrap_or_default()
                    .iter()
                    .map(|c| c.inner.clone())
                    .collect(),
                puts: puts
                    .unwrap_or_default()
                    .iter()
                    .map(|p| p.inner.clone())
                    .collect(),
            },
        }
    }

    fn has_options(&self) -> bool {
        self.inner.has_options()
    }

    #[getter]
    fn call_count(&self) -> usize {
        self.inner.calls.len()
    }
    #[getter]
    fn put_count(&self) -> usize {
        self.inner.puts.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "CallPutSchedule(calls={}, puts={})",
            self.inner.calls.len(),
            self.inner.puts.len()
        )
    }
}

/// Cashflow specification for a bond (fixed, floating, or amortizing).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CashflowSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashflowSpec {
    pub(crate) inner: CashflowSpec,
}

#[pymethods]
impl PyCashflowSpec {
    #[getter]
    fn spec_type(&self) -> &'static str {
        match &self.inner {
            CashflowSpec::Fixed(_) => "fixed",
            CashflowSpec::Floating(_) => "floating",
            CashflowSpec::Amortizing { .. } => "amortizing",
        }
    }

    fn __repr__(&self) -> String {
        format!("CashflowSpec(type='{}')", self.spec_type())
    }
}

/// Fixed-income bond instrument (builder-only API).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Bond",
    frozen,
    from_py_object
)]
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
    unsendable,
    skip_from_py_object
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
    settlement_convention: Option<BondSettlementConvention>,
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
            settlement_convention: None,
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
        let calendar_id = self
            .calendar_id
            .clone()
            .unwrap_or_else(|| "weekends_only".to_string());
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
                    fixing_calendar_id: Some(calendar_id),
                    end_of_month: false,
                    overnight_compounding: None,
                    payment_lag_days: 0,
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
                calendar_id,
                stub: self.stub,
                end_of_month: false,
                payment_lag_days: 0,
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
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyRefMut<'_, Self> {
        // Let Rust validation in Bond::builder().build() handle validation
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
    fn coupon_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyRefMut<'_, Self> {
        // Let Rust validation handle negative rate checks
        slf.coupon_rate = rate;
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.day_count = day_count.0;
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

    #[pyo3(
        text_signature = "($self, calendar_id=None)",
        signature = (calendar_id=None)
    )]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar_id = calendar_id;
        slf
    }

    #[pyo3(text_signature = "($self, amortization=None)")]
    fn amortization(
        mut slf: PyRefMut<'_, Self>,
        amortization: Option<PyAmortizationSpec>,
    ) -> PyRefMut<'_, Self> {
        slf.amortization = amortization.map(|spec| spec.inner);
        slf
    }

    #[pyo3(signature = (convention))]
    fn settlement_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        convention: &PyBondSettlementConvention,
    ) -> PyRefMut<'py, Self> {
        slf.settlement_convention = Some(convention.inner.clone());
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
                end_date: None,
                make_whole: None,
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
                end_date: None,
                make_whole: None,
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
                        calendar_id: "weekends_only".to_string(),
                        fixing_calendar_id: None,
                        end_of_month: false,
                        overnight_compounding: None,
                        payment_lag_days: 0,
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
            .issue_date(issue)
            .maturity(maturity)
            .discount_curve_id(discount)
            .cashflow_spec(slf.make_cashflow_spec())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .settlement_convention_opt(slf.settlement_convention.clone());

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
        date_to_py(py, self.inner.issue_date)
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

    /// Number of settlement days (T+n), if a settlement convention is set.
    #[getter]
    fn settlement_days(&self) -> Option<u32> {
        self.inner.settlement_days()
    }

    /// Number of ex-coupon days before a coupon date, if set.
    #[getter]
    fn ex_coupon_days(&self) -> Option<u32> {
        self.inner.ex_coupon_days()
    }

    /// Calendar identifier used for ex-coupon day counting, if set.
    #[getter]
    fn ex_coupon_calendar_id(&self) -> Option<String> {
        self.inner.ex_coupon_calendar_id().map(|s| s.to_string())
    }

    /// Accrual method ("linear" or "compounded").
    #[getter]
    fn accrual_method(&self) -> &'static str {
        match &self.inner.accrual_method {
            finstack_valuations::cashflow::AccrualMethod::Linear => "linear",
            finstack_valuations::cashflow::AccrualMethod::Compounded => "compounded",
            _ => "unknown",
        }
    }

    /// Whether the bond has any call or put options.
    #[getter]
    fn has_call_put(&self) -> bool {
        self.inner
            .call_put
            .as_ref()
            .map_or(false, |cp| cp.has_options())
    }

    /// Validate all bond parameters.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If validation fails (e.g. issue >= maturity, notional <= 0)
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Generate the full cashflow schedule for this bond.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data for forward rate fixing (required for floating-rate bonds)
    ///
    /// Returns
    /// -------
    /// CashFlowSchedule
    ///     Complete schedule of coupon and principal cashflows
    fn get_full_schedule(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
    ) -> PyResult<PyCashFlowSchedule> {
        let schedule = py
            .detach(|| self.inner.get_full_schedule(&market.inner))
            .map_err(core_to_py)?;
        Ok(PyCashFlowSchedule::new(schedule))
    }

    #[getter]
    fn cashflow_spec(&self) -> PyCashflowSpec {
        PyCashflowSpec {
            inner: self.inner.cashflow_spec.clone(),
        }
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
    module.add_class::<PyBondSettlementConvention>()?;
    module.add_class::<PyAccrualMethod>()?;
    module.add_class::<PyMakeWholeSpec>()?;
    module.add_class::<PyCallPut>()?;
    module.add_class::<PyCallPutSchedule>()?;
    module.add_class::<PyCashflowSpec>()?;
    module.add_class::<PyBond>()?;
    module.add_class::<PyBondBuilder>()?;
    Ok(vec![
        "BondSettlementConvention",
        "AccrualMethod",
        "MakeWholeSpec",
        "CallPut",
        "CallPutSchedule",
        "CashflowSpec",
        "Bond",
        "BondBuilder",
    ])
}
