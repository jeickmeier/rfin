//! Node type bindings for statements.

use finstack_statements::types::{NodeSpec, NodeType};
use wasm_bindgen::prelude::*;

/// Node type enumeration.
///
/// Defines the three types of statement nodes.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsNodeType {
    inner: NodeType,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsNodeType {
    /// Value node - contains only explicit data (actuals or assumptions).
    #[wasm_bindgen(getter)]
    #[allow(non_snake_case)]
    pub fn VALUE() -> JsNodeType {
        JsNodeType {
            inner: NodeType::Value,
        }
    }

    /// Calculated node - derives values from formulas only.
    #[wasm_bindgen(getter)]
    pub fn CALCULATED() -> JsNodeType {
        JsNodeType {
            inner: NodeType::Calculated,
        }
    }

    /// Mixed node - combines values, forecasts, and formulas with precedence rules.
    #[wasm_bindgen(getter)]
    pub fn MIXED() -> JsNodeType {
        JsNodeType {
            inner: NodeType::Mixed,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<NodeType> for JsNodeType {
    fn from(inner: NodeType) -> Self {
        Self { inner }
    }
}

impl From<JsNodeType> for NodeType {
    fn from(js: JsNodeType) -> Self {
        js.inner
    }
}

/// Statement node specification.
///
/// Defines a single node in a financial model with its values, formulas,
/// forecasts, and metadata.
#[wasm_bindgen]
pub struct JsNodeSpec {
    pub(crate) inner: NodeSpec,
}

#[wasm_bindgen]
impl JsNodeSpec {
    /// Create a new node specification from JSON.
    ///
    /// # Arguments
    /// * `value` - JavaScript object representing the node spec
    ///
    /// # Returns
    /// Node specification instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsNodeSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsNodeSpec { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize NodeSpec: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize NodeSpec: {}", e)))
    }

    /// Get node identifier.
    #[wasm_bindgen(getter, js_name = nodeId)]
    pub fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    /// Get node type.
    #[wasm_bindgen(getter, js_name = nodeType)]
    pub fn node_type(&self) -> JsNodeType {
        JsNodeType::from(self.inner.node_type)
    }

    /// Get human-readable name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Get formula text.
    #[wasm_bindgen(getter, js_name = formulaText)]
    pub fn formula_text(&self) -> Option<String> {
        self.inner.formula_text.clone()
    }

    /// Get where clause text.
    #[wasm_bindgen(getter, js_name = whereText)]
    pub fn where_text(&self) -> Option<String> {
        self.inner.where_text.clone()
    }
}

impl JsNodeSpec {
    #[allow(dead_code)]
    pub(crate) fn new(inner: NodeSpec) -> Self {
        Self { inner }
    }
}
