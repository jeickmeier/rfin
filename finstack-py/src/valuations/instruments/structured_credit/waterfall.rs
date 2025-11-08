//! Python bindings for structured credit waterfall engine.
//!
//! This module exposes the generalized waterfall engine to Python, including:
//! - WaterfallTier, Recipient, AllocationMode
//! - WaterfallEngine execution
//! - WaterfallResult with tier allocations
//! - Pre-built templates (CLO, CMBS, CRE)

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::{
    AllocationMode as RustAllocationMode, PaymentCalculation as RustPaymentCalculation,
    PaymentRecipient as RustPaymentRecipient, PaymentType as RustPaymentType,
    Recipient as RustRecipient, WaterfallTier as RustWaterfallTier,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

// ============================================================================
// ENUMS
// ============================================================================

/// Allocation mode within a tier.
///
/// Examples:
///     >>> mode = AllocationMode.Sequential
///     >>> mode = AllocationMode.ProRata
#[pyclass(module = "finstack.valuations", name = "AllocationMode", eq, eq_int)]
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

impl From<RustAllocationMode> for PyAllocationMode {
    fn from(value: RustAllocationMode) -> Self {
        match value {
            RustAllocationMode::Sequential => PyAllocationMode::Sequential,
            RustAllocationMode::ProRata => PyAllocationMode::ProRata,
        }
    }
}

/// Payment type classification.
///
/// Examples:
///     >>> ptype = PaymentType.Fee
///     >>> ptype = PaymentType.Interest
#[pyclass(module = "finstack.valuations", name = "PaymentType", eq, eq_int)]
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

impl From<RustPaymentType> for PyPaymentType {
    fn from(value: RustPaymentType) -> Self {
        match value {
            RustPaymentType::Fee => PyPaymentType::Fee,
            RustPaymentType::Interest => PyPaymentType::Interest,
            RustPaymentType::Principal => PyPaymentType::Principal,
            RustPaymentType::Residual => PyPaymentType::Residual,
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
#[pyclass(module = "finstack.valuations", name = "WaterfallTier")]
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
        let rust_recipient: RustPaymentRecipient = serde_json::from_str(recipient_type)
            .map_err(|e| PyValueError::new_err(format!("Invalid recipient_type: {}", e)))?;

        // Parse calculation from JSON
        let rust_calculation: RustPaymentCalculation = serde_json::from_str(calculation)
            .map_err(|e| PyValueError::new_err(format!("Invalid calculation: {}", e)))?;

        let recipient = RustRecipient::new(recipient_id, rust_recipient, rust_calculation);
        inner.recipients.push(recipient);

        Python::with_gil(|py| Py::new(py, Self { inner }))
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
        Python::with_gil(|py| Py::new(py, Self { inner }))
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
        Python::with_gil(|py| Py::new(py, Self { inner }))
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
        Python::with_gil(|py| Py::new(py, Self { inner }))
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
        Python::with_gil(|py| Py::new(py, Self { inner }))
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
        Python::with_gil(|py| Py::new(py, Self { inner }))
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
// TEMPLATE FUNCTIONS
// ============================================================================

/// Create a CLO 2.0 waterfall template.
///
/// Args:
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     dict: Waterfall configuration as JSON-serializable dict
///
/// Examples:
///     >>> waterfall = clo_2_0_template("USD")
///     >>> print(waterfall["tiers"])
#[pyfunction]
#[pyo3(text_signature = "(currency)")]
fn clo_2_0_template(currency: String) -> PyResult<PyObject> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency: {:?}", e)))?;

    let waterfall = finstack_valuations::instruments::structured_credit::clo_2_0_template(curr);

    Python::with_gil(|py| {
        let json = serde_json::to_string(&waterfall)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))?;
        let json_module = PyModule::import(py, "json")?;
        let result = json_module.call_method1("loads", (json,))?;
        Ok(result.into())
    })
}

/// Create a CMBS standard waterfall template.
///
/// Args:
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     dict: Waterfall configuration as JSON-serializable dict
#[pyfunction]
#[pyo3(text_signature = "(currency)")]
fn cmbs_standard_template(currency: String) -> PyResult<PyObject> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency: {:?}", e)))?;

    let waterfall =
        finstack_valuations::instruments::structured_credit::cmbs_standard_template(curr);

    Python::with_gil(|py| {
        let json = serde_json::to_string(&waterfall)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))?;
        let json_module = PyModule::import(py, "json")?;
        let result = json_module.call_method1("loads", (json,))?;
        Ok(result.into())
    })
}

/// Create a CRE operating company waterfall template.
///
/// Args:
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     dict: Waterfall configuration as JSON-serializable dict
#[pyfunction]
#[pyo3(text_signature = "(currency)")]
fn cre_operating_company_template(currency: String) -> PyResult<PyObject> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency: {:?}", e)))?;

    let waterfall =
        finstack_valuations::instruments::structured_credit::cre_operating_company_template(curr);

    Python::with_gil(|py| {
        let json = serde_json::to_string(&waterfall)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))?;
        let json_module = PyModule::import(py, "json")?;
        let result = json_module.call_method1("loads", (json,))?;
        Ok(result.into())
    })
}

/// Get a waterfall template by name.
///
/// Args:
///     template_name: Template name ("clo_2.0", "cmbs_standard", "cre_operating")
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     dict: Waterfall configuration as JSON-serializable dict
///
/// Examples:
///     >>> waterfall = get_waterfall_template("clo_2.0", "USD")
#[pyfunction]
#[pyo3(text_signature = "(template_name, currency)")]
fn get_waterfall_template(template_name: String, currency: String) -> PyResult<PyObject> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency: {:?}", e)))?;

    let waterfall =
        finstack_valuations::instruments::structured_credit::get_template(&template_name, curr)
            .ok_or_else(|| {
                PyValueError::new_err(format!("Template '{}' not found", template_name))
            })?;

    Python::with_gil(|py| {
        let json = serde_json::to_string(&waterfall)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))?;
        let json_module = PyModule::import(py, "json")?;
        let result = json_module.call_method1("loads", (json,))?;
        Ok(result.into())
    })
}

/// List available waterfall templates.
///
/// Returns:
///     list[dict]: List of template metadata with name, description, and deal_type
///
/// Examples:
///     >>> templates = available_waterfall_templates()
///     >>> for t in templates:
///     ...     print(f"{t['name']}: {t['description']}")
#[pyfunction]
#[pyo3(text_signature = "()")]
fn available_waterfall_templates(py: Python<'_>) -> PyResult<Py<pyo3::types::PyList>> {
    let templates = finstack_valuations::instruments::structured_credit::available_templates();

    let result = pyo3::types::PyList::empty(py);
    for t in templates {
        let dict = PyDict::new(py);
        dict.set_item("name", t.name)?;
        dict.set_item("description", t.description)?;
        dict.set_item("deal_type", format!("{:?}", t.deal_type))?;
        result.append(dict)?;
    }
    Ok(result.into())
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
    module.add_function(wrap_pyfunction!(clo_2_0_template, module)?)?;
    module.add_function(wrap_pyfunction!(cmbs_standard_template, module)?)?;
    module.add_function(wrap_pyfunction!(cre_operating_company_template, module)?)?;
    module.add_function(wrap_pyfunction!(get_waterfall_template, module)?)?;
    module.add_function(wrap_pyfunction!(available_waterfall_templates, module)?)?;

    Ok(vec![
        "AllocationMode",
        "PaymentType",
        "WaterfallTier",
        "clo_2_0_template",
        "cmbs_standard_template",
        "cre_operating_company_template",
        "get_waterfall_template",
        "available_waterfall_templates",
    ])
}
