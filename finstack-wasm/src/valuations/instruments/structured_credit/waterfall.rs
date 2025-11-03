//! WASM bindings for structured credit waterfall engine.
//!
//! This module exposes the generalized waterfall engine to TypeScript/JavaScript, including:
//! - WaterfallTier, AllocationMode, PaymentType
//! - Waterfall templates (CLO, CMBS, CRE)
//! - JSON serialization/deserialization

use crate::core::error::js_error;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::structured_credit::{
    AllocationMode as RustAllocationMode, PaymentType as RustPaymentType,
    WaterfallTier as RustWaterfallTier,
};
use wasm_bindgen::prelude::*;

// ============================================================================
// ENUMS
// ============================================================================

/// Allocation mode within a tier.
///
/// Values:
/// - 0: Sequential - Pay recipients in order
/// - 1: ProRata - Distribute proportionally by weight
#[wasm_bindgen(js_name = AllocationMode)]
#[derive(Clone, Copy, Debug)]
pub enum JsAllocationMode {
    Sequential = 0,
    ProRata = 1,
}

impl From<JsAllocationMode> for RustAllocationMode {
    fn from(value: JsAllocationMode) -> Self {
        match value {
            JsAllocationMode::Sequential => RustAllocationMode::Sequential,
            JsAllocationMode::ProRata => RustAllocationMode::ProRata,
        }
    }
}

impl From<RustAllocationMode> for JsAllocationMode {
    fn from(value: RustAllocationMode) -> Self {
        match value {
            RustAllocationMode::Sequential => JsAllocationMode::Sequential,
            RustAllocationMode::ProRata => JsAllocationMode::ProRata,
        }
    }
}

/// Payment type classification.
///
/// Values:
/// - 0: Fee - Fee payment
/// - 1: Interest - Interest payment
/// - 2: Principal - Principal payment
/// - 3: Residual - Residual/equity distribution
#[wasm_bindgen(js_name = PaymentType)]
#[derive(Clone, Copy, Debug)]
pub enum JsPaymentType {
    Fee = 0,
    Interest = 1,
    Principal = 2,
    Residual = 3,
}

impl From<JsPaymentType> for RustPaymentType {
    fn from(value: JsPaymentType) -> Self {
        match value {
            JsPaymentType::Fee => RustPaymentType::Fee,
            JsPaymentType::Interest => RustPaymentType::Interest,
            JsPaymentType::Principal => RustPaymentType::Principal,
            JsPaymentType::Residual => RustPaymentType::Residual,
        }
    }
}

impl From<RustPaymentType> for JsPaymentType {
    fn from(value: RustPaymentType) -> Self {
        match value {
            RustPaymentType::Fee => JsPaymentType::Fee,
            RustPaymentType::Interest => JsPaymentType::Interest,
            RustPaymentType::Principal => JsPaymentType::Principal,
            RustPaymentType::Residual => JsPaymentType::Residual,
        }
    }
}

// ============================================================================
// WATERFALL TIER
// ============================================================================

/// Waterfall tier with multiple recipients (JSON-based API).
///
/// Use JSON to create tiers with recipients and allocation modes.
///
/// Example:
/// ```javascript
/// const tier = WaterfallTier.fromJson(JSON.stringify({
///   id: "fees",
///   priority: 1,
///   recipients: [...],
///   payment_type: "Fee",
///   allocation_mode: "Sequential",
///   divertible: false
/// }));
/// ```
#[wasm_bindgen(js_name = WaterfallTier)]
#[derive(Clone, Debug)]
pub struct JsWaterfallTier(RustWaterfallTier);

#[wasm_bindgen(js_class = WaterfallTier)]
impl JsWaterfallTier {
    /// Create from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsWaterfallTier, JsValue> {
        serde_json::from_str(json_str)
            .map(JsWaterfallTier)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Convert to JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.0).map_err(|e| js_error(e.to_string()))
    }

    /// Get tier ID.
    #[wasm_bindgen(getter, js_name = tierId)]
    pub fn tier_id(&self) -> String {
        self.0.id.clone()
    }

    /// Get priority.
    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> usize {
        self.0.priority
    }

    /// Get number of recipients.
    #[wasm_bindgen(getter, js_name = recipientCount)]
    pub fn recipient_count(&self) -> usize {
        self.0.recipients.len()
    }

    /// Get payment type as number.
    #[wasm_bindgen(getter, js_name = paymentType)]
    pub fn payment_type(&self) -> u8 {
        JsPaymentType::from(self.0.payment_type) as u8
    }

    /// Get allocation mode as number.
    #[wasm_bindgen(getter, js_name = allocationMode)]
    pub fn allocation_mode(&self) -> u8 {
        JsAllocationMode::from(self.0.allocation_mode) as u8
    }

    /// Is tier divertible.
    #[wasm_bindgen(getter)]
    pub fn divertible(&self) -> bool {
        self.0.divertible
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
///     JSON string of waterfall configuration
///
/// Example:
/// ```javascript
/// const waterfall = clo20Template("USD");
/// const config = JSON.parse(waterfall);
/// console.log(config.tiers.length); // 5 tiers
/// ```
#[wasm_bindgen(js_name = clo20Template)]
pub fn clo_2_0_template(currency: &str) -> Result<String, JsValue> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| js_error(format!("Invalid currency: {:?}", e)))?;

    let waterfall = finstack_valuations::instruments::structured_credit::clo_2_0_template(curr);

    serde_json::to_string_pretty(&waterfall).map_err(|e| js_error(e.to_string()))
}

/// Create a CMBS standard waterfall template.
///
/// Args:
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     JSON string of waterfall configuration
#[wasm_bindgen(js_name = cmbsStandardTemplate)]
pub fn cmbs_standard_template(currency: &str) -> Result<String, JsValue> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| js_error(format!("Invalid currency: {:?}", e)))?;

    let waterfall =
        finstack_valuations::instruments::structured_credit::cmbs_standard_template(curr);

    serde_json::to_string_pretty(&waterfall).map_err(|e| js_error(e.to_string()))
}

/// Create a CRE operating company waterfall template.
///
/// Args:
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     JSON string of waterfall configuration
#[wasm_bindgen(js_name = creOperatingCompanyTemplate)]
pub fn cre_operating_company_template(currency: &str) -> Result<String, JsValue> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| js_error(format!("Invalid currency: {:?}", e)))?;

    let waterfall =
        finstack_valuations::instruments::structured_credit::cre_operating_company_template(curr);

    serde_json::to_string_pretty(&waterfall).map_err(|e| js_error(e.to_string()))
}

/// Get a waterfall template by name.
///
/// Args:
///     template_name: Template name ("clo_2.0", "cmbs_standard", "cre_operating")
///     currency: Currency code (e.g., "USD")
///
/// Returns:
///     JSON string of waterfall configuration
///
/// Example:
/// ```javascript
/// const waterfall = getWaterfallTemplate("clo_2.0", "USD");
/// const config = JSON.parse(waterfall);
/// ```
#[wasm_bindgen(js_name = getWaterfallTemplate)]
pub fn get_waterfall_template(template_name: &str, currency: &str) -> Result<String, JsValue> {
    let curr: Currency = currency
        .parse()
        .map_err(|e| js_error(format!("Invalid currency: {:?}", e)))?;

    let waterfall = finstack_valuations::instruments::structured_credit::get_template(
        template_name,
        curr,
    )
    .ok_or_else(|| js_error(format!("Template '{}' not found", template_name)))?;

    serde_json::to_string_pretty(&waterfall).map_err(|e| js_error(e.to_string()))
}

/// List available waterfall templates.
///
/// Returns:
///     JSON string array of template metadata
///
/// Example:
/// ```javascript
/// const templates = JSON.parse(availableWaterfallTemplates());
/// templates.forEach(t => console.log(`${t.name}: ${t.description}`));
/// ```
#[wasm_bindgen(js_name = availableWaterfallTemplates)]
pub fn available_waterfall_templates() -> Result<String, JsValue> {
    let templates = finstack_valuations::instruments::structured_credit::available_templates();

    let metadata: Vec<_> = templates
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "deal_type": format!("{:?}", t.deal_type),
            })
        })
        .collect();

    serde_json::to_string_pretty(&metadata).map_err(|e| js_error(e.to_string()))
}

