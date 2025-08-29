//! Python bindings for workout and recovery management.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use finstack_valuations::workout::{
    WorkoutEngine, WorkoutState, WorkoutPolicy, WorkoutStrategy,
    RateModification, PrincipalModification, RecoveryWaterfall, RecoveryTier,
    ClaimAmount,
};
use std::collections::HashMap;

use crate::core::dates::PyDate;
use crate::core::money::PyMoney;

/// Python wrapper for workout state.
#[pyclass(name = "WorkoutState")]
#[derive(Clone, Debug)]
pub enum PyWorkoutState {
    Performing(),
    Stressed { indicators: Vec<String> },
    Default { default_date: PyDate, reason: String },
    Workout { start_date: PyDate, workout_type: String },
    Recovered { recovery_date: PyDate, recovery_rate: f64 },
    WrittenOff { writeoff_date: PyDate, loss_amount: PyMoney },
}

#[pymethods]
impl PyWorkoutState {
    #[new]
    #[pyo3(signature = (state_type, **kwargs))]
    pub fn new(state_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match state_type {
            "performing" => Ok(Self::Performing()),
            "stressed" => {
                let indicators = kwargs
                    .and_then(|d| d.get_item("indicators").ok()?)
                    .and_then(|v| v.extract::<Vec<String>>().ok())
                    .unwrap_or_default();
                Ok(Self::Stressed { indicators })
            }
            "default" => {
                let default_date = kwargs
                    .and_then(|d| d.get_item("default_date").ok()?)
                    .and_then(|v| v.extract::<PyDate>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("default_date required"))?;
                let reason = kwargs
                    .and_then(|d| d.get_item("reason").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("reason required"))?;
                Ok(Self::Default { default_date, reason })
            }
            "workout" => {
                let start_date = kwargs
                    .and_then(|d| d.get_item("start_date").ok()?)
                    .and_then(|v| v.extract::<PyDate>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("start_date required"))?;
                let workout_type = kwargs
                    .and_then(|d| d.get_item("workout_type").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("workout_type required"))?;
                Ok(Self::Workout { start_date, workout_type })
            }
            "recovered" => {
                let recovery_date = kwargs
                    .and_then(|d| d.get_item("recovery_date").ok()?)
                    .and_then(|v| v.extract::<PyDate>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("recovery_date required"))?;
                let recovery_rate = kwargs
                    .and_then(|d| d.get_item("recovery_rate").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("recovery_rate required"))?;
                Ok(Self::Recovered { recovery_date, recovery_rate })
            }
            "written_off" => {
                let writeoff_date = kwargs
                    .and_then(|d| d.get_item("writeoff_date").ok()?)
                    .and_then(|v| v.extract::<PyDate>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("writeoff_date required"))?;
                let loss_amount = kwargs
                    .and_then(|d| d.get_item("loss_amount").ok()?)
                    .and_then(|v| v.extract::<PyMoney>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("loss_amount required"))?;
                Ok(Self::WrittenOff { writeoff_date, loss_amount })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown state type: {}", state_type),
            )),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Self::Performing() => "Performing".to_string(),
            Self::Stressed { indicators } => format!("Stressed(indicators={:?})", indicators),
            Self::Default { default_date, reason } => format!("Default(date={}, reason='{}')", default_date, reason),
            Self::Workout { start_date, workout_type } => format!("Workout(start={}, type='{}')", start_date, workout_type),
            Self::Recovered { recovery_date, recovery_rate } => format!("Recovered(date={}, rate={:.2}%)", recovery_date, recovery_rate * 100.0),
            Self::WrittenOff { writeoff_date, loss_amount } => format!("WrittenOff(date={}, loss={})", writeoff_date, loss_amount),
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl PyWorkoutState {
    fn to_rust(&self) -> WorkoutState {
        match self {
            Self::Performing() => WorkoutState::Performing,
            Self::Stressed { indicators } => WorkoutState::Stressed { indicators: indicators.clone() },
            Self::Default { default_date, reason } => WorkoutState::Default { default_date: default_date.inner(), reason: reason.clone() },
            Self::Workout { start_date, workout_type } => WorkoutState::Workout { start_date: start_date.inner(), workout_type: workout_type.clone() },
            Self::Recovered { recovery_date, recovery_rate } => WorkoutState::Recovered { recovery_date: recovery_date.inner(), recovery_rate: *recovery_rate },
            Self::WrittenOff { writeoff_date, loss_amount } => WorkoutState::WrittenOff { writeoff_date: writeoff_date.inner(), loss_amount: loss_amount.inner() },
        }
    }
}

/// Python wrapper for rate modification.
#[pyclass(name = "RateModification")]
#[derive(Clone)]
pub enum PyRateModification {
    ReduceBy { bps: f64 },
    SetTo { rate: f64 },
    ConvertToPIK { pik_rate: f64 },
    SplitCashPIK { cash_rate: f64, pik_rate: f64 },
}

#[pymethods]
impl PyRateModification {
    #[new]
    #[pyo3(signature = (mod_type, **kwargs))]
    pub fn new(mod_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match mod_type {
            "reduce_by" => {
                let bps = kwargs
                    .and_then(|d| d.get_item("bps").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("bps required"))?;
                Ok(Self::ReduceBy { bps })
            }
            "set_to" => {
                let rate = kwargs
                    .and_then(|d| d.get_item("rate").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("rate required"))?;
                Ok(Self::SetTo { rate })
            }
            "convert_to_pik" => {
                let pik_rate = kwargs
                    .and_then(|d| d.get_item("pik_rate").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("pik_rate required"))?;
                Ok(Self::ConvertToPIK { pik_rate })
            }
            "split_cash_pik" => {
                let cash_rate = kwargs
                    .and_then(|d| d.get_item("cash_rate").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("cash_rate required"))?;
                let pik_rate = kwargs
                    .and_then(|d| d.get_item("pik_rate").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("pik_rate required"))?;
                Ok(Self::SplitCashPIK { cash_rate, pik_rate })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown modification type: {}", mod_type),
            )),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Self::ReduceBy { bps } => format!("ReduceBy({} bps)", bps),
            Self::SetTo { rate } => format!("SetTo({:.2}%)", rate * 100.0),
            Self::ConvertToPIK { pik_rate } => format!("ConvertToPIK({:.2}%)", pik_rate * 100.0),
            Self::SplitCashPIK { cash_rate, pik_rate } => 
                format!("SplitCashPIK(cash={:.2}%, pik={:.2}%)", cash_rate * 100.0, pik_rate * 100.0),
        }
    }
}

impl PyRateModification {
    fn to_rust(&self) -> RateModification {
        match self {
            Self::ReduceBy { bps } => RateModification::ReduceBy { bps: *bps },
            Self::SetTo { rate } => RateModification::SetTo { rate: *rate },
            Self::ConvertToPIK { pik_rate } => RateModification::ConvertToPIK { pik_rate: *pik_rate },
            Self::SplitCashPIK { cash_rate, pik_rate } => 
                RateModification::SplitCashPIK { cash_rate: *cash_rate, pik_rate: *pik_rate },
        }
    }
}

/// Python wrapper for principal modification.
#[pyclass(name = "PrincipalModification")]
#[derive(Clone)]
pub enum PyPrincipalModification {
    Forgive { percentage: f64 },
    Defer { percentage: f64 },
    Reamortize { months: i32 },
}

#[pymethods]
impl PyPrincipalModification {
    #[new]
    #[pyo3(signature = (mod_type, **kwargs))]
    pub fn new(mod_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match mod_type {
            "forgive" => {
                let percentage = kwargs
                    .and_then(|d| d.get_item("percentage").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("percentage required"))?;
                Ok(Self::Forgive { percentage })
            }
            "defer" => {
                let percentage = kwargs
                    .and_then(|d| d.get_item("percentage").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("percentage required"))?;
                Ok(Self::Defer { percentage })
            }
            "reamortize" => {
                let months = kwargs
                    .and_then(|d| d.get_item("months").ok()?)
                    .and_then(|v| v.extract::<i32>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("months required"))?;
                Ok(Self::Reamortize { months })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown modification type: {}", mod_type),
            )),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Self::Forgive { percentage } => format!("Forgive({:.1}%)", percentage * 100.0),
            Self::Defer { percentage } => format!("Defer({:.1}%)", percentage * 100.0),
            Self::Reamortize { months } => format!("Reamortize({} months)", months),
        }
    }
}

impl PyPrincipalModification {
    fn to_rust(&self) -> PrincipalModification {
        match self {
            Self::Forgive { percentage } => PrincipalModification::Forgive { percentage: *percentage },
            Self::Defer { percentage } => PrincipalModification::Defer { percentage: *percentage },
            Self::Reamortize { months } => PrincipalModification::Reamortize { months: *months },
        }
    }
}

/// Python wrapper for workout strategy.
#[pyclass(name = "WorkoutStrategy")]
#[derive(Clone)]
pub struct PyWorkoutStrategy {
    pub name: String,
    pub forbearance_months: Option<i32>,
    pub rate_modification: Option<PyRateModification>,
    pub principal_modification: Option<PyPrincipalModification>,
    pub maturity_extension_months: Option<i32>,
    pub additional_collateral: Option<String>,
    pub exit_fee_pct: Option<f64>,
}

#[pymethods]
impl PyWorkoutStrategy {
    #[new]
    #[pyo3(signature = (name, forbearance_months=None, rate_modification=None, principal_modification=None, maturity_extension_months=None, additional_collateral=None, exit_fee_pct=None))]
    pub fn new(
        name: String,
        forbearance_months: Option<i32>,
        rate_modification: Option<PyRateModification>,
        principal_modification: Option<PyPrincipalModification>,
        maturity_extension_months: Option<i32>,
        additional_collateral: Option<String>,
        exit_fee_pct: Option<f64>,
    ) -> Self {
        Self {
            name,
            forbearance_months,
            rate_modification,
            principal_modification,
            maturity_extension_months,
            additional_collateral,
            exit_fee_pct,
        }
    }

    fn __str__(&self) -> String {
        format!("WorkoutStrategy(name='{}')", self.name)
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl PyWorkoutStrategy {
    fn to_rust(&self) -> WorkoutStrategy {
        WorkoutStrategy {
            name: self.name.clone(),
            forbearance_months: self.forbearance_months,
            rate_modification: self.rate_modification.as_ref().map(|m| m.to_rust()),
            principal_modification: self.principal_modification.as_ref().map(|m| m.to_rust()),
            maturity_extension_months: self.maturity_extension_months,
            additional_collateral: self.additional_collateral.clone(),
            exit_fee_pct: self.exit_fee_pct,
        }
    }
}

/// Python wrapper for recovery tier.
#[pyclass(name = "RecoveryTier")]
#[derive(Clone)]
pub struct PyRecoveryTier {
    pub name: String,
    pub claim_type: String,
    pub claim_amount: PyClaimAmount,
    pub recovery_pct: f64,
}

/// Python wrapper for claim amount.
#[pyclass(name = "ClaimAmount")]
#[derive(Clone)]
pub enum PyClaimAmount {
    Fixed { amount: PyMoney },
    PercentOfOutstanding { percentage: f64 },
    Calculated { formula: String },
}

#[pymethods]
impl PyClaimAmount {
    #[new]
    #[pyo3(signature = (amount_type, **kwargs))]
    pub fn new(amount_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match amount_type {
            "fixed" => {
                let amount = kwargs
                    .and_then(|d| d.get_item("amount").ok()?)
                    .and_then(|v| v.extract::<PyMoney>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("amount required"))?;
                Ok(Self::Fixed { amount })
            }
            "percent" => {
                let percentage = kwargs
                    .and_then(|d| d.get_item("percentage").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("percentage required"))?;
                Ok(Self::PercentOfOutstanding { percentage })
            }
            "calculated" => {
                let formula = kwargs
                    .and_then(|d| d.get_item("formula").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("formula required"))?;
                Ok(Self::Calculated { formula })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown amount type: {}", amount_type),
            )),
        }
    }
}

impl PyClaimAmount {
    fn to_rust(&self) -> ClaimAmount {
        match self {
            Self::Fixed { amount } => ClaimAmount::Fixed(amount.inner()),
            Self::PercentOfOutstanding { percentage } => ClaimAmount::PercentOfOutstanding(*percentage),
            Self::Calculated { formula } => ClaimAmount::Calculated(formula.clone()),
        }
    }
}

#[pymethods]
impl PyRecoveryTier {
    #[new]
    pub fn new(
        name: String,
        claim_type: String,
        claim_amount: PyClaimAmount,
        recovery_pct: f64,
    ) -> Self {
        Self {
            name,
            claim_type,
            claim_amount,
            recovery_pct,
        }
    }

    fn __str__(&self) -> String {
        format!("RecoveryTier(name='{}', type='{}', recovery={:.1}%)", 
                self.name, self.claim_type, self.recovery_pct * 100.0)
    }
}

impl PyRecoveryTier {
    fn to_rust(&self) -> RecoveryTier {
        RecoveryTier {
            name: self.name.clone(),
            claim_type: self.claim_type.clone(),
            claim_amount: self.claim_amount.to_rust(),
            recovery_pct: self.recovery_pct,
        }
    }
}

/// Python wrapper for recovery waterfall.
#[pyclass(name = "RecoveryWaterfall")]
#[derive(Clone)]
pub struct PyRecoveryWaterfall {
    pub tiers: Vec<PyRecoveryTier>,
}

#[pymethods]
impl PyRecoveryWaterfall {
    #[new]
    pub fn new(tiers: Vec<PyRecoveryTier>) -> Self {
        Self { tiers }
    }

    fn __str__(&self) -> String {
        format!("RecoveryWaterfall({} tiers)", self.tiers.len())
    }
}

impl PyRecoveryWaterfall {
    fn to_rust(&self) -> RecoveryWaterfall {
        RecoveryWaterfall {
            tiers: self.tiers.iter().map(|t| t.to_rust()).collect(),
        }
    }
}

/// Python wrapper for workout policy.
#[pyclass(name = "WorkoutPolicy")]
#[derive(Clone)]
pub struct PyWorkoutPolicy {
    pub name: String,
    pub stress_thresholds: HashMap<String, f64>,
    pub workout_strategies: HashMap<String, PyWorkoutStrategy>,
    pub recovery_waterfall: PyRecoveryWaterfall,
}

#[pymethods]
impl PyWorkoutPolicy {
    #[new]
    pub fn new(
        name: String,
        recovery_waterfall: PyRecoveryWaterfall,
    ) -> Self {
        Self {
            name,
            stress_thresholds: HashMap::new(),
            workout_strategies: HashMap::new(),
            recovery_waterfall,
        }
    }

    /// Add a stress threshold.
    pub fn add_stress_threshold(&mut self, metric: String, threshold: f64) {
        self.stress_thresholds.insert(metric, threshold);
    }

    /// Add a workout strategy.
    pub fn add_strategy(&mut self, strategy: PyWorkoutStrategy) {
        self.workout_strategies.insert(strategy.name.clone(), strategy);
    }

    fn __str__(&self) -> String {
        format!("WorkoutPolicy(name='{}', strategies={})", 
                self.name, self.workout_strategies.len())
    }
}

impl PyWorkoutPolicy {
    fn to_rust(&self) -> WorkoutPolicy {
        WorkoutPolicy {
            name: self.name.clone(),
            stress_thresholds: self.stress_thresholds.clone(),
            default_triggers: Vec::new(), // Would need proper mapping
            workout_strategies: self.workout_strategies.iter()
                .map(|(k, v)| (k.clone(), v.to_rust()))
                .collect(),
            recovery_waterfall: self.recovery_waterfall.to_rust(),
        }
    }
}

/// Python wrapper for recovery analysis.
#[pyclass(name = "RecoveryAnalysis")]
#[derive(Clone)]
pub struct PyRecoveryAnalysis {
    pub expected_recovery: PyMoney,
    pub recovery_rate: f64,
    pub tier_recoveries: Vec<(String, PyMoney)>,
    pub recovery_schedule: Vec<(PyDate, PyMoney)>,
}

#[pymethods]
impl PyRecoveryAnalysis {
    #[getter]
    pub fn expected_recovery(&self) -> PyMoney {
        self.expected_recovery.clone()
    }

    #[getter]
    pub fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }

    #[getter]
    pub fn tier_recoveries(&self) -> Vec<(String, PyMoney)> {
        self.tier_recoveries.clone()
    }
    
    #[getter]
    pub fn recovery_schedule(&self) -> Vec<(PyDate, PyMoney)> {
        self.recovery_schedule.clone()
    }

    fn __str__(&self) -> String {
        format!("RecoveryAnalysis(expected={}, rate={:.1}%)", 
                self.expected_recovery, self.recovery_rate * 100.0)
    }
}

/// Python wrapper for workout engine.
#[pyclass(name = "WorkoutEngine")]
pub struct PyWorkoutEngine {
    engine: WorkoutEngine,
}

#[pymethods]
impl PyWorkoutEngine {
    #[new]
    pub fn new(policy: PyWorkoutPolicy) -> Self {
        Self {
            engine: WorkoutEngine::new(policy.to_rust()),
        }
    }

    /// Transition to a new state.
    pub fn transition(
        &mut self,
        new_state: PyWorkoutState,
        date: PyDate,
        description: String,
    ) -> PyResult<()> {
        self.engine.transition(new_state.to_rust(), date.inner(), description)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get current state.
    pub fn get_state(&self) -> PyWorkoutState {
        match &self.engine.state {
            WorkoutState::Performing => PyWorkoutState::Performing(),
            WorkoutState::Stressed { indicators } => 
                PyWorkoutState::Stressed { indicators: indicators.clone() },
            WorkoutState::Default { default_date, reason } => 
                PyWorkoutState::Default { 
                    default_date: PyDate::from_core(*default_date),
                    reason: reason.clone(),
                },
            WorkoutState::Workout { start_date, workout_type } => 
                PyWorkoutState::Workout {
                    start_date: PyDate::from_core(*start_date),
                    workout_type: workout_type.clone(),
                },
            WorkoutState::Recovered { recovery_date, recovery_rate } => 
                PyWorkoutState::Recovered {
                    recovery_date: PyDate::from_core(*recovery_date),
                    recovery_rate: *recovery_rate,
                },
            WorkoutState::WrittenOff { writeoff_date, loss_amount } => 
                PyWorkoutState::WrittenOff {
                    writeoff_date: PyDate::from_core(*writeoff_date),
                    loss_amount: PyMoney::from_inner(*loss_amount),
                },
        }
    }

    /// Generate recovery analysis.
    pub fn generate_recovery_analysis(
        &mut self,
        outstanding: PyMoney,
        collateral_value: PyMoney,
        as_of: PyDate,
    ) -> PyResult<PyRecoveryAnalysis> {
        let analysis = self.engine.generate_recovery_flows(
            outstanding.inner(),
            collateral_value.inner(),
            as_of.inner(),
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyRecoveryAnalysis {
            expected_recovery: PyMoney::from_inner(analysis.expected_recovery),
            recovery_rate: analysis.recovery_rate,
            tier_recoveries: analysis.tier_recoveries.into_iter()
                .map(|(name, amount)| (name, PyMoney::from_inner(amount)))
                .collect(),
            recovery_schedule: analysis.recovery_schedule.into_iter()
                .map(|(date, amount)| (PyDate::from_core(date), PyMoney::from_inner(amount)))
                .collect(),
        })
    }

    fn __str__(&self) -> String {
        format!("WorkoutEngine(state={:?}, events={})", 
                self.get_state(), self.engine.events.len())
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Register the workout module with Python.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "workout")?;
    
    m.add_class::<PyWorkoutState>()?;
    m.add_class::<PyRateModification>()?;
    m.add_class::<PyPrincipalModification>()?;
    m.add_class::<PyWorkoutStrategy>()?;
    m.add_class::<PyClaimAmount>()?;
    m.add_class::<PyRecoveryTier>()?;
    m.add_class::<PyRecoveryWaterfall>()?;
    m.add_class::<PyWorkoutPolicy>()?;
    m.add_class::<PyRecoveryAnalysis>()?;
    m.add_class::<PyWorkoutEngine>()?;
    
    parent.add_submodule(&m)?;
    parent.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.workout", &m)?;
    
    Ok(())
}
