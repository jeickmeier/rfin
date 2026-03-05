//! Python bindings for cashflow builder specification types.

use finstack_valuations::cashflow::builder::{
    AmortizationSpec, DefaultCurve, DefaultEvent, DefaultModelSpec, FeeTier, FloatingRateSpec,
    Notional, OvernightCompoundingMethod, PrepaymentCurve, PrepaymentModelSpec, RecoveryModelSpec,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use rust_decimal::prelude::ToPrimitive;

use crate::core::cashflow::primitives::PyCFKind;
use crate::core::currency::PyCurrency;
use crate::core::dates::calendar::PyBusinessDayConvention;
use crate::core::dates::utils::py_to_date;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::PyContext;

// ---------------------------------------------------------------------------
// AmortizationSpec
// ---------------------------------------------------------------------------

/// Amortization specification for principal over time.
#[pyclass(
    name = "AmortizationSpec",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAmortizationSpec {
    pub(crate) inner: AmortizationSpec,
}

#[pymethods]
impl PyAmortizationSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// No amortization: principal remains until redemption.
    fn none(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: AmortizationSpec::None,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, final_notional)")]
    /// Linear amortization towards a target final notional.
    fn linear_to(_cls: &Bound<'_, PyType>, final_notional: Bound<'_, PyAny>) -> PyResult<Self> {
        let m = extract_money(&final_notional)?;
        Ok(Self {
            inner: AmortizationSpec::LinearTo { final_notional: m },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, schedule)")]
    /// Step schedule of remaining principal after dates.
    /// ``schedule`` is a sequence of ``(date, Money)`` pairs ordered by date.
    fn step_remaining(
        _cls: &Bound<'_, PyType>,
        schedule: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let mut items = Vec::with_capacity(schedule.len());
        for (d, m) in schedule {
            items.push((py_to_date(&d)?, extract_money(&m)?));
        }
        Ok(Self {
            inner: AmortizationSpec::StepRemaining { schedule: items },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, pct)")]
    /// Fixed percentage of original notional paid each period (e.g., 0.05 = 5%).
    fn percent_per_period(_cls: &Bound<'_, PyType>, pct: f64) -> Self {
        Self {
            inner: AmortizationSpec::PercentOfOriginalPerPeriod { pct },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, items)")]
    /// Custom principal exchanges as absolute cash amounts.
    /// ``items`` is a sequence of ``(date, Money)`` pairs.
    fn custom_principal(
        _cls: &Bound<'_, PyType>,
        items: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let mut out = Vec::with_capacity(items.len());
        for (d, m) in items {
            out.push((py_to_date(&d)?, extract_money(&m)?));
        }
        Ok(Self {
            inner: AmortizationSpec::CustomPrincipal { items: out },
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            AmortizationSpec::None => "AmortizationSpec.none()".to_string(),
            AmortizationSpec::LinearTo { .. } => "AmortizationSpec.linear_to(...)".to_string(),
            AmortizationSpec::StepRemaining { .. } => {
                "AmortizationSpec.step_remaining(...)".to_string()
            }
            AmortizationSpec::PercentOfOriginalPerPeriod { pct } => {
                format!("AmortizationSpec.percent_per_period({pct})")
            }
            AmortizationSpec::CustomPrincipal { .. } => {
                "AmortizationSpec.custom_principal(...)".to_string()
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn extract_amortization_spec(value: &Bound<'_, PyAny>) -> PyResult<AmortizationSpec> {
    if let Ok(spec) = value.extract::<PyRef<PyAmortizationSpec>>() {
        return Ok(spec.inner.clone());
    }
    Err(PyTypeError::new_err("Expected AmortizationSpec"))
}

// ---------------------------------------------------------------------------
// Notional
// ---------------------------------------------------------------------------

/// Principal notional with optional amortization schedule.
#[pyclass(
    name = "Notional",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyNotional {
    pub(crate) inner: Notional,
}

#[pymethods]
impl PyNotional {
    #[classmethod]
    #[pyo3(text_signature = "(cls, amount, currency)")]
    /// Create a par notional (no amortization).
    fn par(_cls: &Bound<'_, PyType>, amount: f64, currency: &PyCurrency) -> Self {
        Self {
            inner: Notional::par(amount, currency.inner),
        }
    }

    #[getter]
    fn initial(&self) -> PyMoney {
        PyMoney::new(self.inner.initial)
    }

    #[getter]
    fn amort(&self) -> PyAmortizationSpec {
        PyAmortizationSpec {
            inner: self.inner.amort.clone(),
        }
    }

    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(crate::errors::core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Notional(initial={}, amort={:?})",
            self.inner.initial, self.inner.amort
        )
    }
}

// ---------------------------------------------------------------------------
// OvernightCompoundingMethod
// ---------------------------------------------------------------------------

/// Overnight rate compounding method for SOFR/SONIA-style indices.
#[pyclass(
    name = "OvernightCompoundingMethod",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyOvernightCompoundingMethod {
    pub(crate) inner: OvernightCompoundingMethod,
}

#[pymethods]
impl PyOvernightCompoundingMethod {
    #[classattr]
    const SIMPLE_AVERAGE: Self = Self {
        inner: OvernightCompoundingMethod::SimpleAverage,
    };

    #[classattr]
    const COMPOUNDED_IN_ARREARS: Self = Self {
        inner: OvernightCompoundingMethod::CompoundedInArrears,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, lookback_days)")]
    /// Compounded in arrears with lookback.
    fn compounded_with_lookback(_cls: &Bound<'_, PyType>, lookback_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithLookback { lookback_days },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, lockout_days)")]
    /// Compounded in arrears with lockout.
    fn compounded_with_lockout(_cls: &Bound<'_, PyType>, lockout_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithLockout { lockout_days },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, shift_days)")]
    /// Compounded with observation shift.
    fn compounded_with_observation_shift(_cls: &Bound<'_, PyType>, shift_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithObservationShift { shift_days },
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ---------------------------------------------------------------------------
// FloatingRateSpec
// ---------------------------------------------------------------------------

/// Full floating rate specification with caps, floors, and compounding.
#[pyclass(
    name = "FloatingRateSpec",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatingRateSpec {
    pub(crate) inner: FloatingRateSpec,
}

#[pymethods]
impl PyFloatingRateSpec {
    #[classmethod]
    #[pyo3(
        signature = (
            index_id, spread_bp, schedule,
            *, gearing=None, gearing_includes_spread=None,
            floor_bp=None, all_in_floor_bp=None, cap_bp=None, index_cap_bp=None,
            reset_lag_days=None, fixing_calendar_id=None,
            overnight_compounding=None
        ),
        text_signature = "(cls, index_id, spread_bp, schedule, *, gearing=1.0, gearing_includes_spread=True, floor_bp=None, all_in_floor_bp=None, cap_bp=None, index_cap_bp=None, reset_lag_days=2, fixing_calendar_id=None, overnight_compounding=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        _cls: &Bound<'_, PyType>,
        index_id: &str,
        spread_bp: f64,
        schedule: &super::builder::PyScheduleParams,
        gearing: Option<f64>,
        gearing_includes_spread: Option<bool>,
        floor_bp: Option<f64>,
        all_in_floor_bp: Option<f64>,
        cap_bp: Option<f64>,
        index_cap_bp: Option<f64>,
        reset_lag_days: Option<i32>,
        fixing_calendar_id: Option<String>,
        overnight_compounding: Option<PyOvernightCompoundingMethod>,
    ) -> Self {
        use rust_decimal::Decimal;
        let calendar_id = schedule.inner.calendar_id.clone();
        Self {
            inner: FloatingRateSpec {
                index_id: finstack_core::types::CurveId::new(index_id),
                spread_bp: Decimal::from_f64_retain(spread_bp).unwrap_or_default(),
                gearing: Decimal::from_f64_retain(gearing.unwrap_or(1.0)).unwrap_or(Decimal::ONE),
                gearing_includes_spread: gearing_includes_spread.unwrap_or(true),
                floor_bp: floor_bp.and_then(Decimal::from_f64_retain),
                all_in_floor_bp: all_in_floor_bp.and_then(Decimal::from_f64_retain),
                cap_bp: cap_bp.and_then(Decimal::from_f64_retain),
                index_cap_bp: index_cap_bp.and_then(Decimal::from_f64_retain),
                reset_freq: schedule.inner.freq,
                reset_lag_days: reset_lag_days.unwrap_or(2),
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: calendar_id.clone(),
                fixing_calendar_id: fixing_calendar_id.or(Some(calendar_id)),
                end_of_month: schedule.inner.end_of_month,
                overnight_compounding: overnight_compounding.map(|o| o.inner),
                fallback: Default::default(),
                payment_lag_days: schedule.inner.payment_lag_days,
            },
        }
    }

    #[getter]
    fn index_id(&self) -> &str {
        self.inner.index_id.as_str()
    }

    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.spread_bp.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn gearing(&self) -> f64 {
        self.inner.gearing.to_f64().unwrap_or(1.0)
    }

    #[getter]
    fn floor_bp(&self) -> Option<f64> {
        self.inner.floor_bp.and_then(|d| d.to_f64())
    }

    #[getter]
    fn cap_bp(&self) -> Option<f64> {
        self.inner.cap_bp.and_then(|d| d.to_f64())
    }

    fn __repr__(&self) -> String {
        format!(
            "FloatingRateSpec(index={}, spread_bp={:.2})",
            self.inner.index_id.as_str(),
            self.inner.spread_bp
        )
    }
}

// ---------------------------------------------------------------------------
// PrepaymentCurve + PrepaymentModelSpec
// ---------------------------------------------------------------------------

/// Prepayment curve shape (constant, PSA, CMBS lockout).
#[pyclass(
    name = "PrepaymentCurve",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrepaymentCurve {
    pub(crate) inner: PrepaymentCurve,
}

#[pymethods]
impl PyPrepaymentCurve {
    #[classattr]
    const CONSTANT: Self = Self {
        inner: PrepaymentCurve::Constant,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, speed_multiplier)")]
    /// PSA (Public Securities Association) prepayment curve.
    fn psa(_cls: &Bound<'_, PyType>, speed_multiplier: f64) -> Self {
        Self {
            inner: PrepaymentCurve::Psa { speed_multiplier },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, lockout_months)")]
    /// CMBS lockout curve: no prepayment during lockout, constant CPR after.
    fn cmbs_lockout(_cls: &Bound<'_, PyType>, lockout_months: u32) -> Self {
        Self {
            inner: PrepaymentCurve::CmbsLockout { lockout_months },
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Prepayment model specification with CPR and optional curve.
#[pyclass(
    name = "PrepaymentModelSpec",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrepaymentModelSpec {
    pub(crate) inner: PrepaymentModelSpec,
}

#[pymethods]
impl PyPrepaymentModelSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, cpr)")]
    /// Constant CPR prepayment model.
    fn constant_cpr(_cls: &Bound<'_, PyType>, cpr: f64) -> Self {
        Self {
            inner: PrepaymentModelSpec::constant_cpr(cpr),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, speed_multiplier)")]
    /// PSA prepayment model with given speed multiplier.
    fn psa(_cls: &Bound<'_, PyType>, speed_multiplier: f64) -> Self {
        Self {
            inner: PrepaymentModelSpec::psa(speed_multiplier),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// PSA 100% standard prepayment model.
    fn psa_100(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: PrepaymentModelSpec::psa_100(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, lockout_months, post_lockout_cpr)")]
    /// CMBS with lockout: no prepayment during lockout, constant CPR after.
    fn cmbs_with_lockout(
        _cls: &Bound<'_, PyType>,
        lockout_months: u32,
        post_lockout_cpr: f64,
    ) -> Self {
        Self {
            inner: PrepaymentModelSpec::cmbs_with_lockout(lockout_months, post_lockout_cpr),
        }
    }

    #[pyo3(text_signature = "(self, seasoning_months)")]
    /// Compute the Single Monthly Mortality (SMM) rate for a given month.
    fn smm(&self, seasoning_months: u32) -> f64 {
        self.inner.smm(seasoning_months)
    }

    #[getter]
    fn cpr(&self) -> f64 {
        self.inner.cpr
    }

    fn __repr__(&self) -> String {
        format!(
            "PrepaymentModelSpec(cpr={:.4}, curve={:?})",
            self.inner.cpr, self.inner.curve
        )
    }
}

// ---------------------------------------------------------------------------
// DefaultCurve + DefaultModelSpec + DefaultEvent
// ---------------------------------------------------------------------------

/// Default curve shape (constant, SDA).
#[pyclass(
    name = "DefaultCurve",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDefaultCurve {
    pub(crate) inner: DefaultCurve,
}

#[pymethods]
impl PyDefaultCurve {
    #[classattr]
    const CONSTANT: Self = Self {
        inner: DefaultCurve::Constant,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, speed_multiplier)")]
    /// SDA (Standard Default Assumption) curve with speed multiplier.
    fn sda(_cls: &Bound<'_, PyType>, speed_multiplier: f64) -> Self {
        Self {
            inner: DefaultCurve::Sda { speed_multiplier },
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Default model specification with CDR and optional curve.
#[pyclass(
    name = "DefaultModelSpec",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDefaultModelSpec {
    pub(crate) inner: DefaultModelSpec,
}

#[pymethods]
impl PyDefaultModelSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, cdr)")]
    /// Constant CDR default model.
    fn constant_cdr(_cls: &Bound<'_, PyType>, cdr: f64) -> Self {
        Self {
            inner: DefaultModelSpec::constant_cdr(cdr),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, speed_multiplier)")]
    /// SDA default model with given speed multiplier.
    fn sda(_cls: &Bound<'_, PyType>, speed_multiplier: f64) -> Self {
        Self {
            inner: DefaultModelSpec::sda(speed_multiplier),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Standard 2% CDR default model.
    fn cdr_2pct(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: DefaultModelSpec::cdr_2pct(),
        }
    }

    #[pyo3(text_signature = "(self, seasoning_months)")]
    /// Compute the Monthly Default Rate (MDR) for a given month.
    fn mdr(&self, seasoning_months: u32) -> f64 {
        self.inner.mdr(seasoning_months)
    }

    #[getter]
    fn cdr(&self) -> f64 {
        self.inner.cdr
    }

    fn __repr__(&self) -> String {
        format!(
            "DefaultModelSpec(cdr={:.4}, curve={:?})",
            self.inner.cdr, self.inner.curve
        )
    }
}

/// A specific default event with date, amount, and recovery parameters.
#[pyclass(
    name = "DefaultEvent",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDefaultEvent {
    pub(crate) inner: DefaultEvent,
}

#[pymethods]
impl PyDefaultEvent {
    #[classmethod]
    #[pyo3(
        signature = (default_date, defaulted_amount, recovery_rate, recovery_lag, *, recovery_bdc=None, recovery_calendar_id=None),
        text_signature = "(cls, default_date, defaulted_amount, recovery_rate, recovery_lag, *, recovery_bdc=None, recovery_calendar_id=None)"
    )]
    fn new(
        _cls: &Bound<'_, PyType>,
        default_date: Bound<'_, PyAny>,
        defaulted_amount: f64,
        recovery_rate: f64,
        recovery_lag: u32,
        recovery_bdc: Option<PyBusinessDayConvention>,
        recovery_calendar_id: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: DefaultEvent {
                default_date: py_to_date(&default_date).context("default_date")?,
                defaulted_amount,
                recovery_rate,
                recovery_lag,
                recovery_bdc: recovery_bdc.map(|b| b.inner),
                recovery_calendar_id,
            },
        })
    }

    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(crate::errors::core_to_py)
    }

    #[getter]
    fn default_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDate>> {
        let d = self.inner.default_date;
        pyo3::types::PyDate::new(py, d.year(), d.month() as u8, d.day())
    }

    #[getter]
    fn defaulted_amount(&self) -> f64 {
        self.inner.defaulted_amount
    }

    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    #[getter]
    fn recovery_lag(&self) -> u32 {
        self.inner.recovery_lag
    }

    fn __repr__(&self) -> String {
        format!(
            "DefaultEvent(date={}, amount={:.2}, recovery={:.2}%)",
            self.inner.default_date,
            self.inner.defaulted_amount,
            self.inner.recovery_rate * 100.0
        )
    }
}

// ---------------------------------------------------------------------------
// RecoveryModelSpec
// ---------------------------------------------------------------------------

/// Recovery model specification with rate and lag.
#[pyclass(
    name = "RecoveryModelSpec",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecoveryModelSpec {
    pub(crate) inner: RecoveryModelSpec,
}

#[pymethods]
impl PyRecoveryModelSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, rate, recovery_lag)")]
    /// Create a recovery model with rate (0.0-1.0) and lag in months.
    fn with_lag(_cls: &Bound<'_, PyType>, rate: f64, recovery_lag: u32) -> Self {
        Self {
            inner: RecoveryModelSpec::with_lag(rate, recovery_lag),
        }
    }

    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(crate::errors::core_to_py)
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    #[getter]
    fn recovery_lag(&self) -> u32 {
        self.inner.recovery_lag
    }

    fn __repr__(&self) -> String {
        format!(
            "RecoveryModelSpec(rate={:.2}%, lag={}m)",
            self.inner.rate * 100.0,
            self.inner.recovery_lag
        )
    }
}

// ---------------------------------------------------------------------------
// FeeTier + evaluate_fee_tiers
// ---------------------------------------------------------------------------

/// Fee tier defining a utilization threshold and corresponding basis point fee.
#[pyclass(
    name = "FeeTier",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeTier {
    pub(crate) inner: FeeTier,
}

#[pymethods]
impl PyFeeTier {
    #[classmethod]
    #[pyo3(text_signature = "(cls, threshold, bps)")]
    /// Create a fee tier from utilization threshold (0.0-1.0) and fee in bps.
    fn from_bps(_cls: &Bound<'_, PyType>, threshold: f64, bps: f64) -> Self {
        Self {
            inner: FeeTier::from_bps(
                threshold,
                finstack_core::types::Bps::new(bps.round() as i32),
            ),
        }
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn bps(&self) -> f64 {
        self.inner.bps.to_f64().unwrap_or(0.0)
    }

    fn __repr__(&self) -> String {
        format!(
            "FeeTier(threshold={:.2}, bps={:.2})",
            self.inner.threshold, self.inner.bps
        )
    }
}

/// Evaluate tiered fees given utilization level.
#[pyfunction]
#[pyo3(text_signature = "(tiers, utilization)")]
pub fn evaluate_fee_tiers(tiers: Vec<PyRef<PyFeeTier>>, utilization: f64) -> f64 {
    let rust_tiers: Vec<FeeTier> = tiers.iter().map(|t| t.inner.clone()).collect();
    let result = finstack_valuations::cashflow::builder::evaluate_fee_tiers(
        &rust_tiers,
        rust_decimal::Decimal::from_f64_retain(utilization).unwrap_or_default(),
    );
    result.to_f64().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// PrincipalEvent
// ---------------------------------------------------------------------------

/// A principal draw/repay event that adjusts outstanding balance.
#[pyclass(
    name = "PrincipalEvent",
    module = "finstack.valuations.cashflow.builder",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrincipalEvent {
    pub(crate) inner: finstack_valuations::cashflow::builder::PrincipalEvent,
}

#[pymethods]
impl PyPrincipalEvent {
    #[classmethod]
    #[pyo3(text_signature = "(cls, date, delta, cash, kind)")]
    /// Create a principal event.
    ///
    /// Args:
    ///     date: Event date
    ///     delta: Notional change (positive = repay, negative = draw)
    ///     cash: Cash flow amount
    ///     kind: Cashflow kind (e.g., CFKind.NOTIONAL)
    fn new(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        delta: Bound<'_, PyAny>,
        cash: Bound<'_, PyAny>,
        kind: &PyCFKind,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: finstack_valuations::cashflow::builder::PrincipalEvent {
                date: py_to_date(&date).context("date")?,
                delta: extract_money(&delta).context("delta")?,
                cash: extract_money(&cash).context("cash")?,
                kind: kind.inner,
            },
        })
    }

    #[getter]
    fn date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDate>> {
        let d = self.inner.date;
        pyo3::types::PyDate::new(py, d.year(), d.month() as u8, d.day())
    }

    #[getter]
    fn delta(&self) -> PyMoney {
        PyMoney::new(self.inner.delta)
    }

    #[getter]
    fn cash(&self) -> PyMoney {
        PyMoney::new(self.inner.cash)
    }

    #[getter]
    fn kind(&self) -> PyCFKind {
        PyCFKind::new(self.inner.kind)
    }

    fn __repr__(&self) -> String {
        format!(
            "PrincipalEvent(date={}, delta={}, cash={})",
            self.inner.date, self.inner.delta, self.inner.cash
        )
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAmortizationSpec>()?;
    module.add_class::<PyNotional>()?;
    module.add_class::<PyOvernightCompoundingMethod>()?;
    module.add_class::<PyFloatingRateSpec>()?;
    module.add_class::<PyPrepaymentCurve>()?;
    module.add_class::<PyPrepaymentModelSpec>()?;
    module.add_class::<PyDefaultCurve>()?;
    module.add_class::<PyDefaultModelSpec>()?;
    module.add_class::<PyDefaultEvent>()?;
    module.add_class::<PyRecoveryModelSpec>()?;
    module.add_class::<PyFeeTier>()?;
    module.add_class::<PyPrincipalEvent>()?;
    module.add_function(wrap_pyfunction!(evaluate_fee_tiers, module)?)?;
    Ok(vec![
        "AmortizationSpec",
        "DefaultCurve",
        "DefaultEvent",
        "DefaultModelSpec",
        "FeeTier",
        "FloatingRateSpec",
        "Notional",
        "OvernightCompoundingMethod",
        "PrepaymentCurve",
        "PrepaymentModelSpec",
        "PrincipalEvent",
        "RecoveryModelSpec",
        "evaluate_fee_tiers",
    ])
}
