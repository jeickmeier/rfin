//! Python bindings for structured credit waterfall engine.
//!
//! This module exposes the generalized waterfall engine to Python, including:
//! - WaterfallTier, Recipient, AllocationMode
//! - Waterfall execution
//! - WaterfallDistribution with tier allocations

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::waterfall::CoverageTestRules as RustCoverageTestRules;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    AllocationMode as RustAllocationMode, PaymentCalculation as RustPaymentCalculation,
    PaymentType as RustPaymentType, Recipient as RustRecipient, RecipientType as RustRecipientType,
    Waterfall as RustWaterfall, WaterfallCoverageTrigger as RustWaterfallCoverageTrigger,
    WaterfallTier as RustWaterfallTier,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;

use super::tranches::PyTrancheStructure;

// ============================================================================
// ENUMS
// ============================================================================

/// Allocation mode within a tier.
///
/// Examples:
///     >>> mode = AllocationMode.Sequential
///     >>> mode = AllocationMode.ProRata
#[pyclass(
    module = "finstack.valuations",
    name = "AllocationMode",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyAllocationMode {
    /// Pay recipients sequentially in order
    Sequential = 0,
    /// Distribute proportionally by weight
    ProRata = 1,
}

impl From<PyAllocationMode> for RustAllocationMode {
    fn from(value: PyAllocationMode) -> Self {
        match value {
            PyAllocationMode::Sequential => RustAllocationMode::Sequential,
            PyAllocationMode::ProRata => RustAllocationMode::ProRata,
        }
    }
}

impl TryFrom<RustAllocationMode> for PyAllocationMode {
    type Error = &'static str;

    fn try_from(value: RustAllocationMode) -> Result<Self, Self::Error> {
        match value {
            RustAllocationMode::Sequential => Ok(PyAllocationMode::Sequential),
            RustAllocationMode::ProRata => Ok(PyAllocationMode::ProRata),
            _ => Err("unknown AllocationMode variant"),
        }
    }
}

/// Payment type classification.
///
/// Examples:
///     >>> ptype = PaymentType.Fee
///     >>> ptype = PaymentType.Interest
#[pyclass(
    module = "finstack.valuations",
    name = "PaymentType",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyPaymentType {
    /// Fee payment
    Fee = 0,
    /// Interest payment
    Interest = 1,
    /// Principal payment
    Principal = 2,
    /// Residual/equity distribution
    Residual = 3,
}

impl From<PyPaymentType> for RustPaymentType {
    fn from(value: PyPaymentType) -> Self {
        match value {
            PyPaymentType::Fee => RustPaymentType::Fee,
            PyPaymentType::Interest => RustPaymentType::Interest,
            PyPaymentType::Principal => RustPaymentType::Principal,
            PyPaymentType::Residual => RustPaymentType::Residual,
        }
    }
}

impl TryFrom<RustPaymentType> for PyPaymentType {
    type Error = &'static str;

    fn try_from(value: RustPaymentType) -> Result<Self, Self::Error> {
        match value {
            RustPaymentType::Fee => Ok(PyPaymentType::Fee),
            RustPaymentType::Interest => Ok(PyPaymentType::Interest),
            RustPaymentType::Principal => Ok(PyPaymentType::Principal),
            RustPaymentType::Residual => Ok(PyPaymentType::Residual),
            _ => Err("unknown PaymentType variant"),
        }
    }
}

// ============================================================================
// WATERFALL TIER
// ============================================================================

/// Waterfall tier with multiple recipients.
///
/// A tier groups related payments with a priority level and allocation mode.
///
/// Args:
///     tier_id: Unique tier identifier
///     priority: Priority order (lower = higher priority)
///     payment_type: Type of payment (Fee, Interest, Principal, Residual)
///
/// Examples:
///     >>> tier = WaterfallTier("fees", 1, PaymentType.Fee)
///     >>> tier.add_recipient(recipient)
///     >>> tier.set_allocation_mode(AllocationMode.Sequential)
#[pyclass(module = "finstack.valuations", name = "WaterfallTier", from_py_object)]
#[derive(Clone)]
pub struct PyWaterfallTier {
    inner: RustWaterfallTier,
}

#[pymethods]
impl PyWaterfallTier {
    #[new]
    #[pyo3(signature = (tier_id, priority, payment_type))]
    fn new(tier_id: String, priority: usize, payment_type: PyPaymentType) -> Self {
        Self {
            inner: RustWaterfallTier::new(tier_id, priority, payment_type.into()),
        }
    }

    /// Add a recipient to this tier.
    ///
    /// Args:
    ///     recipient_id: Unique recipient identifier
    ///     recipient_type: Type of recipient (from JSON)
    ///     calculation: Payment calculation (from JSON)
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, recipient_id, recipient_type, calculation)")]
    fn add_recipient(
        slf: PyRefMut<'_, Self>,
        recipient_id: String,
        recipient_type: &str,
        calculation: &str,
    ) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        // Parse recipient type from JSON
        let rust_recipient: RustRecipientType = serde_json::from_str(recipient_type)
            .map_err(|e| PyValueError::new_err(format!("Invalid recipient_type: {}", e)))?;

        // Parse calculation from JSON
        let rust_calculation: RustPaymentCalculation = serde_json::from_str(calculation)
            .map_err(|e| PyValueError::new_err(format!("Invalid calculation: {}", e)))?;

        let recipient = RustRecipient::new(recipient_id, rust_recipient, rust_calculation);
        inner.recipients.push(recipient);

        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Add a fixed fee recipient helper.
    ///
    /// Args:
    ///     recipient_id: Unique recipient identifier
    ///     provider_name: Service provider name
    ///     amount: Fixed fee amount
    ///     currency: Currency code (e.g., "USD")
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, recipient_id, provider_name, amount, currency)")]
    fn add_fixed_fee(
        slf: PyRefMut<'_, Self>,
        recipient_id: String,
        provider_name: String,
        amount: f64,
        currency: String,
    ) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        let curr: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {:?}", e)))?;
        let recipient =
            RustRecipient::fixed_fee(recipient_id, provider_name, Money::new(amount, curr));
        inner.recipients.push(recipient);
        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Add a tranche interest recipient helper.
    ///
    /// Args:
    ///     recipient_id: Unique recipient identifier
    ///     tranche_id: Tranche identifier
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, recipient_id, tranche_id)")]
    fn add_tranche_interest(
        slf: PyRefMut<'_, Self>,
        recipient_id: String,
        tranche_id: String,
    ) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        let recipient = RustRecipient::tranche_interest(recipient_id, tranche_id);
        inner.recipients.push(recipient);
        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Add a tranche principal recipient helper.
    ///
    /// Args:
    ///     recipient_id: Unique recipient identifier
    ///     tranche_id: Tranche identifier
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, recipient_id, tranche_id)")]
    fn add_tranche_principal(
        slf: PyRefMut<'_, Self>,
        recipient_id: String,
        tranche_id: String,
    ) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        let recipient = RustRecipient::tranche_principal(recipient_id, tranche_id, None);
        inner.recipients.push(recipient);
        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Set allocation mode for this tier.
    ///
    /// Args:
    ///     mode: AllocationMode (Sequential or ProRata)
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, mode)")]
    fn set_allocation_mode(slf: PyRefMut<'_, Self>, mode: PyAllocationMode) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        inner.allocation_mode = mode.into();
        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Mark tier as divertible.
    ///
    /// Args:
    ///     divertible: Whether this tier can be diverted
    ///
    /// Returns:
    ///     WaterfallTier: Self for method chaining
    #[pyo3(text_signature = "(self, divertible)")]
    fn set_divertible(slf: PyRefMut<'_, Self>, divertible: bool) -> PyResult<Py<Self>> {
        let mut inner = slf.inner.clone();
        inner.divertible = divertible;
        Python::attach(|py| Py::new(py, Self { inner }))
    }

    /// Get tier ID.
    #[getter]
    fn tier_id(&self) -> &str {
        &self.inner.id
    }

    /// Get priority.
    #[getter]
    fn priority(&self) -> usize {
        self.inner.priority
    }

    /// Get number of recipients.
    #[getter]
    fn recipient_count(&self) -> usize {
        self.inner.recipients.len()
    }

    /// Get payment type.
    #[getter]
    fn payment_type(&self) -> PyResult<PyPaymentType> {
        PyPaymentType::try_from(self.inner.payment_type)
            .map_err(crate::errors::InternalError::new_err)
    }

    /// Get allocation mode.
    #[getter]
    fn allocation_mode(&self) -> PyResult<PyAllocationMode> {
        PyAllocationMode::try_from(self.inner.allocation_mode)
            .map_err(crate::errors::InternalError::new_err)
    }

    /// Whether this tier can be diverted.
    #[getter]
    fn divertible(&self) -> bool {
        self.inner.divertible
    }

    fn __repr__(&self) -> String {
        format!(
            "WaterfallTier(id='{}', priority={}, payment_type={:?}, recipients={})",
            self.inner.id,
            self.inner.priority,
            self.inner.payment_type,
            self.inner.recipients.len()
        )
    }
}

// ============================================================================
// RECIPIENT
// ============================================================================

/// Individual payment recipient within a waterfall tier.
///
/// Args:
///     id: Unique recipient identifier
///     recipient_type_json: Recipient type as JSON string
///     calculation_json: Payment calculation as JSON string
///
/// Examples:
///     >>> r = Recipient.tranche_interest("a_int", "tranche_a")
///     >>> r = Recipient.fixed_fee("trustee", "TrustCo", 25_000.0, "USD")
#[pyclass(
    module = "finstack.valuations",
    name = "Recipient",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRecipient {
    pub(crate) inner: RustRecipient,
}

#[pymethods]
impl PyRecipient {
    #[new]
    #[pyo3(signature = (id, recipient_type_json, calculation_json))]
    fn new(id: String, recipient_type_json: &str, calculation_json: &str) -> PyResult<Self> {
        let recipient_type: finstack_valuations::instruments::fixed_income::structured_credit::RecipientType =
            serde_json::from_str(recipient_type_json)
                .map_err(|e| PyValueError::new_err(format!("Invalid recipient_type JSON: {e}")))?;
        let calculation: finstack_valuations::instruments::fixed_income::structured_credit::PaymentCalculation =
            serde_json::from_str(calculation_json)
                .map_err(|e| PyValueError::new_err(format!("Invalid calculation JSON: {e}")))?;
        Ok(Self {
            inner: RustRecipient::new(id, recipient_type, calculation),
        })
    }

    /// Return a new recipient with the given pro-rata weight.
    #[pyo3(text_signature = "($self, weight)")]
    fn with_weight(&self, weight: f64) -> Self {
        Self {
            inner: self.inner.clone().with_weight(weight),
        }
    }

    /// Create a fixed fee recipient.
    #[classmethod]
    #[pyo3(text_signature = "(cls, id, provider_name, amount, currency)")]
    fn fixed_fee(
        _cls: &Bound<'_, PyType>,
        id: String,
        provider_name: String,
        amount: f64,
        currency: &str,
    ) -> PyResult<Self> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        Ok(Self {
            inner: RustRecipient::fixed_fee(id, provider_name, Money::new(amount, ccy)),
        })
    }

    /// Create a tranche interest recipient.
    #[classmethod]
    #[pyo3(text_signature = "(cls, id, tranche_id)")]
    fn tranche_interest(_cls: &Bound<'_, PyType>, id: String, tranche_id: String) -> Self {
        Self {
            inner: RustRecipient::tranche_interest(id, tranche_id),
        }
    }

    /// Create a tranche principal recipient.
    #[classmethod]
    #[pyo3(signature = (id, tranche_id, target_balance_amount=None, currency="USD"))]
    fn tranche_principal(
        _cls: &Bound<'_, PyType>,
        id: String,
        tranche_id: String,
        target_balance_amount: Option<f64>,
        currency: &str,
    ) -> Self {
        let tb = target_balance_amount.map(|amt| {
            let ccy: Currency = currency.parse().unwrap_or(Currency::USD);
            Money::new(amt, ccy)
        });
        Self {
            inner: RustRecipient::tranche_principal(id, tranche_id, tb),
        }
    }

    /// Unique recipient identifier.
    #[getter]
    fn recipient_id(&self) -> &str {
        &self.inner.id
    }

    /// Pro-rata weight (None = equal weight).
    #[getter]
    fn weight(&self) -> Option<f64> {
        self.inner.weight
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

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustRecipient = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize Recipient: {e}")))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "Recipient(id='{}', type={:?})",
            self.inner.id, self.inner.recipient_type
        )
    }
}

// ============================================================================
// WATERFALL COVERAGE TRIGGER
// ============================================================================

/// Waterfall-level coverage trigger for OC/IC diversion.
///
/// Args:
///     tranche_id: Tranche where the test applies
///     oc_trigger: OC trigger level (e.g. 1.15 for 115%)
///     ic_trigger: IC trigger level (e.g. 1.10 for 110%)
///
/// Examples:
///     >>> trigger = WaterfallCoverageTrigger("tranche_a", oc_trigger=1.20, ic_trigger=1.10)
#[pyclass(
    module = "finstack.valuations",
    name = "WaterfallCoverageTrigger",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyWaterfallCoverageTrigger {
    pub(crate) inner: RustWaterfallCoverageTrigger,
}

#[pymethods]
impl PyWaterfallCoverageTrigger {
    #[new]
    #[pyo3(signature = (tranche_id, oc_trigger=None, ic_trigger=None))]
    fn new(tranche_id: String, oc_trigger: Option<f64>, ic_trigger: Option<f64>) -> Self {
        Self {
            inner: RustWaterfallCoverageTrigger {
                tranche_id,
                oc_trigger,
                ic_trigger,
            },
        }
    }

    /// Tranche where the test applies.
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    /// OC trigger level.
    #[getter]
    fn oc_trigger(&self) -> Option<f64> {
        self.inner.oc_trigger
    }

    /// IC trigger level.
    #[getter]
    fn ic_trigger(&self) -> Option<f64> {
        self.inner.ic_trigger
    }

    fn __repr__(&self) -> String {
        format!(
            "WaterfallCoverageTrigger(tranche_id='{}', oc={:?}, ic={:?})",
            self.inner.tranche_id, self.inner.oc_trigger, self.inner.ic_trigger,
        )
    }
}

// ============================================================================
// COVERAGE TEST RULES
// ============================================================================

/// Coverage test rules including rating haircuts and par-value thresholds.
///
/// Args:
///     par_value_threshold: Optional par-value threshold ratio
///
/// Examples:
///     >>> rules = CoverageTestRules()
///     >>> rules = CoverageTestRules(par_value_threshold=1.05)
///     >>> rules.is_empty()
#[pyclass(
    module = "finstack.valuations",
    name = "CoverageTestRules",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCoverageTestRules {
    pub(crate) inner: RustCoverageTestRules,
}

#[pymethods]
impl PyCoverageTestRules {
    #[new]
    #[pyo3(signature = (par_value_threshold=None))]
    fn new(par_value_threshold: Option<f64>) -> Self {
        let mut rules = RustCoverageTestRules::empty();
        rules.par_value_threshold = par_value_threshold;
        Self { inner: rules }
    }

    /// Create empty coverage test rules (no haircuts, no threshold).
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustCoverageTestRules::empty(),
        }
    }

    /// Whether no rules are configured.
    #[pyo3(text_signature = "($self)")]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
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

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustCoverageTestRules = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize CoverageTestRules: {e}"))
        })?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "CoverageTestRules(par_value_threshold={:?}, haircuts={})",
            self.inner.par_value_threshold,
            self.inner.haircuts.len(),
        )
    }
}

// ============================================================================
// WATERFALL
// ============================================================================

/// Main waterfall engine with tier-based payment distribution.
///
/// Args:
///     currency: Base currency code (default "USD")
///
/// Examples:
///     >>> wf = Waterfall("USD")
///     >>> wf = wf.add_tier(tier)
///     >>> wf = Waterfall.standard_sequential("USD", tranches, fee_recipients)
#[pyclass(
    module = "finstack.valuations",
    name = "Waterfall",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyWaterfall {
    pub(crate) inner: RustWaterfall,
}

#[pymethods]
impl PyWaterfall {
    #[new]
    #[pyo3(signature = (currency="USD"))]
    fn new(currency: &str) -> PyResult<Self> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        Ok(Self {
            inner: RustWaterfall::new(ccy),
        })
    }

    /// Create a WaterfallBuilder for fluent construction.
    #[classmethod]
    #[pyo3(signature = (currency="USD"))]
    fn builder(_cls: &Bound<'_, PyType>, currency: &str) -> PyResult<PyWaterfallBuilder> {
        PyWaterfallBuilder::new(currency)
    }

    /// Return a new waterfall with the tier added.
    #[pyo3(text_signature = "($self, tier)")]
    fn add_tier(&self, tier: PyWaterfallTier) -> Self {
        Self {
            inner: self.inner.clone().add_tier(tier.inner),
        }
    }

    /// Return a new waterfall with the coverage trigger added.
    #[pyo3(text_signature = "($self, trigger)")]
    fn add_coverage_trigger(&self, trigger: PyWaterfallCoverageTrigger) -> Self {
        Self {
            inner: self.inner.clone().add_coverage_trigger(trigger.inner),
        }
    }

    /// Return a new waterfall with coverage test rules attached.
    #[pyo3(text_signature = "($self, rules)")]
    fn with_coverage_rules(&self, rules: PyCoverageTestRules) -> Self {
        Self {
            inner: self.inner.clone().with_coverage_rules(rules.inner),
        }
    }

    /// Create a standard sequential waterfall for a tranche structure.
    ///
    /// Builds a typical CLO/ABS waterfall with fees, interest,
    /// principal (divertible), and equity tiers.
    #[classmethod]
    #[pyo3(text_signature = "(cls, currency, tranches, fee_recipients)")]
    fn standard_sequential(
        _cls: &Bound<'_, PyType>,
        currency: &str,
        tranches: &PyTrancheStructure,
        fee_recipients: Vec<PyRecipient>,
    ) -> PyResult<Self> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        let rust_recipients: Vec<RustRecipient> =
            fee_recipients.into_iter().map(|r| r.inner).collect();
        Ok(Self {
            inner: RustWaterfall::standard_sequential(ccy, &tranches.inner, rust_recipients),
        })
    }

    /// Number of tiers in the waterfall.
    #[getter]
    fn tier_count(&self) -> usize {
        self.inner.tiers.len()
    }

    /// Base currency code.
    #[getter]
    fn base_currency(&self) -> String {
        format!("{}", self.inner.base_currency)
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

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustWaterfall = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize Waterfall: {e}")))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "Waterfall(tiers={}, currency={})",
            self.inner.tiers.len(),
            self.inner.base_currency,
        )
    }
}

// ============================================================================
// WATERFALL BUILDER
// ============================================================================

/// Fluent builder for constructing a ``Waterfall``.
///
/// Args:
///     currency: Base currency code (default "USD")
///
/// Examples:
///     >>> wf = (Waterfall.builder("USD")
///     ...     .add_tier(fees_tier)
///     ...     .add_tier(interest_tier)
///     ...     .build())
#[pyclass(
    module = "finstack.valuations",
    name = "WaterfallBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyWaterfallBuilder {
    engine: RustWaterfall,
    next_priority: usize,
}

impl PyWaterfallBuilder {
    fn new(currency: &str) -> PyResult<Self> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        Ok(Self {
            engine: RustWaterfall::new(ccy),
            next_priority: 1,
        })
    }
}

#[pymethods]
impl PyWaterfallBuilder {
    #[new]
    #[pyo3(signature = (currency="USD"))]
    fn new_py(currency: &str) -> PyResult<Self> {
        Self::new(currency)
    }

    /// Add a tier to the waterfall.
    #[pyo3(text_signature = "($self, tier)")]
    fn add_tier(mut slf: PyRefMut<'_, Self>, tier: PyWaterfallTier) -> PyRefMut<'_, Self> {
        let mut inner_tier = tier.inner;
        if inner_tier.priority == 0 {
            inner_tier.priority = slf.next_priority;
            slf.next_priority += 1;
        }
        slf.engine.tiers.push(inner_tier);
        slf.engine.tiers.sort_by_key(|t| t.priority);
        slf
    }

    /// Add a coverage trigger.
    #[pyo3(text_signature = "($self, trigger)")]
    fn add_coverage_trigger(
        mut slf: PyRefMut<'_, Self>,
        trigger: PyWaterfallCoverageTrigger,
    ) -> PyRefMut<'_, Self> {
        slf.engine.coverage_triggers.push(trigger.inner);
        slf
    }

    /// Attach coverage test rules.
    #[pyo3(text_signature = "($self, rules)")]
    fn coverage_rules(
        mut slf: PyRefMut<'_, Self>,
        rules: PyCoverageTestRules,
    ) -> PyRefMut<'_, Self> {
        slf.engine.coverage_rules = Some(rules.inner);
        slf
    }

    /// Build the ``Waterfall`` from accumulated configuration.
    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyWaterfall {
        PyWaterfall {
            inner: slf.engine.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "WaterfallBuilder(tiers={}, currency={})",
            self.engine.tiers.len(),
            self.engine.base_currency,
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
    module.add_class::<PyAllocationMode>()?;
    module.add_class::<PyPaymentType>()?;
    module.add_class::<PyWaterfallTier>()?;
    module.add_class::<PyRecipient>()?;
    module.add_class::<PyWaterfallCoverageTrigger>()?;
    module.add_class::<PyCoverageTestRules>()?;
    module.add_class::<PyWaterfall>()?;
    module.add_class::<PyWaterfallBuilder>()?;

    Ok(vec![
        "AllocationMode",
        "PaymentType",
        "WaterfallTier",
        "Recipient",
        "WaterfallCoverageTrigger",
        "CoverageTestRules",
        "Waterfall",
        "WaterfallBuilder",
    ])
}
