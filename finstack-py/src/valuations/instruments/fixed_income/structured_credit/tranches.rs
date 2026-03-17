//! Python bindings for structured credit tranche types.
//!
//! Wraps `Tranche`, `TrancheBuilder`, `TrancheStructure`, `CoverageTrigger`,
//! and `CreditEnhancement` from the core valuations library.

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::CreditRating;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CoverageTrigger as RustCoverageTrigger, CreditEnhancement as RustCreditEnhancement,
    Seniority as TrancheSeniority, Tranche as RustTranche, TrancheBuilder as RustTrancheBuilder,
    TrancheCoupon, TrancheStructure as RustTrancheStructure, TriggerConsequence,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ============================================================================
// HELPERS
// ============================================================================

fn parse_seniority(s: &str) -> PyResult<TrancheSeniority> {
    match s {
        "Senior" | "senior" | "SENIOR" => Ok(TrancheSeniority::Senior),
        "Mezzanine" | "mezzanine" | "MEZZANINE" | "Mezz" | "mezz" => {
            Ok(TrancheSeniority::Mezzanine)
        }
        "Subordinated" | "subordinated" | "SUBORDINATED" | "Sub" | "sub" => {
            Ok(TrancheSeniority::Subordinated)
        }
        "Equity" | "equity" | "EQUITY" => Ok(TrancheSeniority::Equity),
        other => Err(PyValueError::new_err(format!(
            "Unknown seniority: '{other}'. Expected Senior, Mezzanine, Subordinated, or Equity"
        ))),
    }
}

fn parse_credit_rating(s: &str) -> PyResult<CreditRating> {
    s.parse::<CreditRating>()
        .map_err(|e| PyValueError::new_err(format!("Invalid credit rating '{s}': {e}")))
}

fn parse_currency(s: &str) -> PyResult<Currency> {
    s.parse::<Currency>()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency '{s}': {e:?}")))
}

fn parse_trigger_consequence(s: &str) -> PyResult<TriggerConsequence> {
    match s {
        "DivertCashFlow" | "divert_cash_flow" => Ok(TriggerConsequence::DivertCashFlow),
        "TrapExcessSpread" | "trap_excess_spread" => Ok(TriggerConsequence::TrapExcessSpread),
        "AccelerateAmortization" | "accelerate_amortization" => {
            Ok(TriggerConsequence::AccelerateAmortization)
        }
        "StopReinvestment" | "stop_reinvestment" => Ok(TriggerConsequence::StopReinvestment),
        "ReduceManagerFee" | "reduce_manager_fee" => Ok(TriggerConsequence::ReduceManagerFee),
        other => Err(PyValueError::new_err(format!(
            "Unknown trigger consequence: '{other}'"
        ))),
    }
}

fn consequence_to_string(c: &TriggerConsequence) -> String {
    match c {
        TriggerConsequence::DivertCashFlow => "DivertCashFlow".to_string(),
        TriggerConsequence::TrapExcessSpread => "TrapExcessSpread".to_string(),
        TriggerConsequence::AccelerateAmortization => "AccelerateAmortization".to_string(),
        TriggerConsequence::StopReinvestment => "StopReinvestment".to_string(),
        TriggerConsequence::ReduceManagerFee => "ReduceManagerFee".to_string(),
        TriggerConsequence::Custom(s) => format!("Custom({s})"),
        _ => "Unknown".to_string(),
    }
}

fn parse_day_count_str(s: &str) -> PyResult<DayCount> {
    let n = s.to_ascii_lowercase().replace([' ', '-'], "_");
    match n.as_str() {
        "act360" | "act/360" | "act_360" | "actual/360" => Ok(DayCount::Act360),
        "act365f" | "act/365f" | "act_365f" | "actual/365f" => Ok(DayCount::Act365F),
        "act365l" | "act/365l" | "act_365l" | "actual/365l" => Ok(DayCount::Act365L),
        "30/360" | "30_360" | "thirty/360" | "30u/360" => Ok(DayCount::Thirty360),
        "30e/360" | "30e_360" | "30/360e" => Ok(DayCount::ThirtyE360),
        "actact" | "act/act" | "act_act" | "actual/actual" => Ok(DayCount::ActAct),
        other => Err(PyValueError::new_err(format!("Unknown day-count: {other}"))),
    }
}

fn parse_tenor_str(s: &str) -> PyResult<Tenor> {
    let normalized = s.to_ascii_lowercase();
    match normalized.as_str() {
        "annual" | "1y" | "yearly" => Ok(Tenor::annual()),
        "semiannual" | "semi_annual" | "6m" | "semi" => Ok(Tenor::semi_annual()),
        "quarterly" | "qtr" | "3m" => Ok(Tenor::quarterly()),
        "monthly" | "1m" => Ok(Tenor::monthly()),
        "biweekly" | "2w" => Ok(Tenor::biweekly()),
        "weekly" | "1w" => Ok(Tenor::weekly()),
        "daily" | "1d" => Ok(Tenor::daily()),
        other => Err(PyValueError::new_err(format!(
            "Unknown frequency/tenor: '{other}'"
        ))),
    }
}

/// Convert a `Bound<PyAny>` to a JSON string (accepts str or dict).
fn extract_json_str(value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = value.extract::<String>() {
        return Ok(s);
    }
    if let Ok(dict) = value.cast::<pyo3::types::PyDict>() {
        let py = dict.py();
        let json = PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize dict: {e}")))?;
        return Ok(json);
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected JSON string or dict",
    ))
}

// ============================================================================
// PyCoverageTrigger
// ============================================================================

/// Coverage test trigger specification.
///
/// Args:
///     trigger_level: Trigger threshold (e.g. 1.20 for 120% OC).
///     consequence: Consequence when triggered (e.g. "DivertCashFlow").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CoverageTrigger",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCoverageTrigger {
    pub(crate) inner: RustCoverageTrigger,
}

#[pymethods]
impl PyCoverageTrigger {
    #[new]
    #[pyo3(text_signature = "(trigger_level, consequence)")]
    fn new(trigger_level: f64, consequence: &str) -> PyResult<Self> {
        let cons = parse_trigger_consequence(consequence)?;
        Ok(Self {
            inner: RustCoverageTrigger::new(trigger_level, cons),
        })
    }

    /// Return a new trigger with the specified cure level.
    #[pyo3(text_signature = "($self, cure_level)")]
    fn with_cure_level(&self, cure_level: f64) -> Self {
        Self {
            inner: self.inner.clone().with_cure_level(cure_level),
        }
    }

    /// Check if currently breached at the given coverage level.
    #[pyo3(text_signature = "($self, current_level)")]
    fn is_breached(&self, current_level: f64) -> bool {
        self.inner.is_breached(current_level)
    }

    /// Check if a breach is cured at the given coverage level.
    #[pyo3(text_signature = "($self, current_level)")]
    fn is_cured(&self, current_level: f64) -> bool {
        self.inner.is_cured(current_level)
    }

    #[getter]
    fn trigger_level(&self) -> f64 {
        self.inner.trigger_level
    }

    #[getter]
    fn cure_level(&self) -> Option<f64> {
        self.inner.cure_level
    }

    #[getter]
    fn consequence(&self) -> String {
        consequence_to_string(&self.inner.consequence)
    }

    fn __repr__(&self) -> String {
        format!(
            "CoverageTrigger(level={}, consequence='{}')",
            self.inner.trigger_level,
            consequence_to_string(&self.inner.consequence)
        )
    }
}

// ============================================================================
// PyCreditEnhancement
// ============================================================================

/// Credit enhancement mechanisms for a tranche.
///
/// Args:
///     subordination_amount: Subordination amount.
///     oc_amount: Overcollateralization amount.
///     reserve_amount: Reserve account balance.
///     currency: Currency code (e.g. "USD").
///     excess_spread: Available excess spread (default 0.0).
///     cash_trap_active: Whether cash trap is active (default false).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CreditEnhancement",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditEnhancement {
    pub(crate) inner: RustCreditEnhancement,
}

#[pymethods]
impl PyCreditEnhancement {
    #[new]
    #[pyo3(
        signature = (subordination_amount, oc_amount, reserve_amount, currency, excess_spread=0.0, cash_trap_active=false),
        text_signature = "(subordination_amount, oc_amount, reserve_amount, currency, excess_spread=0.0, cash_trap_active=False)"
    )]
    fn new(
        subordination_amount: f64,
        oc_amount: f64,
        reserve_amount: f64,
        currency: &str,
        excess_spread: f64,
        cash_trap_active: bool,
    ) -> PyResult<Self> {
        let ccy = parse_currency(currency)?;
        Ok(Self {
            inner: RustCreditEnhancement {
                subordination: Money::new(subordination_amount, ccy),
                overcollateralization: Money::new(oc_amount, ccy),
                reserve_account: Money::new(reserve_amount, ccy),
                excess_spread,
                cash_trap_active,
            },
        })
    }

    #[getter]
    fn subordination(&self) -> PyMoney {
        PyMoney::new(self.inner.subordination)
    }

    #[getter]
    fn overcollateralization(&self) -> PyMoney {
        PyMoney::new(self.inner.overcollateralization)
    }

    #[getter]
    fn reserve_account(&self) -> PyMoney {
        PyMoney::new(self.inner.reserve_account)
    }

    #[getter]
    fn excess_spread(&self) -> f64 {
        self.inner.excess_spread
    }

    #[getter]
    fn cash_trap_active(&self) -> bool {
        self.inner.cash_trap_active
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditEnhancement(sub={}, oc={}, reserve={}, spread={:.4}, trap={})",
            self.inner.subordination,
            self.inner.overcollateralization,
            self.inner.reserve_account,
            self.inner.excess_spread,
            self.inner.cash_trap_active,
        )
    }
}

// ============================================================================
// PyTranche
// ============================================================================

/// Structured credit tranche with attachment/detachment points.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Tranche",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTranche {
    pub(crate) inner: RustTranche,
}

#[pymethods]
impl PyTranche {
    // -- Getters ---------------------------------------------------------------

    #[getter]
    fn tranche_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn attachment_point(&self) -> f64 {
        self.inner.attachment_point
    }

    #[getter]
    fn detachment_point(&self) -> f64 {
        self.inner.detachment_point
    }

    #[getter]
    fn seniority(&self) -> String {
        format!("{}", self.inner.seniority)
    }

    #[getter]
    fn rating(&self) -> Option<String> {
        self.inner.rating.map(|r| format!("{r}"))
    }

    #[getter]
    fn original_balance(&self) -> PyMoney {
        PyMoney::new(self.inner.original_balance)
    }

    #[getter]
    fn current_balance(&self) -> PyMoney {
        PyMoney::new(self.inner.current_balance)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn payment_priority(&self) -> u32 {
        self.inner.payment_priority
    }

    #[getter]
    fn is_revolving(&self) -> bool {
        self.inner.is_revolving
    }

    #[getter]
    fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    #[getter]
    fn coupon_rate(&self) -> f64 {
        self.inner.coupon.current_rate(self.inner.maturity)
    }

    #[getter]
    fn target_balance(&self) -> Option<PyMoney> {
        self.inner.target_balance.as_ref().map(|m| PyMoney::new(*m))
    }

    #[getter]
    fn oc_trigger(&self) -> Option<PyCoverageTrigger> {
        self.inner
            .oc_trigger
            .as_ref()
            .map(|t| PyCoverageTrigger { inner: t.clone() })
    }

    #[getter]
    fn ic_trigger(&self) -> Option<PyCoverageTrigger> {
        self.inner
            .ic_trigger
            .as_ref()
            .map(|t| PyCoverageTrigger { inner: t.clone() })
    }

    #[getter]
    fn credit_enhancement(&self) -> PyCreditEnhancement {
        PyCreditEnhancement {
            inner: self.inner.credit_enhancement.clone(),
        }
    }

    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.frequency)
    }

    #[getter]
    fn deferred_interest(&self) -> PyMoney {
        PyMoney::new(self.inner.deferred_interest)
    }

    #[getter]
    fn can_reinvest(&self) -> bool {
        self.inner.can_reinvest
    }

    #[getter]
    fn expected_maturity(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match &self.inner.expected_maturity {
            Some(d) => date_to_py(py, *d).map(Some),
            None => Ok(None),
        }
    }

    // -- Serde -----------------------------------------------------------------

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustTranche = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize Tranche: {e}")))?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    // -- Builder ---------------------------------------------------------------

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn builder(_cls: &Bound<'_, PyType>) -> PyTrancheBuilder {
        PyTrancheBuilder {
            inner: RustTrancheBuilder::new(),
        }
    }

    // -- Methods ---------------------------------------------------------------

    /// Tranche thickness (detachment - attachment).
    #[pyo3(text_signature = "($self)")]
    fn thickness(&self) -> f64 {
        self.inner.thickness()
    }

    /// Check if this tranche is the first-loss piece (attachment at 0%).
    #[pyo3(text_signature = "($self)")]
    fn is_first_loss(&self) -> bool {
        self.inner.is_first_loss()
    }

    /// Check if tranche is impaired given cumulative pool losses.
    #[pyo3(text_signature = "($self, cumulative_loss_pct)")]
    fn is_impaired(&self, cumulative_loss_pct: f64) -> bool {
        self.inner.is_impaired(cumulative_loss_pct)
    }

    /// Calculate loss allocated to this tranche.
    #[pyo3(text_signature = "($self, cumulative_loss_pct, total_pool_balance_amount, currency)")]
    fn loss_allocation(
        &self,
        cumulative_loss_pct: f64,
        total_pool_balance_amount: f64,
        currency: &str,
    ) -> PyResult<PyMoney> {
        let ccy = parse_currency(currency)?;
        let pool_balance = Money::new(total_pool_balance_amount, ccy);
        Ok(PyMoney::new(
            self.inner
                .loss_allocation(cumulative_loss_pct, pool_balance),
        ))
    }

    /// Current balance after applying cumulative losses.
    #[pyo3(text_signature = "($self, cumulative_loss_pct, total_pool_balance_amount, currency)")]
    fn current_balance_after_losses(
        &self,
        cumulative_loss_pct: f64,
        total_pool_balance_amount: f64,
        currency: &str,
    ) -> PyResult<PyMoney> {
        let ccy = parse_currency(currency)?;
        let pool_balance = Money::new(total_pool_balance_amount, ccy);
        Ok(PyMoney::new(self.inner.current_balance_after_losses(
            cumulative_loss_pct,
            pool_balance,
        )))
    }

    /// Return a new tranche with the specified credit rating.
    #[pyo3(text_signature = "($self, rating_str)")]
    fn with_rating(&self, rating_str: &str) -> PyResult<Self> {
        let rating = parse_credit_rating(rating_str)?;
        Ok(Self {
            inner: self.inner.clone().with_rating(rating),
        })
    }

    /// Return a new tranche marked as revolving.
    #[pyo3(text_signature = "($self)")]
    fn revolving(&self) -> Self {
        Self {
            inner: self.inner.clone().revolving(),
        }
    }

    /// Return a new tranche with the specified OC trigger.
    #[pyo3(text_signature = "($self, trigger)")]
    fn with_oc_trigger(&self, trigger: PyCoverageTrigger) -> Self {
        Self {
            inner: self.inner.clone().with_oc_trigger(trigger.inner),
        }
    }

    /// Return a new tranche with the specified IC trigger.
    #[pyo3(text_signature = "($self, trigger)")]
    fn with_ic_trigger(&self, trigger: PyCoverageTrigger) -> Self {
        Self {
            inner: self.inner.clone().with_ic_trigger(trigger.inner),
        }
    }

    /// Return a new tranche with the specified expected maturity date.
    #[pyo3(text_signature = "($self, date)")]
    fn with_expected_maturity(&self, date: &Bound<'_, PyAny>) -> PyResult<Self> {
        let d = py_to_date(date)?;
        Ok(Self {
            inner: self.inner.clone().with_expected_maturity(d),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Tranche(id='{}', {:.1}/{:.1}, seniority={}, balance={})",
            self.inner.id,
            self.inner.attachment_point,
            self.inner.detachment_point,
            self.inner.seniority,
            self.inner.current_balance,
        )
    }
}

// ============================================================================
// PyTrancheBuilder
// ============================================================================

/// Fluent builder for constructing `Tranche` instances.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyTrancheBuilder {
    inner: RustTrancheBuilder,
}

#[pymethods]
impl PyTrancheBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustTrancheBuilder::new(),
        }
    }

    /// Set the tranche identifier.
    #[pyo3(text_signature = "($self, name)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, name: &'py str) -> PyRefMut<'py, Self> {
        slf.inner = std::mem::take(&mut slf.inner).id(name);
        slf
    }

    /// Set attachment and detachment points.
    #[pyo3(text_signature = "($self, attachment, detachment)")]
    fn attachment_detachment(
        mut slf: PyRefMut<'_, Self>,
        attachment: f64,
        detachment: f64,
    ) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).attachment_detachment(attachment, detachment);
        slf
    }

    /// Set tranche seniority from string.
    #[pyo3(text_signature = "($self, seniority_str)")]
    fn seniority<'py>(
        mut slf: PyRefMut<'py, Self>,
        seniority_str: &'py str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let sen = parse_seniority(seniority_str)?;
        slf.inner = std::mem::take(&mut slf.inner).seniority(sen);
        Ok(slf)
    }

    /// Set original balance from amount and currency.
    #[pyo3(text_signature = "($self, amount, currency)")]
    fn balance<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: f64,
        currency: &'py str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let ccy = parse_currency(currency)?;
        slf.inner = std::mem::take(&mut slf.inner).balance(Money::new(amount, ccy));
        Ok(slf)
    }

    /// Set a fixed-rate coupon.
    #[pyo3(text_signature = "($self, rate)")]
    fn coupon_fixed(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).coupon(TrancheCoupon::Fixed { rate });
        slf
    }

    /// Set a floating-rate coupon from a JSON string or dict.
    #[pyo3(text_signature = "($self, spec_json)")]
    fn coupon_floating<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec_json: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let json_str = extract_json_str(spec_json)?;
        let floating_spec: finstack_valuations::cashflow::builder::FloatingRateSpec =
            serde_json::from_str(&json_str).map_err(|e| {
                PyValueError::new_err(format!("Invalid floating rate spec JSON: {e}"))
            })?;
        slf.inner = std::mem::take(&mut slf.inner).coupon(TrancheCoupon::Floating(floating_spec));
        Ok(slf)
    }

    /// Set the legal maturity date.
    #[pyo3(text_signature = "($self, date)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let d = py_to_date(&date)?;
        slf.inner = std::mem::take(&mut slf.inner).maturity(d);
        Ok(slf)
    }

    /// Set the credit rating from string.
    #[pyo3(text_signature = "($self, rating_str)")]
    fn rating<'py>(
        mut slf: PyRefMut<'py, Self>,
        rating_str: &'py str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let rating = parse_credit_rating(rating_str)?;
        slf.inner = std::mem::take(&mut slf.inner).rating(rating);
        Ok(slf)
    }

    /// Set the payment frequency from string.
    #[pyo3(text_signature = "($self, tenor_str)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        tenor_str: &'py str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let tenor = parse_tenor_str(tenor_str)?;
        slf.inner = std::mem::take(&mut slf.inner).frequency(tenor);
        Ok(slf)
    }

    /// Set the day count convention from string.
    #[pyo3(text_signature = "($self, dc_str)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        dc_str: &'py str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let dc = parse_day_count_str(dc_str)?;
        slf.inner = std::mem::take(&mut slf.inner).day_count(dc);
        Ok(slf)
    }

    /// Build the `Tranche`, validating all required fields.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyTranche> {
        let builder = std::mem::take(&mut slf.inner);
        let tranche = builder.build().map_err(core_to_py)?;
        Ok(PyTranche { inner: tranche })
    }

    fn __repr__(&self) -> String {
        "TrancheBuilder(...)".to_string()
    }
}

impl Default for PyTrancheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PyTrancheStructure
// ============================================================================

/// Collection of tranches forming the capital structure.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheStructure",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTrancheStructure {
    pub(crate) inner: RustTrancheStructure,
}

#[pymethods]
impl PyTrancheStructure {
    #[new]
    #[pyo3(text_signature = "(tranches)")]
    fn new(tranches: Vec<PyTranche>) -> PyResult<Self> {
        let rust_tranches: Vec<RustTranche> = tranches.into_iter().map(|t| t.inner).collect();
        let structure = RustTrancheStructure::new(rust_tranches).map_err(core_to_py)?;
        Ok(Self { inner: structure })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustTrancheStructure = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize TrancheStructure: {e}"))
        })?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    /// Filter tranches by seniority string.
    #[pyo3(text_signature = "($self, seniority)")]
    fn by_seniority(&self, seniority: &str) -> PyResult<Vec<PyTranche>> {
        let sen = parse_seniority(seniority)?;
        Ok(self
            .inner
            .by_seniority(sen)
            .into_iter()
            .map(|t| PyTranche { inner: t.clone() })
            .collect())
    }

    /// Get tranches senior to the given tranche ID.
    #[pyo3(text_signature = "($self, tranche_id)")]
    fn senior_to(&self, tranche_id: &str) -> Vec<PyTranche> {
        self.inner
            .senior_to(tranche_id)
            .into_iter()
            .map(|t| PyTranche { inner: t.clone() })
            .collect()
    }

    /// Total balance of tranches senior to the given tranche.
    #[pyo3(text_signature = "($self, tranche_id)")]
    fn senior_balance(&self, tranche_id: &str) -> PyMoney {
        PyMoney::new(self.inner.senior_balance(tranche_id))
    }

    /// Subordination amount for the given tranche.
    #[pyo3(text_signature = "($self, tranche_id)")]
    fn subordination_amount(&self, tranche_id: &str) -> PyMoney {
        PyMoney::new(self.inner.subordination_amount(tranche_id))
    }

    #[getter]
    fn tranche_count(&self) -> usize {
        self.inner.tranches.len()
    }

    #[getter]
    fn total_size(&self) -> PyMoney {
        PyMoney::new(self.inner.total_size)
    }

    #[getter]
    fn tranches(&self) -> Vec<PyTranche> {
        self.inner
            .tranches
            .iter()
            .map(|t| PyTranche { inner: t.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "TrancheStructure(tranches={}, total_size={})",
            self.inner.tranches.len(),
            self.inner.total_size,
        )
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCoverageTrigger>()?;
    module.add_class::<PyCreditEnhancement>()?;
    module.add_class::<PyTranche>()?;
    module.add_class::<PyTrancheBuilder>()?;
    module.add_class::<PyTrancheStructure>()?;

    Ok(vec![
        "CoverageTrigger",
        "CreditEnhancement",
        "Tranche",
        "TrancheBuilder",
        "TrancheStructure",
    ])
}
