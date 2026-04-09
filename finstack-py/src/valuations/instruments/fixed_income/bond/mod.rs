//! Bond instrument bindings.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! This module only handles type conversion and builder ergonomics - no business logic
//! or financial calculations belong here.

pub(crate) mod dynamic_recovery;
pub(crate) mod endogenous_hazard;
pub(crate) mod mc_config;
pub(crate) mod merton;
pub(crate) mod toggle_exercise;
pub(crate) mod tree_config;

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
    BondBuilderParams, CallPut, CallPutSchedule, CashflowSpec, FloatingConventionParams,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::PricingOverrides;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

use finstack_valuations::cashflow::builder::specs::CouponType;

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

    /// Settlement lag in business days (e.g., 2 = T+2 standard settlement).
    #[getter]
    fn settlement_days(&self) -> u32 {
        self.inner.settlement_days
    }
    /// Number of business days before a coupon date when the bond trades ex-coupon.
    #[getter]
    fn ex_coupon_days(&self) -> u32 {
        self.inner.ex_coupon_days
    }
    /// Holiday calendar identifier used for ex-coupon day counting.
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

    /// Reference treasury curve identifier for make-whole calculation.
    #[getter]
    fn reference_curve_id(&self) -> String {
        self.inner.reference_curve_id.as_str().to_string()
    }
    /// Make-whole spread in basis points added to the reference curve
    /// (e.g., 50.0 = 50bps = 0.50%).
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

    /// First date the call/put option becomes exercisable.
    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    /// Exercise price as percentage of par (e.g., 102.0 = 102% of par).
    #[getter]
    fn price_pct_of_par(&self) -> f64 {
        self.inner.price_pct_of_par
    }
    /// Last date the call/put option is exercisable (if a range is specified).
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
            CashflowSpec::StepUp(_) => "step_up",
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
    coupon_type: CouponType,
    quoted_clean_price: Option<f64>,
    forward_curve: Option<CurveId>,
    float_margin_bp: f64,
    float_gearing: f64,
    float_reset_lag_days: i32,
    merton_mc_config: Option<finstack_valuations::instruments::fixed_income::bond::pricing::engine::merton_mc::MertonMcConfig>,
    tree_steps: Option<usize>,
    tree_volatility: Option<f64>,
    mean_reversion: Option<f64>,
    call_friction_cents: Option<f64>,
    tree_model: Option<finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::TreeModelChoice>,
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
            coupon_type: CouponType::Cash,
            quoted_clean_price: None,
            forward_curve: None,
            float_margin_bp: 0.0,
            float_gearing: 1.0,
            float_reset_lag_days: 2,
            merton_mc_config: None,
            tree_steps: None,
            tree_volatility: None,
            mean_reversion: None,
            call_friction_cents: None,
            tree_model: None,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn make_cashflow_spec(&self) -> PyResult<CashflowSpec> {
        use crate::valuations::common::f64_to_decimal;
        Ok(CashflowSpec::from_bond_builder_params(BondBuilderParams {
            coupon_rate: f64_to_decimal(self.coupon_rate, "coupon_rate")?,
            coupon_type: self.coupon_type,
            frequency: self.frequency,
            day_count: self.day_count,
            bdc: self.bdc,
            stub: self.stub,
            calendar_id: self.calendar_id.clone(),
            forward_curve: self.forward_curve.clone(),
            float_margin_bp: f64_to_decimal(self.float_margin_bp, "float_margin_bp")?,
            float_gearing: f64_to_decimal(self.float_gearing, "float_gearing")?,
            float_reset_lag_days: self.float_reset_lag_days,
            amortization: self.amortization.clone(),
        }))
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
        if self.maturity.is_none() {
            return Err(PyValueError::new_err(
                "Maturity date must be provided via maturity().",
            ));
        }
        // issue_date fallback (maturity − 365d) is handled by the core BondBuilder.
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
        slf.coupon_rate = rate;
        slf
    }

    /// Set the coupon type: ``"cash"`` (default) or ``"pik"``.
    #[pyo3(text_signature = "($self, coupon_type)")]
    fn coupon_type(
        mut slf: PyRefMut<'_, Self>,
        coupon_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        match coupon_type.to_lowercase().as_str() {
            "cash" => slf.coupon_type = CouponType::Cash,
            "pik" => slf.coupon_type = CouponType::PIK,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown coupon type '{other}': use 'cash' or 'pik'"
                )));
            }
        }
        Ok(slf)
    }

    /// Attach a Merton MC configuration for structural credit pricing
    /// via the pricer registry.
    #[pyo3(text_signature = "($self, config)")]
    fn merton_mc<'py>(
        mut slf: PyRefMut<'py, Self>,
        config: &Bound<'py, mc_config::PyMertonMcConfig>,
    ) -> PyRefMut<'py, Self> {
        slf.merton_mc_config = Some(config.borrow().inner.clone());
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

    /// Set number of time steps for tree-based pricing (default: 100).
    #[pyo3(text_signature = "($self, steps)")]
    fn tree_steps(mut slf: PyRefMut<'_, Self>, steps: usize) -> PyRefMut<'_, Self> {
        slf.tree_steps = Some(steps);
        slf
    }

    /// Set short rate volatility for tree-based pricing (annualized).
    ///
    /// Interpretation depends on model type:
    /// - Ho-Lee (default): Normal volatility in rate units (0.01 = 100 bps)
    /// - BDT: Lognormal volatility as proportion (0.20 = 20%)
    #[pyo3(text_signature = "($self, vol)")]
    fn tree_volatility(mut slf: PyRefMut<'_, Self>, vol: f64) -> PyRefMut<'_, Self> {
        slf.tree_volatility = Some(vol);
        slf
    }

    /// Set mean reversion speed for Hull-White extension (annualized).
    ///
    /// When set, transforms the Ho-Lee tree into Hull-White 1F.
    /// Typical values: 0.01-0.10 (1-10% per year).
    #[pyo3(
        text_signature = "($self, kappa=None)",
        signature = (kappa=None)
    )]
    fn mean_reversion(mut slf: PyRefMut<'_, Self>, kappa: Option<f64>) -> PyRefMut<'_, Self> {
        slf.mean_reversion = kappa;
        slf
    }

    /// Set issuer call exercise friction in cents per 100 of outstanding principal.
    ///
    /// Raises the exercise threshold without changing the redemption price.
    /// For example, 50.0 = $0.50 per $100 par.
    #[pyo3(
        text_signature = "($self, cents=None)",
        signature = (cents=None)
    )]
    fn call_friction_cents(mut slf: PyRefMut<'_, Self>, cents: Option<f64>) -> PyRefMut<'_, Self> {
        slf.call_friction_cents = cents;
        slf
    }

    /// Set the short-rate model choice for tree pricing.
    #[pyo3(
        text_signature = "($self, model=None)",
        signature = (model=None)
    )]
    fn tree_model<'py>(
        mut slf: PyRefMut<'py, Self>,
        model: Option<&tree_config::PyTreeModelChoice>,
    ) -> PyRefMut<'py, Self> {
        slf.tree_model = model.map(|m| m.inner.clone());
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
                use crate::valuations::common::f64_to_decimal;
                let freq = bond.cashflow_spec.frequency();
                let dc = bond.cashflow_spec.day_count();
                bond.cashflow_spec =
                    CashflowSpec::floating_with_conventions(FloatingConventionParams {
                        index_id: fwd,
                        spread_bp: f64_to_decimal(slf.float_margin_bp, "float_margin_bp")?,
                        gearing: f64_to_decimal(slf.float_gearing, "float_gearing")?,
                        reset_lag_days: slf.float_reset_lag_days,
                        coupon_type: CouponType::Cash,
                        freq,
                        dc,
                        bdc: BusinessDayConvention::Following,
                        calendar_id: "weekends_only".to_string(),
                        stub: StubKind::None,
                    });
            }

            return Ok(PyBond::new(bond));
        }

        let money = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BondBuilder internal error: missing notional after validation",
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

        let mut overrides = PricingOverrides::default();
        if let Some(mc_config) = slf.merton_mc_config.clone() {
            overrides = overrides.with_merton_mc(mc_config);
        }
        if let Some(px) = slf.quoted_clean_price {
            overrides = overrides.with_clean_price(px);
        }
        if let Some(steps) = slf.tree_steps {
            overrides = overrides.with_tree_steps(steps);
        }
        if let Some(vol) = slf.tree_volatility {
            overrides = overrides.with_tree_volatility(vol);
        }
        if let Some(mr) = slf.mean_reversion {
            overrides.model_config.mean_reversion = Some(mr);
        }
        if let Some(fc) = slf.call_friction_cents {
            overrides = overrides.with_call_friction_cents(fc);
        }

        let mut builder = Bond::builder()
            .id(slf.instrument_id.clone())
            .notional(money)
            .maturity(maturity)
            .discount_curve_id(discount)
            .cashflow_spec(slf.make_cashflow_spec()?)
            .pricing_overrides(overrides)
            .attributes(Attributes::new())
            .settlement_convention_opt(slf.settlement_convention.clone());

        if let Some(issue) = slf.issue {
            builder = builder.issue_date(issue);
        }

        if let Some(credit) = slf.credit_curve.clone() {
            builder = builder.credit_curve_id_opt(Some(credit));
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
    /// Start a fluent builder for a bond instrument.
    ///
    /// The builder applies market-standard defaults that can be overridden:
    ///
    /// - ``day_count``: 30/360 -- US corporate and government bond convention
    /// - ``frequency``: semi-annual (6M) -- US market standard
    /// - ``bdc``: Following -- adjusts coupon dates forward to the next business day
    /// - ``coupon_rate``: 0.0 -- defaults to zero-coupon; override for coupon bonds
    /// - ``float_reset_lag_days``: 2 (T+2) -- standard settlement lag for FRNs
    /// - ``float_gearing``: 1.0 -- no leverage on the floating index
    /// - ``stub``: None -- no stub periods
    ///
    /// Examples
    /// --------
    /// >>> from datetime import date
    /// >>> bond = (Bond.builder("us-corp-5y")
    /// ...     .notional(1_000_000)
    /// ...     .currency("USD")
    /// ...     .coupon_rate(0.05)
    /// ...     .frequency("6m")
    /// ...     .issue(date(2024, 1, 15))
    /// ...     .maturity(date(2029, 1, 15))
    /// ...     .disc_id("USD-OIS")
    /// ...     .build())
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

    /// Annual coupon rate as decimal (e.g., 0.05 = 5%) for fixed-rate bonds.
    ///
    /// Returns 0.0 for floating-rate or zero-coupon bonds.
    ///
    /// Returns
    /// -------
    /// float
    ///     Annual coupon rate as a decimal fraction.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the internal decimal value cannot be represented as float.
    #[getter]
    fn coupon(&self) -> PyResult<f64> {
        use rust_decimal::prelude::ToPrimitive;
        match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec
                .rate
                .to_f64()
                .ok_or_else(|| PyValueError::new_err("coupon: decimal to f64 conversion failed")),
            CashflowSpec::Amortizing { base, .. } => match &**base {
                CashflowSpec::Fixed(spec) => spec.rate.to_f64().ok_or_else(|| {
                    PyValueError::new_err("coupon: decimal to f64 conversion failed")
                }),
                _ => Ok(0.0),
            },
            _ => Ok(0.0),
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

    /// Number of settlement business days (e.g., 2 = T+2), if a settlement
    /// convention is set.
    ///
    /// Returns:
    ///     int | None: Settlement lag in business days, or ``None``.
    #[getter]
    fn settlement_days(&self) -> Option<u32> {
        self.inner.settlement_days()
    }

    /// Number of business days before a coupon date when the bond trades
    /// ex-coupon, if set.
    ///
    /// Returns:
    ///     int | None: Ex-coupon period in business days, or ``None``.
    #[getter]
    fn ex_coupon_days(&self) -> Option<u32> {
        self.inner.ex_coupon_days()
    }

    /// Holiday calendar identifier used for ex-coupon day counting, if set.
    ///
    /// Returns:
    ///     str | None: Calendar identifier, or ``None``.
    #[getter]
    fn ex_coupon_calendar_id(&self) -> Option<String> {
        self.inner.ex_coupon_calendar_id().map(|s| s.to_string())
    }

    /// Accrual method (``"linear"`` or ``"compounded"``).
    ///
    /// Returns:
    ///     str: Accrual method label.
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
            .is_some_and(|cp| cp.has_options())
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

    /// Generate the canonical cashflow schedule for this bond.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data for forward rate fixing (required for floating-rate bonds)
    /// as_of : datetime.date, optional
    ///     Valuation date. Defaults to the bond discount curve base date.
    ///
    /// Returns
    /// -------
    /// CashFlowSchedule
    ///     Holder-view contractual schedule returned by ``CashflowProvider.cashflow_schedule()``
    #[pyo3(
        signature = (market, *, as_of=None),
        text_signature = "($self, market, *, as_of=None)"
    )]
    fn cashflow_schedule(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyCashFlowSchedule> {
        let as_of_date = if let Some(as_of) = as_of {
            py_to_date(&as_of)?
        } else {
            market
                .inner
                .get_discount(self.inner.discount_curve_id.as_str())
                .map_err(core_to_py)?
                .base_date()
        };
        let schedule = py
            .detach(|| {
                use finstack_valuations::cashflow::CashflowProvider;
                self.inner.cashflow_schedule(&market.inner, as_of_date)
            })
            .map_err(core_to_py)?;
        Ok(PyCashFlowSchedule::new(schedule))
    }

    /// Cashflow schedule with discount factors, survival probabilities, and PVs.
    ///
    /// Generates the bond's full cashflow schedule and enriches each row with the
    /// discount factor and (when a credit/hazard curve is configured) survival
    /// probability used to compute its present value. The bond's own
    /// ``discount_curve`` and ``hazard_curve`` are wired automatically.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data containing discount and optional hazard curves.
    /// as_of : datetime.date, optional
    ///     Valuation date. Defaults to the discount curve's base date.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     One row per cashflow with columns: ``pay_date``, ``kind``, ``amount``,
    ///     ``discount_factor``, ``pv``, and optionally ``survival_prob``.
    #[pyo3(
        signature = (market, *, as_of=None),
        text_signature = "($self, market, *, as_of=None)"
    )]
    fn pricing_cashflows(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<pyo3_polars::PyDataFrame> {
        let as_of_date = as_of.map(|d| py_to_date(&d)).transpose()?;
        let frame = py
            .detach(|| self.inner.pricing_cashflows(&market.inner, as_of_date))
            .map_err(core_to_py)?;
        crate::valuations::cashflow::dataframe::period_dataframe_to_polars(frame)
    }

    /// Whether the bond has floating-rate coupons requiring forward curve projection.
    ///
    /// Returns:
    ///     bool: ``True`` for FRNs and amortizing bonds with a floating base.
    fn has_floating_coupons(&self) -> bool {
        self.inner.has_floating_coupons()
    }

    /// Accrual configuration derived from bond settlement conventions.
    ///
    /// Returns:
    ///     AccrualConfig: Configuration for accrued interest calculation.
    fn accrual_config(&self) -> crate::valuations::cashflow::accrual::PyAccrualConfig {
        crate::valuations::cashflow::accrual::PyAccrualConfig {
            inner: self.inner.accrual_config(),
        }
    }

    #[getter]
    fn cashflow_spec(&self) -> PyCashflowSpec {
        PyCashflowSpec {
            inner: self.inner.cashflow_spec.clone(),
        }
    }

    /// Price the bond using Monte Carlo simulation with structural credit model.
    ///
    /// Parameters
    /// ----------
    /// config : MertonMcConfig
    ///     Monte Carlo configuration with Merton model and optional credit specs.
    /// discount_rate : float
    ///     Risk-free discount rate.
    /// as_of : datetime.date
    ///     Valuation date.
    ///
    /// Returns
    /// -------
    /// MertonMcResult
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the bond's cashflow spec is not supported or simulation fails.
    #[pyo3(signature = (config, discount_rate, as_of))]
    fn price_merton_mc(
        &self,
        config: &mc_config::PyMertonMcConfig,
        discount_rate: f64,
        as_of: &Bound<'_, pyo3::types::PyAny>,
    ) -> PyResult<mc_config::PyMertonMcResult> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .price_merton_mc(&config.inner, discount_rate, date)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(mc_config::PyMertonMcResult { inner: result })
    }

    /// Calculate OAS using a tree pricer.
    ///
    /// Convenience method equivalent to ``TreePricer(config).calculate_oas(...)``.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount and optionally hazard curves.
    /// as_of : datetime.date
    ///     Valuation date.
    /// clean_price_pct : float
    ///     Market clean price as percentage of par (e.g., 98.5).
    /// config : TreePricerConfig | None, optional
    ///     Tree pricer configuration. If ``None``, uses bond pricing overrides
    ///     or default config.
    ///
    /// Returns
    /// -------
    /// float
    ///     OAS in basis points.
    #[pyo3(signature = (market, as_of, clean_price_pct, config=None))]
    fn calculate_oas(
        &self,
        market: &PyMarketContext,
        as_of: &Bound<'_, pyo3::types::PyAny>,
        clean_price_pct: f64,
        config: Option<&tree_config::PyTreePricerConfig>,
    ) -> PyResult<f64> {
        use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::{
            bond_tree_config, TreePricer as RustTreePricer,
        };
        let date = py_to_date(as_of).context("as_of")?;
        let pricer = match config {
            Some(c) => RustTreePricer::with_config(c.inner.clone()),
            None => RustTreePricer::with_config(bond_tree_config(&self.inner)),
        };
        pricer
            .calculate_oas(&self.inner, &market.inner, date, clean_price_pct)
            .map_err(core_to_py)
    }

    /// Price the bond at a given OAS using the short-rate tree.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data.
    /// as_of : datetime.date
    ///     Valuation date.
    /// oas_decimal : float
    ///     OAS in decimal form (e.g., 0.015 = 150 bp).
    ///
    /// Returns
    /// -------
    /// float
    ///     Dirty price in currency units.
    #[pyo3(signature = (market, as_of, oas_decimal))]
    fn price_from_oas(
        &self,
        market: &PyMarketContext,
        as_of: &Bound<'_, pyo3::types::PyAny>,
        oas_decimal: f64,
    ) -> PyResult<f64> {
        let date = py_to_date(as_of).context("as_of")?;
        finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::price_from_oas(
            &self.inner,
            &market.inner,
            date,
            oas_decimal,
        )
        .map_err(core_to_py)
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
    py: Python<'py>,
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

    let mut exports = vec![
        "BondSettlementConvention",
        "AccrualMethod",
        "MakeWholeSpec",
        "CallPut",
        "CallPutSchedule",
        "CashflowSpec",
        "Bond",
        "BondBuilder",
    ];

    let merton_exports = merton::register(py, module)?;
    exports.extend(merton_exports.iter().copied());

    let endogenous_exports = endogenous_hazard::register(py, module)?;
    exports.extend(endogenous_exports.iter().copied());

    let recovery_exports = dynamic_recovery::register(py, module)?;
    exports.extend(recovery_exports.iter().copied());

    let toggle_exports = toggle_exercise::register(py, module)?;
    exports.extend(toggle_exports.iter().copied());

    let mc_config_exports = mc_config::register(py, module)?;
    exports.extend(mc_config_exports.iter().copied());

    let tree_config_exports = tree_config::register(py, module)?;
    exports.extend(tree_config_exports.iter().copied());

    Ok(exports)
}
