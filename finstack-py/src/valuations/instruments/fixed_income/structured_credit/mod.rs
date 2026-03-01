//! Python bindings for structured credit instruments (ABS, RMBS, CMBS, CLO).

pub(crate) mod waterfall;

use crate::core::common::args::TenorArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::Tenor;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CreditFactors, CreditModelConfig, DealType, DefaultAssumptions, MarketConditions,
    Metadata as DealMetadata, Overrides as DealOverrides, Pool, Seniority, StructuredCredit,
    TrancheStructure,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use std::fmt;
use std::sync::Arc;

fn parse_structured_credit_json(value: &Bound<'_, PyAny>) -> PyResult<StructuredCredit> {
    if let Ok(json_str) = value.extract::<&str>() {
        return serde_json::from_str(json_str)
            .map_err(|err| PyValueError::new_err(err.to_string()));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        use crate::errors::PyContext;
        let py = dict.py();
        let json = pyo3::types::PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()
            .context("json dumps")?;
        return serde_json::from_str(&json).map_err(|err| PyValueError::new_err(err.to_string()));
    }
    Err(PyTypeError::new_err(
        "Expected JSON string or dict convertible to JSON",
    ))
}

// ============================================================================
// ENUM WRAPPERS
// ============================================================================

/// Structured credit deal type classification.
///
/// Provides class-level constants for each deal type variant:
/// ``DealType.CLO``, ``DealType.ABS``, ``DealType.RMBS``, etc.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DealType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDealType {
    pub(crate) inner: DealType,
}

#[pymethods]
impl PyDealType {
    #[classattr]
    #[pyo3(name = "CLO")]
    fn clo() -> Self {
        Self {
            inner: DealType::CLO,
        }
    }
    #[classattr]
    #[pyo3(name = "CBO")]
    fn cbo() -> Self {
        Self {
            inner: DealType::CBO,
        }
    }
    #[classattr]
    #[pyo3(name = "ABS")]
    fn abs_() -> Self {
        Self {
            inner: DealType::ABS,
        }
    }
    #[classattr]
    #[pyo3(name = "RMBS")]
    fn rmbs() -> Self {
        Self {
            inner: DealType::RMBS,
        }
    }
    #[classattr]
    #[pyo3(name = "CMBS")]
    fn cmbs() -> Self {
        Self {
            inner: DealType::CMBS,
        }
    }
    #[classattr]
    #[pyo3(name = "AUTO")]
    fn auto() -> Self {
        Self {
            inner: DealType::Auto,
        }
    }
    #[classattr]
    #[pyo3(name = "CARD")]
    fn card() -> Self {
        Self {
            inner: DealType::Card,
        }
    }

    fn __repr__(&self) -> String {
        format!("DealType.{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut h);
        h.finish()
    }
}

/// Tranche seniority classification.
///
/// Provides class-level constants: ``TrancheSeniority.SENIOR``,
/// ``TrancheSeniority.MEZZANINE``, ``TrancheSeniority.SUBORDINATED``,
/// ``TrancheSeniority.EQUITY``.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheSeniority",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTrancheSeniority {
    pub(crate) inner: Seniority,
}

#[pymethods]
impl PyTrancheSeniority {
    #[classattr]
    #[pyo3(name = "SENIOR")]
    fn senior() -> Self {
        Self {
            inner: Seniority::Senior,
        }
    }
    #[classattr]
    #[pyo3(name = "MEZZANINE")]
    fn mezzanine() -> Self {
        Self {
            inner: Seniority::Mezzanine,
        }
    }
    #[classattr]
    #[pyo3(name = "SUBORDINATED")]
    fn subordinated() -> Self {
        Self {
            inner: Seniority::Subordinated,
        }
    }
    #[classattr]
    #[pyo3(name = "EQUITY")]
    fn equity() -> Self {
        Self {
            inner: Seniority::Equity,
        }
    }

    fn __repr__(&self) -> String {
        format!("TrancheSeniority.{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut h);
        h.finish()
    }
}

// ============================================================================
// STRUCTURED CREDIT INSTRUMENT
// ============================================================================

/// Unified structured credit instrument wrapper (ABS, CLO, CMBS, RMBS).
///
/// This single Python class provides a cleaner API with deal type discrimination.
///
/// Examples:
///     >>> deal = StructuredCredit.from_json(json.dumps({...}))
///     >>> deal.instrument_id
///     'clo_2024_1'
///     >>> deal.deal_type
///     'CLO'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StructuredCredit",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStructuredCredit {
    pub(crate) inner: Arc<StructuredCredit>,
}

impl PyStructuredCredit {
    pub(crate) fn new(inner: StructuredCredit) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyStructuredCredit {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Parse a JSON payload into a structured credit instrument.
    ///
    /// Args:
    ///     data: JSON string or dict describing the structured credit deal.
    ///
    /// Returns:
    ///     StructuredCredit: Parsed structured credit instrument wrapper.
    ///
    /// Raises:
    ///     ValueError: If the JSON cannot be parsed.
    ///     TypeError: If ``data`` is neither a string nor dict-like object.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let deal = parse_structured_credit_json(&data)?;
        Ok(Self::new(deal))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the deal.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Deal type classification (ABS, CLO, CMBS, or RMBS).
    ///
    /// Returns:
    ///     str: Deal type string.
    #[getter]
    fn deal_type(&self) -> String {
        format!("{:?}", self.inner.deal_type)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.StructuredCredit``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::StructuredCredit)
    }

    /// Closing date of the deal.
    #[getter]
    fn closing_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.closing_date)
    }

    /// First payment date.
    #[getter]
    fn first_payment_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.first_payment_date)
    }

    /// Legal maturity date.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Payment frequency as a string.
    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.frequency)
    }

    /// Base CDR annual rate from default assumptions.
    #[getter]
    fn base_cdr_annual(&self) -> f64 {
        self.inner.default_assumptions.base_cdr_annual
    }

    /// Base recovery rate from default assumptions.
    #[getter]
    fn base_recovery_rate(&self) -> f64 {
        self.inner.default_assumptions.base_recovery_rate
    }

    /// Base CPR annual rate from default assumptions.
    #[getter]
    fn base_cpr_annual(&self) -> f64 {
        self.inner.default_assumptions.base_cpr_annual
    }

    #[pyo3(text_signature = "(self)")]
    /// Serialize the structured credit definition back to JSON.
    ///
    /// Returns:
    ///     str: Pretty-printed JSON representation of the instrument.
    ///
    /// Raises:
    ///     ValueError: If serialization fails.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    /// Number of tranches in the structure.
    ///
    /// Returns:
    ///     int: Count of tranches.
    #[getter]
    fn tranche_count(&self) -> usize {
        self.inner.tranches.tranches.len()
    }

    /// Create a fluent builder for constructing a StructuredCredit instrument.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for the instrument.
    ///
    /// Returns:
    ///     StructuredCreditBuilder: Builder instance with fluent setter methods.
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyStructuredCreditBuilder {
        PyStructuredCreditBuilder::new_with_id(InstrumentId::new(instrument_id))
    }
}

impl fmt::Display for PyStructuredCredit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StructuredCredit({:?}, id={}, tranches={})",
            self.inner.deal_type,
            self.inner.id,
            self.inner.tranches.tranches.len()
        )
    }
}

// ============================================================================
// JSON HELPER
// ============================================================================

/// Convert a Python dict or JSON string into a Rust JSON string.
fn extract_json_str(value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = value.extract::<String>() {
        return Ok(s);
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let py = dict.py();
        let json = PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize dict: {e}")))?;
        return Ok(json);
    }
    Err(PyTypeError::new_err("Expected JSON string or dict"))
}

// ============================================================================
// BUILDER
// ============================================================================

/// Fluent builder for ``StructuredCredit`` instruments.
///
/// Top-level scalar fields are set via typed methods. Complex nested configs
/// (pool, tranches, credit model, etc.) accept either a Python ``dict`` or a
/// JSON string, which are deserialized internally.
///
/// Examples:
///     >>> import datetime
///     >>> builder = StructuredCredit.builder("clo_2024_1")
///     >>> builder = builder.deal_type(DealType.CLO)
///     >>> builder = builder.closing_date(datetime.date(2024, 1, 15))
///     >>> # ... set remaining required fields ...
///     >>> deal = builder.build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StructuredCreditBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyStructuredCreditBuilder {
    instrument_id: InstrumentId,
    deal_type: Option<DealType>,
    closing_date: Option<time::Date>,
    first_payment_date: Option<time::Date>,
    maturity: Option<time::Date>,
    reinvestment_end_date: Option<time::Date>,
    frequency: Tenor,
    discount_curve_id: Option<CurveId>,
    pool_json: Option<String>,
    tranches_json: Option<String>,
    default_assumptions_json: Option<String>,
    market_conditions_json: Option<String>,
    credit_factors_json: Option<String>,
    deal_metadata_json: Option<String>,
    behavior_overrides_json: Option<String>,
    credit_model_json: Option<String>,
    cleanup_call_pct: Option<f64>,
}

impl PyStructuredCreditBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            deal_type: None,
            closing_date: None,
            first_payment_date: None,
            maturity: None,
            reinvestment_end_date: None,
            frequency: Tenor::quarterly(),
            discount_curve_id: None,
            pool_json: None,
            tranches_json: None,
            default_assumptions_json: None,
            market_conditions_json: None,
            credit_factors_json: None,
            deal_metadata_json: None,
            behavior_overrides_json: None,
            credit_model_json: None,
            cleanup_call_pct: None,
        }
    }
}

#[pymethods]
impl PyStructuredCreditBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Set the deal type classification.
    #[pyo3(text_signature = "($self, deal_type)")]
    fn deal_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        deal_type: &'py PyDealType,
    ) -> PyRefMut<'py, Self> {
        slf.deal_type = Some(deal_type.inner);
        slf
    }

    /// Set the closing (issuance) date.
    #[pyo3(text_signature = "($self, date)")]
    fn closing_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.closing_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    /// Set the first payment date.
    #[pyo3(text_signature = "($self, date)")]
    fn first_payment_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.first_payment_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    /// Set the legal maturity date.
    #[pyo3(text_signature = "($self, date)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&date)?);
        Ok(slf)
    }

    /// Set the reinvestment period end date (optional).
    #[pyo3(signature = (date=None))]
    fn reinvestment_end_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.reinvestment_end_date = date.map(|d| py_to_date(&d)).transpose()?;
        Ok(slf)
    }

    /// Set the payment frequency (e.g. ``"3m"``, ``"1q"``).
    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.frequency = frequency.0;
        slf
    }

    /// Set the discount curve identifier.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id<'py>(mut slf: PyRefMut<'py, Self>, curve_id: &'py str) -> PyRefMut<'py, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    /// Set the collateral pool from a dict or JSON string.
    #[pyo3(text_signature = "($self, pool)")]
    fn pool<'py>(
        mut slf: PyRefMut<'py, Self>,
        pool: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pool_json = Some(extract_json_str(&pool)?);
        Ok(slf)
    }

    /// Set the tranche structure from a dict or JSON string.
    #[pyo3(text_signature = "($self, tranches)")]
    fn tranches<'py>(
        mut slf: PyRefMut<'py, Self>,
        tranches: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.tranches_json = Some(extract_json_str(&tranches)?);
        Ok(slf)
    }

    /// Set default assumptions from a dict or JSON string.
    #[pyo3(text_signature = "($self, assumptions)")]
    fn default_assumptions<'py>(
        mut slf: PyRefMut<'py, Self>,
        assumptions: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.default_assumptions_json = Some(extract_json_str(&assumptions)?);
        Ok(slf)
    }

    /// Set market conditions from a dict or JSON string.
    #[pyo3(text_signature = "($self, conditions)")]
    fn market_conditions<'py>(
        mut slf: PyRefMut<'py, Self>,
        conditions: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.market_conditions_json = Some(extract_json_str(&conditions)?);
        Ok(slf)
    }

    /// Set credit factors from a dict or JSON string.
    #[pyo3(text_signature = "($self, factors)")]
    fn credit_factors<'py>(
        mut slf: PyRefMut<'py, Self>,
        factors: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.credit_factors_json = Some(extract_json_str(&factors)?);
        Ok(slf)
    }

    /// Set deal metadata from a dict or JSON string.
    #[pyo3(text_signature = "($self, metadata)")]
    fn deal_metadata<'py>(
        mut slf: PyRefMut<'py, Self>,
        metadata: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.deal_metadata_json = Some(extract_json_str(&metadata)?);
        Ok(slf)
    }

    /// Set behavioral overrides from a dict or JSON string.
    #[pyo3(text_signature = "($self, overrides)")]
    fn behavior_overrides<'py>(
        mut slf: PyRefMut<'py, Self>,
        overrides: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.behavior_overrides_json = Some(extract_json_str(&overrides)?);
        Ok(slf)
    }

    /// Set the credit model config from a dict or JSON string.
    #[pyo3(text_signature = "($self, config)")]
    fn credit_model<'py>(
        mut slf: PyRefMut<'py, Self>,
        config: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.credit_model_json = Some(extract_json_str(&config)?);
        Ok(slf)
    }

    /// Set the clean-up call threshold (fraction of original pool balance).
    #[pyo3(signature = (pct=None))]
    fn cleanup_call_pct(mut slf: PyRefMut<'_, Self>, pct: Option<f64>) -> PyRefMut<'_, Self> {
        slf.cleanup_call_pct = pct;
        slf
    }

    /// Build the ``StructuredCredit`` instrument.
    ///
    /// Required fields: ``deal_type``, ``closing_date``, ``first_payment_date``,
    /// ``maturity``, ``disc_id``, ``pool``, ``tranches``.
    ///
    /// Raises:
    ///     ValueError: If a required field is missing or JSON is invalid.
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyStructuredCredit> {
        let deal_type = slf
            .deal_type
            .ok_or_else(|| PyValueError::new_err("deal_type() is required"))?;
        let closing_date = slf
            .closing_date
            .ok_or_else(|| PyValueError::new_err("closing_date() is required"))?;
        let first_payment_date = slf
            .first_payment_date
            .ok_or_else(|| PyValueError::new_err("first_payment_date() is required"))?;
        let maturity = slf
            .maturity
            .ok_or_else(|| PyValueError::new_err("maturity() is required"))?;
        let discount_curve_id = slf
            .discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("disc_id() is required"))?;

        let pool: Pool = slf
            .pool_json
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("pool() is required"))
            .and_then(|j| {
                serde_json::from_str(j)
                    .map_err(|e| PyValueError::new_err(format!("Invalid pool JSON: {e}")))
            })?;

        let tranches: TrancheStructure = slf
            .tranches_json
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("tranches() is required"))
            .and_then(|j| {
                serde_json::from_str(j)
                    .map_err(|e| PyValueError::new_err(format!("Invalid tranches JSON: {e}")))
            })?;

        let default_assumptions: DefaultAssumptions = slf
            .default_assumptions_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j).map_err(|e| {
                    PyValueError::new_err(format!("Invalid default_assumptions JSON: {e}"))
                })
            })
            .transpose()?
            .unwrap_or_default();

        let market_conditions: MarketConditions = slf
            .market_conditions_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j).map_err(|e| {
                    PyValueError::new_err(format!("Invalid market_conditions JSON: {e}"))
                })
            })
            .transpose()?
            .unwrap_or_default();

        let credit_factors: CreditFactors = slf
            .credit_factors_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j)
                    .map_err(|e| PyValueError::new_err(format!("Invalid credit_factors JSON: {e}")))
            })
            .transpose()?
            .unwrap_or_default();

        let deal_metadata: DealMetadata = slf
            .deal_metadata_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j)
                    .map_err(|e| PyValueError::new_err(format!("Invalid deal_metadata JSON: {e}")))
            })
            .transpose()?
            .unwrap_or_default();

        let behavior_overrides: DealOverrides = slf
            .behavior_overrides_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j).map_err(|e| {
                    PyValueError::new_err(format!("Invalid behavior_overrides JSON: {e}"))
                })
            })
            .transpose()?
            .unwrap_or_default();

        let credit_model: CreditModelConfig = slf
            .credit_model_json
            .as_ref()
            .map(|j| {
                serde_json::from_str(j)
                    .map_err(|e| PyValueError::new_err(format!("Invalid credit_model JSON: {e}")))
            })
            .transpose()?
            .unwrap_or_default();

        let deal = StructuredCredit {
            id: slf.instrument_id.clone(),
            deal_type,
            pool,
            tranches,
            closing_date,
            first_payment_date,
            reinvestment_end_date: slf.reinvestment_end_date,
            maturity,
            frequency: slf.frequency,
            payment_calendar_id: None,
            payment_bdc: None,
            discount_curve_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
            credit_model,
            market_conditions,
            credit_factors,
            deal_metadata,
            behavior_overrides,
            default_assumptions,
            hedge_swaps: Vec::new(),
            cleanup_call_pct: slf.cleanup_call_pct,
        };

        Ok(PyStructuredCredit::new(deal))
    }

    fn __repr__(&self) -> String {
        format!(
            "StructuredCreditBuilder(id='{}')",
            self.instrument_id.as_str()
        )
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDealType>()?;
    module.add_class::<PyTrancheSeniority>()?;
    module.add_class::<PyStructuredCredit>()?;
    module.add_class::<PyStructuredCreditBuilder>()?;

    let waterfall_exports = waterfall::register(py, module)?;

    let mut exports = vec![
        "DealType",
        "TrancheSeniority",
        "StructuredCredit",
        "StructuredCreditBuilder",
    ];
    exports.extend(waterfall_exports.iter().copied());

    Ok(exports)
}
