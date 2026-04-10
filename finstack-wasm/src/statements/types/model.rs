//! Model type bindings for statements.

use finstack_statements::types::{CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec};
use wasm_bindgen::prelude::*;

/// Debt instrument specification for capital structure.
///
/// Defines a debt instrument (bond, loan, etc.) in the capital structure.
#[wasm_bindgen]
pub struct JsDebtInstrumentSpec {
    pub(crate) inner: DebtInstrumentSpec,
}

#[wasm_bindgen]
impl JsDebtInstrumentSpec {
    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Debt instrument specification
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsDebtInstrumentSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsDebtInstrumentSpec { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize DebtInstrumentSpec: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize DebtInstrumentSpec: {}", e))
        })
    }
}

/// Capital structure specification.
///
/// Defines the debt and equity instruments in an entity's capital structure.
#[wasm_bindgen]
pub struct JsCapitalStructureSpec {
    pub(crate) inner: CapitalStructureSpec,
}

#[wasm_bindgen]
impl JsCapitalStructureSpec {
    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Capital structure specification
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCapitalStructureSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsCapitalStructureSpec { inner })
            .map_err(|e| {
                JsValue::from_str(&format!(
                    "Failed to deserialize CapitalStructureSpec: {}",
                    e
                ))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize CapitalStructureSpec: {}", e))
        })
    }
}

/// Financial model specification.
///
/// Defines a complete financial statement model with periods, nodes,
/// and optional capital structure.
#[wasm_bindgen]
pub struct JsFinancialModelSpec {
    pub(crate) inner: FinancialModelSpec,
}

#[wasm_bindgen]
impl JsFinancialModelSpec {
    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Financial model specification
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFinancialModelSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsFinancialModelSpec { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize FinancialModelSpec: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize FinancialModelSpec: {}", e))
        })
    }

    /// Get model identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get number of periods.
    #[wasm_bindgen(js_name = periodCount)]
    pub fn period_count(&self) -> usize {
        self.inner.periods.len()
    }

    /// Get number of nodes.
    #[wasm_bindgen(js_name = nodeCount)]
    pub fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    /// Check if model has capital structure.
    #[wasm_bindgen(js_name = hasCapitalStructure)]
    pub fn has_capital_structure(&self) -> bool {
        self.inner.capital_structure.is_some()
    }
}

impl JsFinancialModelSpec {
    pub(crate) fn new(inner: FinancialModelSpec) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> &FinancialModelSpec {
        &self.inner
    }
}
