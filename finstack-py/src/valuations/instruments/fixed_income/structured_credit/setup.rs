//! Python bindings for structured credit deal configuration types.
//!
//! Wraps `DealConfig`, `DealDates`, `DealFees`, `CoverageTestConfig`,
//! `DefaultAssumptions`, `MarketConditions`, `CreditFactors`, `Metadata`,
//! `Overrides`, and `CreditModelConfig` from `finstack_valuations`.

use crate::core::common::args::TenorArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use finstack_core::currency::Currency;
use finstack_core::HashMap;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CoverageTestConfig as RustCoverageTestConfig, CreditFactors as RustCreditFactors,
    CreditModelConfig as RustCreditModelConfig, DealConfig as RustDealConfig,
    DealDates as RustDealDates, DealFees as RustDealFees, DealType,
    DefaultAssumptions as RustDefaultAssumptions, MarketConditions as RustMarketConditions,
    Metadata as RustMetadata, Overrides as RustOverrides,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ============================================================================
// HELPER: serde round-trip
// ============================================================================

fn to_py_dict<T: serde::Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
    pythonize::pythonize(py, &json_value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
}

fn from_py_dict<T: serde::de::DeserializeOwned>(data: &Bound<'_, PyAny>) -> PyResult<T> {
    let json_value: serde_json::Value = pythonize::depythonize(data)
        .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
    serde_json::from_value(json_value)
        .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {e}")))
}

fn parse_currency(currency: &str) -> PyResult<Currency> {
    currency
        .parse::<Currency>()
        .map_err(|_| PyValueError::new_err(format!("Unknown currency: {currency}")))
}

fn parse_deal_type(deal_type_str: &str) -> PyResult<DealType> {
    match deal_type_str.to_uppercase().as_str() {
        "CLO" => Ok(DealType::CLO),
        "CBO" => Ok(DealType::CBO),
        "ABS" => Ok(DealType::ABS),
        "RMBS" => Ok(DealType::RMBS),
        "CMBS" => Ok(DealType::CMBS),
        "AUTO" => Ok(DealType::Auto),
        "CARD" => Ok(DealType::Card),
        other => Err(PyValueError::new_err(format!("Unknown deal type: {other}"))),
    }
}

// ============================================================================
// DefaultAssumptions
// ============================================================================

/// Default / prepayment / recovery assumptions for structured credit.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DefaultAssumptions",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDefaultAssumptions {
    pub(crate) inner: RustDefaultAssumptions,
}

#[pymethods]
impl PyDefaultAssumptions {
    #[new]
    #[pyo3(signature = (
        base_cdr_annual = None,
        base_recovery_rate = None,
        base_cpr_annual = None,
        psa_speed = None,
        sda_speed = None,
        abs_speed_monthly = None,
    ))]
    fn new(
        base_cdr_annual: Option<f64>,
        base_recovery_rate: Option<f64>,
        base_cpr_annual: Option<f64>,
        psa_speed: Option<f64>,
        sda_speed: Option<f64>,
        abs_speed_monthly: Option<f64>,
    ) -> Self {
        let mut inner = RustDefaultAssumptions::default();
        if let Some(v) = base_cdr_annual {
            inner.base_cdr_annual = v;
        }
        if let Some(v) = base_recovery_rate {
            inner.base_recovery_rate = v;
        }
        if let Some(v) = base_cpr_annual {
            inner.base_cpr_annual = v;
        }
        inner.psa_speed = psa_speed;
        inner.sda_speed = sda_speed;
        inner.abs_speed_monthly = abs_speed_monthly;
        Self { inner }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn clo_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustDefaultAssumptions::clo_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn rmbs_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustDefaultAssumptions::rmbs_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn abs_auto_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustDefaultAssumptions::abs_auto_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn cmbs_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustDefaultAssumptions::cmbs_standard(),
        }
    }

    #[getter]
    fn base_cdr_annual(&self) -> f64 {
        self.inner.base_cdr_annual
    }

    #[getter]
    fn base_recovery_rate(&self) -> f64 {
        self.inner.base_recovery_rate
    }

    #[getter]
    fn base_cpr_annual(&self) -> f64 {
        self.inner.base_cpr_annual
    }

    #[getter]
    fn psa_speed(&self) -> Option<f64> {
        self.inner.psa_speed
    }

    #[getter]
    fn sda_speed(&self) -> Option<f64> {
        self.inner.sda_speed
    }

    #[getter]
    fn abs_speed_monthly(&self) -> Option<f64> {
        self.inner.abs_speed_monthly
    }

    #[getter]
    fn cpr_by_asset_type(&self) -> HashMap<String, f64> {
        self.inner.cpr_by_asset_type.clone()
    }

    #[getter]
    fn cdr_by_asset_type(&self) -> HashMap<String, f64> {
        self.inner.cdr_by_asset_type.clone()
    }

    #[getter]
    fn recovery_by_asset_type(&self) -> HashMap<String, f64> {
        self.inner.recovery_by_asset_type.clone()
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DefaultAssumptions(cdr={:.4}, recovery={:.4}, cpr={:.4})",
            self.inner.base_cdr_annual, self.inner.base_recovery_rate, self.inner.base_cpr_annual
        )
    }
}

// ============================================================================
// DealFees
// ============================================================================

/// Fee structure for a structured credit deal.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealFees",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDealFees {
    pub(crate) inner: RustDealFees,
}

#[pymethods]
impl PyDealFees {
    #[new]
    #[pyo3(signature = (
        trustee_fee_annual,
        senior_mgmt_fee_bps = 0.0,
        subordinated_mgmt_fee_bps = 0.0,
        servicing_fee_bps = 0.0,
        master_servicer_fee_bps = None,
        special_servicer_fee_bps = None,
    ))]
    fn new(
        trustee_fee_annual: &Bound<'_, PyAny>,
        senior_mgmt_fee_bps: f64,
        subordinated_mgmt_fee_bps: f64,
        servicing_fee_bps: f64,
        master_servicer_fee_bps: Option<f64>,
        special_servicer_fee_bps: Option<f64>,
    ) -> PyResult<Self> {
        let fee_money = crate::core::money::extract_money(trustee_fee_annual)?;
        Ok(Self {
            inner: RustDealFees {
                trustee_fee_annual: fee_money,
                senior_mgmt_fee_bps,
                subordinated_mgmt_fee_bps,
                servicing_fee_bps,
                master_servicer_fee_bps,
                special_servicer_fee_bps,
            },
        })
    }

    #[classmethod]
    #[pyo3(signature = (currency = "USD"))]
    fn clo_standard(_cls: &Bound<'_, PyType>, currency: &str) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealFees::clo_standard(parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (currency = "USD"))]
    fn abs_standard(_cls: &Bound<'_, PyType>, currency: &str) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealFees::abs_standard(parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (currency = "USD"))]
    fn cmbs_standard(_cls: &Bound<'_, PyType>, currency: &str) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealFees::cmbs_standard(parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (currency = "USD"))]
    fn rmbs_standard(_cls: &Bound<'_, PyType>, currency: &str) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealFees::rmbs_standard(parse_currency(currency)?),
        })
    }

    #[getter]
    fn trustee_fee_annual(&self) -> PyMoney {
        PyMoney::new(self.inner.trustee_fee_annual)
    }

    #[getter]
    fn senior_mgmt_fee_bps(&self) -> f64 {
        self.inner.senior_mgmt_fee_bps
    }

    #[getter]
    fn subordinated_mgmt_fee_bps(&self) -> f64 {
        self.inner.subordinated_mgmt_fee_bps
    }

    #[getter]
    fn servicing_fee_bps(&self) -> f64 {
        self.inner.servicing_fee_bps
    }

    #[getter]
    fn master_servicer_fee_bps(&self) -> Option<f64> {
        self.inner.master_servicer_fee_bps
    }

    #[getter]
    fn special_servicer_fee_bps(&self) -> Option<f64> {
        self.inner.special_servicer_fee_bps
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DealFees(trustee={}, sr_mgmt_bps={:.1}, sub_mgmt_bps={:.1}, svc_bps={:.1})",
            self.inner.trustee_fee_annual,
            self.inner.senior_mgmt_fee_bps,
            self.inner.subordinated_mgmt_fee_bps,
            self.inner.servicing_fee_bps,
        )
    }
}

// ============================================================================
// DealDates
// ============================================================================

/// Key dates for a structured credit deal.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealDates",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDealDates {
    pub(crate) inner: RustDealDates,
}

#[pymethods]
impl PyDealDates {
    #[new]
    #[pyo3(signature = (closing_date, first_payment_date, maturity, frequency = None))]
    fn new(
        closing_date: &Bound<'_, PyAny>,
        first_payment_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        frequency: Option<TenorArg>,
    ) -> PyResult<Self> {
        let freq = frequency
            .map(|f| f.0)
            .unwrap_or_else(finstack_core::dates::Tenor::quarterly);
        Ok(Self {
            inner: RustDealDates {
                closing_date: py_to_date(closing_date)?,
                first_payment_date: py_to_date(first_payment_date)?,
                reinvestment_end_date: None,
                maturity: py_to_date(maturity)?,
                frequency: freq,
            },
        })
    }

    /// Return a copy with the reinvestment end date set.
    #[pyo3(text_signature = "(self, date)")]
    fn with_reinvestment_end(&self, date: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new_inner = self.inner.clone();
        new_inner.reinvestment_end_date = Some(py_to_date(date)?);
        Ok(Self { inner: new_inner })
    }

    #[getter]
    fn closing_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.closing_date)
    }

    #[getter]
    fn first_payment_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.first_payment_date)
    }

    #[getter]
    fn reinvestment_end_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .reinvestment_end_date
            .map(|d| date_to_py(py, d))
            .transpose()
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.frequency)
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DealDates(closing={}, maturity={}, freq={})",
            self.inner.closing_date, self.inner.maturity, self.inner.frequency,
        )
    }
}

// ============================================================================
// CoverageTestConfig
// ============================================================================

/// OC/IC coverage test configuration.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CoverageTestConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCoverageTestConfig {
    pub(crate) inner: RustCoverageTestConfig,
}

#[pymethods]
impl PyCoverageTestConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustCoverageTestConfig::new(),
        }
    }

    /// Return the default haircuts as a dict (rating -> haircut fraction).
    #[classmethod]
    fn default_haircuts(_cls: &Bound<'_, PyType>) -> PyResult<Py<PyAny>> {
        let py = _cls.py();
        let haircuts = RustCoverageTestConfig::default_haircuts();
        let dict = pyo3::types::PyDict::new(py);
        for (rating, value) in haircuts {
            dict.set_item(format!("{:?}", rating), value)?;
        }
        Ok(dict.into())
    }

    /// Return a copy with an OC test added for the given tranche.
    #[pyo3(text_signature = "(self, tranche_id, trigger_level)")]
    fn add_oc_test(&self, tranche_id: &str, trigger_level: f64) -> Self {
        let mut inner = self.inner.clone();
        inner.add_oc_test(tranche_id, trigger_level);
        Self { inner }
    }

    /// Return a copy with an IC test added for the given tranche.
    #[pyo3(text_signature = "(self, tranche_id, trigger_level)")]
    fn add_ic_test(&self, tranche_id: &str, trigger_level: f64) -> Self {
        let mut inner = self.inner.clone();
        inner.add_ic_test(tranche_id, trigger_level);
        Self { inner }
    }

    #[getter]
    fn oc_triggers(&self) -> HashMap<String, f64> {
        self.inner.oc_triggers.clone()
    }

    #[getter]
    fn ic_triggers(&self) -> HashMap<String, f64> {
        self.inner.ic_triggers.clone()
    }

    #[getter]
    fn haircuts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        for (rating, value) in &self.inner.haircuts {
            dict.set_item(format!("{:?}", rating), *value)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn par_value_threshold(&self) -> Option<f64> {
        self.inner.par_value_threshold
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "CoverageTestConfig(oc_tests={}, ic_tests={})",
            self.inner.oc_triggers.len(),
            self.inner.ic_triggers.len(),
        )
    }
}

// ============================================================================
// MarketConditions
// ============================================================================

/// Market conditions affecting prepayment / default behavior.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MarketConditions",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarketConditions {
    pub(crate) inner: RustMarketConditions,
}

#[pymethods]
impl PyMarketConditions {
    #[new]
    #[pyo3(signature = (refi_rate = 0.0))]
    fn new(refi_rate: f64) -> Self {
        Self {
            inner: RustMarketConditions {
                refi_rate,
                ..RustMarketConditions::default()
            },
        }
    }

    #[getter]
    fn refi_rate(&self) -> f64 {
        self.inner.refi_rate
    }

    #[getter]
    fn original_rate(&self) -> Option<f64> {
        self.inner.original_rate
    }

    #[getter]
    fn hpa(&self) -> Option<f64> {
        self.inner.hpa
    }

    #[getter]
    fn unemployment(&self) -> Option<f64> {
        self.inner.unemployment
    }

    #[getter]
    fn seasonal_factor(&self) -> Option<f64> {
        self.inner.seasonal_factor
    }

    #[getter]
    fn custom_factors(&self) -> HashMap<String, f64> {
        self.inner.custom_factors.clone()
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("MarketConditions(refi_rate={:.4})", self.inner.refi_rate)
    }
}

// ============================================================================
// CreditFactors
// ============================================================================

/// Credit factors affecting default probability.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CreditFactors",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditFactors {
    pub(crate) inner: RustCreditFactors,
}

#[pymethods]
impl PyCreditFactors {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustCreditFactors::default(),
        }
    }

    #[getter]
    fn credit_score(&self) -> Option<u32> {
        self.inner.credit_score
    }

    #[getter]
    fn dti(&self) -> Option<f64> {
        self.inner.dti
    }

    #[getter]
    fn ltv(&self) -> Option<f64> {
        self.inner.ltv
    }

    #[getter]
    fn delinquency_days(&self) -> u32 {
        self.inner.delinquency_days
    }

    #[getter]
    fn unemployment_rate(&self) -> Option<f64> {
        self.inner.unemployment_rate
    }

    #[getter]
    fn custom_factors(&self) -> HashMap<String, f64> {
        self.inner.custom_factors.clone()
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditFactors(score={:?}, dti={:?}, ltv={:?})",
            self.inner.credit_score, self.inner.dti, self.inner.ltv,
        )
    }
}

// ============================================================================
// Metadata (aliased as DealMetadata in existing code)
// ============================================================================

/// Deal metadata (manager, servicer, trustee identifiers).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealMetadata",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMetadata {
    pub(crate) inner: RustMetadata,
}

#[pymethods]
impl PyMetadata {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustMetadata::default(),
        }
    }

    #[getter]
    fn manager_id(&self) -> Option<String> {
        self.inner.manager_id.clone()
    }

    #[getter]
    fn servicer_id(&self) -> Option<String> {
        self.inner.servicer_id.clone()
    }

    #[getter]
    fn master_servicer_id(&self) -> Option<String> {
        self.inner.master_servicer_id.clone()
    }

    #[getter]
    fn special_servicer_id(&self) -> Option<String> {
        self.inner.special_servicer_id.clone()
    }

    #[getter]
    fn trustee_id(&self) -> Option<String> {
        self.inner.trustee_id.clone()
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DealMetadata(manager={:?}, servicer={:?})",
            self.inner.manager_id, self.inner.servicer_id,
        )
    }
}

// ============================================================================
// Overrides (aliased as DealOverrides in existing code)
// ============================================================================

/// Behavioral overrides for structured credit modeling.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealOverrides",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyOverrides {
    pub(crate) inner: RustOverrides,
}

#[pymethods]
impl PyOverrides {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustOverrides::default(),
        }
    }

    #[getter]
    fn cpr_annual(&self) -> Option<f64> {
        self.inner.cpr_annual
    }

    #[getter]
    fn abs_speed(&self) -> Option<f64> {
        self.inner.abs_speed
    }

    #[getter]
    fn psa_speed_multiplier(&self) -> Option<f64> {
        self.inner.psa_speed_multiplier
    }

    #[getter]
    fn cdr_annual(&self) -> Option<f64> {
        self.inner.cdr_annual
    }

    #[getter]
    fn sda_speed_multiplier(&self) -> Option<f64> {
        self.inner.sda_speed_multiplier
    }

    #[getter]
    fn recovery_rate(&self) -> Option<f64> {
        self.inner.recovery_rate
    }

    #[getter]
    fn recovery_lag_months(&self) -> Option<u32> {
        self.inner.recovery_lag_months
    }

    #[getter]
    fn reinvestment_price(&self) -> Option<f64> {
        self.inner.reinvestment_price
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DealOverrides(cpr={:?}, cdr={:?}, recovery={:?})",
            self.inner.cpr_annual, self.inner.cdr_annual, self.inner.recovery_rate,
        )
    }
}

// ============================================================================
// CreditModelConfig
// ============================================================================

/// Credit model configuration (prepayment, default, recovery specs).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CreditModelConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditModelConfig {
    pub(crate) inner: RustCreditModelConfig,
}

#[pymethods]
impl PyCreditModelConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustCreditModelConfig::default(),
        }
    }

    #[getter]
    fn prepayment_spec(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner.prepayment_spec)
    }

    #[getter]
    fn default_spec(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner.default_spec)
    }

    #[getter]
    fn recovery_spec(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner.recovery_spec)
    }

    #[getter]
    fn stochastic_prepay_spec(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .stochastic_prepay_spec
            .as_ref()
            .map(|s| to_py_dict(py, s))
            .transpose()
    }

    #[getter]
    fn stochastic_default_spec(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .stochastic_default_spec
            .as_ref()
            .map(|s| to_py_dict(py, s))
            .transpose()
    }

    #[getter]
    fn correlation_structure(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .correlation_structure
            .as_ref()
            .map(|s| to_py_dict(py, s))
            .transpose()
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        "CreditModelConfig(...)".to_string()
    }
}

// ============================================================================
// DealConfig
// ============================================================================

/// Top-level deal configuration combining dates, fees, tests, and assumptions.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDealConfig {
    pub(crate) inner: RustDealConfig,
}

#[pymethods]
impl PyDealConfig {
    #[classmethod]
    #[pyo3(signature = (deal_type_str, dates, currency = "USD"))]
    fn standard(
        _cls: &Bound<'_, PyType>,
        deal_type_str: &str,
        dates: &PyDealDates,
        currency: &str,
    ) -> PyResult<Self> {
        let dt = parse_deal_type(deal_type_str)?;
        let ccy = parse_currency(currency)?;
        Ok(Self {
            inner: RustDealConfig::standard(dt, dates.inner.clone(), ccy),
        })
    }

    #[classmethod]
    #[pyo3(signature = (dates, currency = "USD"))]
    fn clo_standard(
        _cls: &Bound<'_, PyType>,
        dates: &PyDealDates,
        currency: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealConfig::clo_standard(dates.inner.clone(), parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (dates, currency = "USD"))]
    fn rmbs_standard(
        _cls: &Bound<'_, PyType>,
        dates: &PyDealDates,
        currency: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealConfig::rmbs_standard(dates.inner.clone(), parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (dates, currency = "USD"))]
    fn abs_standard(
        _cls: &Bound<'_, PyType>,
        dates: &PyDealDates,
        currency: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealConfig::abs_standard(dates.inner.clone(), parse_currency(currency)?),
        })
    }

    #[classmethod]
    #[pyo3(signature = (dates, currency = "USD"))]
    fn cmbs_standard(
        _cls: &Bound<'_, PyType>,
        dates: &PyDealDates,
        currency: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RustDealConfig::cmbs_standard(dates.inner.clone(), parse_currency(currency)?),
        })
    }

    #[getter]
    fn dates(&self) -> PyDealDates {
        PyDealDates {
            inner: self.inner.dates.clone(),
        }
    }

    #[getter]
    fn fees(&self) -> PyDealFees {
        PyDealFees {
            inner: self.inner.fees.clone(),
        }
    }

    #[getter]
    fn coverage_tests(&self) -> PyCoverageTestConfig {
        PyCoverageTestConfig {
            inner: self.inner.coverage_tests.clone(),
        }
    }

    #[getter]
    fn default_assumptions(&self) -> PyDefaultAssumptions {
        PyDefaultAssumptions {
            inner: self.inner.default_assumptions.clone(),
        }
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DealConfig(closing={}, maturity={})",
            self.inner.dates.closing_date, self.inner.dates.maturity,
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
    module.add_class::<PyDefaultAssumptions>()?;
    module.add_class::<PyDealFees>()?;
    module.add_class::<PyDealDates>()?;
    module.add_class::<PyCoverageTestConfig>()?;
    module.add_class::<PyMarketConditions>()?;
    module.add_class::<PyCreditFactors>()?;
    module.add_class::<PyMetadata>()?;
    module.add_class::<PyOverrides>()?;
    module.add_class::<PyCreditModelConfig>()?;
    module.add_class::<PyDealConfig>()?;

    Ok(vec![
        "DefaultAssumptions",
        "DealFees",
        "DealDates",
        "CoverageTestConfig",
        "MarketConditions",
        "CreditFactors",
        "DealMetadata",
        "DealOverrides",
        "CreditModelConfig",
        "DealConfig",
    ])
}
