//! WASM bindings for structured credit waterfall engine.
//!
//! This module exposes the generalized waterfall engine to TypeScript/JavaScript, including:
//! - WaterfallTier, AllocationMode, PaymentType
//! - JSON serialization/deserialization

use crate::core::error::js_error;
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
pub struct JsWaterfallTier {
    pub(crate) inner: RustWaterfallTier,
}

#[wasm_bindgen(js_class = WaterfallTier)]
impl JsWaterfallTier {
    /// Create from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsWaterfallTier, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsWaterfallTier { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Convert to JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Get tier ID.
    #[wasm_bindgen(getter, js_name = tierId)]
    pub fn tier_id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get priority.
    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> usize {
        self.inner.priority
    }

    /// Get number of recipients.
    #[wasm_bindgen(getter, js_name = recipientCount)]
    pub fn recipient_count(&self) -> usize {
        self.inner.recipients.len()
    }

    /// Get payment type as number.
    #[wasm_bindgen(getter, js_name = paymentType)]
    pub fn payment_type(&self) -> u8 {
        JsPaymentType::from(self.inner.payment_type) as u8
    }

    /// Get allocation mode as number.
    #[wasm_bindgen(getter, js_name = allocationMode)]
    pub fn allocation_mode(&self) -> u8 {
        JsAllocationMode::from(self.inner.allocation_mode) as u8
    }

    /// Is tier divertible.
    #[wasm_bindgen(getter)]
    pub fn divertible(&self) -> bool {
        self.inner.divertible
    }
}
