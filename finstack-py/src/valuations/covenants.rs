//! Python bindings for covenant evaluation and management.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use finstack_valuations::covenants::{CovenantBreach, CovenantEngine, CovenantSpec};
use finstack_valuations::instruments::fixed_income::loan::covenants::{
    Covenant, CovenantConsequence, CovenantType, ThresholdTest,
};
use finstack_valuations::metrics::MetricId;
use finstack_valuations::results::CovenantReport;
use std::collections::HashMap;

use crate::core::dates::PyDate;
use crate::core::dates::PyFrequency;

/// Python wrapper for covenant types.
#[pyclass(name = "CovenantType")]
#[derive(Clone)]
pub enum PyCovenantType {
    /// Maximum debt to EBITDA ratio
    MaxDebtToEBITDA { threshold: f64 },
    /// Minimum interest coverage ratio
    MinInterestCoverage { threshold: f64 },
    /// Minimum fixed charge coverage ratio
    MinFixedChargeCoverage { threshold: f64 },
    /// Maximum total leverage ratio
    MaxTotalLeverage { threshold: f64 },
    /// Maximum senior leverage ratio
    MaxSeniorLeverage { threshold: f64 },
    /// Minimum asset coverage ratio
    MinAssetCoverage { threshold: f64 },
    /// Negative covenant
    Negative { restriction: String },
    /// Affirmative covenant
    Affirmative { requirement: String },
    /// Custom metric test
    Custom {
        metric: String,
        test_type: String,
        threshold: f64,
    },
}

#[pymethods]
impl PyCovenantType {
    #[new]
    #[pyo3(signature = (covenant_type, **kwargs))]
    pub fn new(covenant_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match covenant_type {
            "max_debt_to_ebitda" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MaxDebtToEBITDA { threshold })
            }
            "min_interest_coverage" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MinInterestCoverage { threshold })
            }
            "min_fixed_charge_coverage" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MinFixedChargeCoverage { threshold })
            }
            "max_total_leverage" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MaxTotalLeverage { threshold })
            }
            "max_senior_leverage" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MaxSeniorLeverage { threshold })
            }
            "min_asset_coverage" => {
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::MinAssetCoverage { threshold })
            }
            "negative" => {
                let restriction = kwargs
                    .and_then(|d| d.get_item("restriction").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("restriction required")
                    })?;
                Ok(Self::Negative { restriction })
            }
            "affirmative" => {
                let requirement = kwargs
                    .and_then(|d| d.get_item("requirement").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("requirement required")
                    })?;
                Ok(Self::Affirmative { requirement })
            }
            "custom" => {
                let metric = kwargs
                    .and_then(|d| d.get_item("metric").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("metric required")
                    })?;
                let test_type = kwargs
                    .and_then(|d| d.get_item("test_type").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .unwrap_or_else(|| "maximum".to_string());
                let threshold = kwargs
                    .and_then(|d| d.get_item("threshold").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("threshold required")
                    })?;
                Ok(Self::Custom {
                    metric,
                    test_type,
                    threshold,
                })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown covenant type: {}",
                covenant_type
            ))),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Self::MaxDebtToEBITDA { threshold } => {
                format!("MaxDebtToEBITDA(threshold={})", threshold)
            }
            Self::MinInterestCoverage { threshold } => {
                format!("MinInterestCoverage(threshold={})", threshold)
            }
            Self::MinFixedChargeCoverage { threshold } => {
                format!("MinFixedChargeCoverage(threshold={})", threshold)
            }
            Self::MaxTotalLeverage { threshold } => {
                format!("MaxTotalLeverage(threshold={})", threshold)
            }
            Self::MaxSeniorLeverage { threshold } => {
                format!("MaxSeniorLeverage(threshold={})", threshold)
            }
            Self::MinAssetCoverage { threshold } => {
                format!("MinAssetCoverage(threshold={})", threshold)
            }
            Self::Negative { restriction } => format!("Negative(restriction='{}')", restriction),
            Self::Affirmative { requirement } => {
                format!("Affirmative(requirement='{}')", requirement)
            }
            Self::Custom {
                metric,
                test_type,
                threshold,
            } => format!(
                "Custom(metric='{}', test_type='{}', threshold={})",
                metric, test_type, threshold
            ),
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl PyCovenantType {
    fn to_rust(&self) -> CovenantType {
        match self {
            Self::MaxDebtToEBITDA { threshold } => CovenantType::MaxDebtToEBITDA {
                threshold: *threshold,
            },
            Self::MinInterestCoverage { threshold } => CovenantType::MinInterestCoverage {
                threshold: *threshold,
            },
            Self::MinFixedChargeCoverage { threshold } => CovenantType::MinFixedChargeCoverage {
                threshold: *threshold,
            },
            Self::MaxTotalLeverage { threshold } => CovenantType::MaxTotalLeverage {
                threshold: *threshold,
            },
            Self::MaxSeniorLeverage { threshold } => CovenantType::MaxSeniorLeverage {
                threshold: *threshold,
            },
            Self::MinAssetCoverage { threshold } => CovenantType::MinAssetCoverage {
                threshold: *threshold,
            },
            Self::Negative { restriction } => CovenantType::Negative {
                restriction: restriction.clone(),
            },
            Self::Affirmative { requirement } => CovenantType::Affirmative {
                requirement: requirement.clone(),
            },
            Self::Custom {
                metric,
                test_type,
                threshold,
            } => {
                let test = if test_type == "minimum" {
                    ThresholdTest::Minimum(*threshold)
                } else {
                    ThresholdTest::Maximum(*threshold)
                };
                CovenantType::Custom {
                    metric: metric.clone(),
                    test,
                }
            }
        }
    }
}

/// Python wrapper for covenant consequence.
#[pyclass(name = "CovenantConsequence")]
#[derive(Clone)]
pub enum PyCovenantConsequence {
    /// Event of default
    Default(),
    /// Increase in interest rate
    RateIncrease { bp_increase: f64 },
    /// Mandatory cash sweep
    CashSweep { sweep_percentage: f64 },
    /// Block on distributions
    BlockDistributions(),
    /// Require additional collateral
    RequireCollateral { description: String },
    /// Accelerate maturity
    AccelerateMaturity { new_maturity: PyDate },
}

#[pymethods]
impl PyCovenantConsequence {
    #[new]
    #[pyo3(signature = (consequence_type, **kwargs))]
    pub fn new(consequence_type: &str, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        match consequence_type {
            "default" => Ok(Self::Default()),
            "rate_increase" => {
                let bp_increase = kwargs
                    .and_then(|d| d.get_item("bp_increase").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("bp_increase required")
                    })?;
                Ok(Self::RateIncrease { bp_increase })
            }
            "cash_sweep" => {
                let sweep_percentage = kwargs
                    .and_then(|d| d.get_item("sweep_percentage").ok()?)
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("sweep_percentage required")
                    })?;
                Ok(Self::CashSweep { sweep_percentage })
            }
            "block_distributions" => Ok(Self::BlockDistributions()),
            "require_collateral" => {
                let description = kwargs
                    .and_then(|d| d.get_item("description").ok()?)
                    .and_then(|v| v.extract::<String>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("description required")
                    })?;
                Ok(Self::RequireCollateral { description })
            }
            "accelerate_maturity" => {
                let new_maturity = kwargs
                    .and_then(|d| d.get_item("new_maturity").ok()?)
                    .and_then(|v| v.extract::<PyDate>().ok())
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("new_maturity required")
                    })?;
                Ok(Self::AccelerateMaturity { new_maturity })
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown consequence type: {}",
                consequence_type
            ))),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Self::Default() => "Default".to_string(),
            Self::RateIncrease { bp_increase } => {
                format!("RateIncrease(bp_increase={})", bp_increase)
            }
            Self::CashSweep { sweep_percentage } => {
                format!("CashSweep(sweep_percentage={})", sweep_percentage)
            }
            Self::BlockDistributions() => "BlockDistributions".to_string(),
            Self::RequireCollateral { description } => {
                format!("RequireCollateral(description='{}')", description)
            }
            Self::AccelerateMaturity { new_maturity } => {
                format!("AccelerateMaturity(new_maturity={})", new_maturity)
            }
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl PyCovenantConsequence {
    fn to_rust(&self) -> CovenantConsequence {
        match self {
            Self::Default() => CovenantConsequence::Default,
            Self::RateIncrease { bp_increase } => CovenantConsequence::RateIncrease {
                bp_increase: *bp_increase,
            },
            Self::CashSweep { sweep_percentage } => CovenantConsequence::CashSweep {
                sweep_percentage: *sweep_percentage,
            },
            Self::BlockDistributions() => CovenantConsequence::BlockDistributions,
            Self::RequireCollateral { description } => CovenantConsequence::RequireCollateral {
                description: description.clone(),
            },
            Self::AccelerateMaturity { new_maturity } => CovenantConsequence::AccelerateMaturity {
                new_maturity: new_maturity.inner(),
            },
        }
    }
}

/// Python wrapper for covenant.
#[pyclass(name = "Covenant")]
#[derive(Clone)]
pub struct PyCovenant {
    pub inner: Covenant,
}

#[pymethods]
impl PyCovenant {
    #[new]
    #[pyo3(signature = (covenant_type, test_frequency, cure_period_days=None))]
    pub fn new(
        covenant_type: PyCovenantType,
        test_frequency: PyFrequency,
        cure_period_days: Option<i32>,
    ) -> Self {
        let mut covenant = Covenant::new(covenant_type.to_rust(), test_frequency.inner());
        if let Some(days) = cure_period_days {
            covenant = covenant.with_cure_period(Some(days));
        }
        Self { inner: covenant }
    }

    /// Add a consequence to the covenant.
    pub fn add_consequence(&mut self, consequence: PyCovenantConsequence) -> PyResult<()> {
        self.inner.consequences.push(consequence.to_rust());
        Ok(())
    }

    /// Set whether the covenant is active.
    pub fn set_active(&mut self, active: bool) -> PyResult<()> {
        self.inner.is_active = active;
        Ok(())
    }

    /// Get description of the covenant.
    pub fn description(&self) -> String {
        self.inner.description()
    }

    fn __str__(&self) -> String {
        self.description()
    }

    fn __repr__(&self) -> String {
        format!("Covenant({})", self.description())
    }
}

/// Python wrapper for covenant report.
#[pyclass(name = "CovenantReport")]
#[derive(Clone)]
pub struct PyCovenantReport {
    pub covenant_type: String,
    pub passed: bool,
    pub actual_value: Option<f64>,
    pub threshold: Option<f64>,
    pub details: Option<String>,
}

#[pymethods]
impl PyCovenantReport {
    /// Check if the covenant passed.
    #[getter]
    pub fn passed(&self) -> bool {
        self.passed
    }

    /// Get the covenant type.
    #[getter]
    pub fn covenant_type(&self) -> &str {
        &self.covenant_type
    }

    /// Get the actual value.
    #[getter]
    pub fn actual_value(&self) -> Option<f64> {
        self.actual_value
    }

    /// Get the threshold.
    #[getter]
    pub fn threshold(&self) -> Option<f64> {
        self.threshold
    }

    /// Get details.
    #[getter]
    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    fn __str__(&self) -> String {
        let status = if self.passed { "PASSED" } else { "FAILED" };
        let mut result = format!("{}: {}", self.covenant_type, status);
        if let Some(actual) = self.actual_value {
            result.push_str(&format!(" (actual: {:.2}", actual));
            if let Some(thresh) = self.threshold {
                result.push_str(&format!(", threshold: {:.2}", thresh));
            }
            result.push(')');
        }
        if let Some(ref details) = self.details {
            result.push_str(&format!(" - {}", details));
        }
        result
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<CovenantReport> for PyCovenantReport {
    fn from(report: CovenantReport) -> Self {
        Self {
            covenant_type: report.covenant_type,
            passed: report.passed,
            actual_value: report.actual_value,
            threshold: report.threshold,
            details: report.details,
        }
    }
}

/// Python wrapper for covenant breach.
#[pyclass(name = "CovenantBreach")]
#[derive(Clone)]
pub struct PyCovenantBreach {
    pub covenant_type: String,
    pub breach_date: PyDate,
    pub actual_value: Option<f64>,
    pub threshold: Option<f64>,
    pub cure_deadline: Option<PyDate>,
    pub is_cured: bool,
}

#[pymethods]
impl PyCovenantBreach {
    #[new]
    pub fn new(
        covenant_type: String,
        breach_date: PyDate,
        actual_value: Option<f64>,
        threshold: Option<f64>,
        cure_deadline: Option<PyDate>,
    ) -> Self {
        Self {
            covenant_type,
            breach_date,
            actual_value,
            threshold,
            cure_deadline,
            is_cured: false,
        }
    }

    /// Mark the breach as cured.
    pub fn mark_cured(&mut self) {
        self.is_cured = true;
    }

    #[getter]
    pub fn covenant_type(&self) -> &str {
        &self.covenant_type
    }

    #[getter]
    pub fn breach_date(&self) -> PyDate {
        self.breach_date.clone()
    }

    #[getter]
    pub fn is_cured(&self) -> bool {
        self.is_cured
    }

    fn __str__(&self) -> String {
        let status = if self.is_cured { "CURED" } else { "ACTIVE" };
        format!(
            "CovenantBreach({} on {} - {})",
            self.covenant_type, self.breach_date, status
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Python wrapper for covenant engine.
#[pyclass(name = "CovenantEngine")]
pub struct PyCovenantEngine {
    engine: CovenantEngine,
}

impl Default for PyCovenantEngine {
    fn default() -> Self {
        Self {
            engine: CovenantEngine::new(),
        }
    }
}

#[pymethods]
impl PyCovenantEngine {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a covenant specification.
    pub fn add_covenant(&mut self, covenant: PyCovenant) -> PyResult<()> {
        let spec = CovenantSpec::with_metric(
            covenant.inner.clone(),
            MetricId::custom("dummy"), // This would need proper metric mapping
        );
        self.engine.add_spec(spec);
        Ok(())
    }

    /// Register a custom metric calculator.
    pub fn register_metric(&mut self, _name: String, _calculator: PyObject) -> PyResult<()> {
        // Note: This would require wrapping Python callable in Rust
        // For now, we'll skip the implementation
        Ok(())
    }

    /// Evaluate covenants and return reports.
    pub fn evaluate(
        &self,
        _context: PyObject,
        _test_date: PyDate,
    ) -> PyResult<HashMap<String, PyCovenantReport>> {
        // Note: This would require proper MetricContext wrapping
        // For now, return empty results
        Ok(HashMap::new())
    }

    /// Add a breach to history.
    pub fn add_breach(&mut self, breach: PyCovenantBreach) -> PyResult<()> {
        self.engine.breach_history.push(CovenantBreach {
            covenant_type: breach.covenant_type.clone(),
            breach_date: breach.breach_date.inner(),
            actual_value: breach.actual_value,
            threshold: breach.threshold,
            cure_deadline: breach.cure_deadline.map(|d| d.inner()),
            is_cured: breach.is_cured,
            applied_consequences: Vec::new(),
        });
        Ok(())
    }

    fn __str__(&self) -> String {
        format!(
            "CovenantEngine(specs={}, breaches={})",
            self.engine.specs.len(),
            self.engine.breach_history.len()
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Register the covenants module with Python.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "covenants")?;

    m.add_class::<PyCovenantType>()?;
    m.add_class::<PyCovenantConsequence>()?;
    m.add_class::<PyCovenant>()?;
    m.add_class::<PyCovenantReport>()?;
    m.add_class::<PyCovenantBreach>()?;
    m.add_class::<PyCovenantEngine>()?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.covenants", &m)?;

    Ok(())
}
