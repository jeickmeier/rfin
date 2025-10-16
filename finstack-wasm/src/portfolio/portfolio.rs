//! Portfolio struct and operations for WASM.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::portfolio::types::{JsEntity, JsPosition};
use finstack_portfolio::Portfolio;
use js_sys::{Array, Object};
use wasm_bindgen::prelude::*;

/// A portfolio of positions across multiple entities.
///
/// The portfolio holds a flat list of positions, each referencing an entity and instrument.
/// Positions can be grouped and aggregated by entity or by arbitrary attributes (tags).
///
/// # Examples
///
/// ```javascript
/// const portfolio = new Portfolio("FUND_A", Currency.USD, new FsDate(2024, 1, 1));
/// portfolio.entities["ACME"] = new Entity("ACME");
/// console.log(portfolio.positions.length);  // 0
/// ```
#[wasm_bindgen]
pub struct JsPortfolio {
    pub(crate) inner: Portfolio,
}

#[wasm_bindgen]
impl JsPortfolio {
    /// Create a new empty portfolio.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique portfolio identifier
    /// * `base_ccy` - Reporting currency
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// New Portfolio instance
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, base_ccy: JsCurrency, as_of: &FsDate) -> Result<JsPortfolio, JsValue> {
        Ok(JsPortfolio {
            inner: Portfolio::new(id, base_ccy.inner(), as_of.inner()),
        })
    }

    /// Get the portfolio identifier.
    ///
    /// # Returns
    ///
    /// Portfolio ID as string
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the portfolio name.
    ///
    /// # Returns
    ///
    /// Portfolio name if set, undefined otherwise
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Set the portfolio name.
    ///
    /// # Arguments
    ///
    /// * `name` - Portfolio name
    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: Option<String>) {
        self.inner.name = name;
    }

    /// Get the base currency.
    ///
    /// # Returns
    ///
    /// Base currency
    #[wasm_bindgen(getter, js_name = baseCcy)]
    pub fn base_ccy(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_ccy)
    }

    /// Get the valuation date.
    ///
    /// # Returns
    ///
    /// Valuation date
    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> FsDate {
        FsDate::from_core(self.inner.as_of)
    }

    /// Get portfolio entities as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping entity IDs to Entity instances
    #[wasm_bindgen(getter)]
    pub fn entities(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        for (id, entity) in &self.inner.entities {
            let js_entity = JsEntity::from_inner(entity.clone());
            js_sys::Reflect::set(&obj, &JsValue::from_str(id), &JsValue::from(js_entity))?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get portfolio positions as an array.
    ///
    /// # Returns
    ///
    /// Array of Position instances
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> Array {
        let arr = Array::new();
        for position in &self.inner.positions {
            let js_position = JsPosition::from_inner(position.clone());
            arr.push(&JsValue::from(js_position));
        }
        arr
    }

    /// Get portfolio tags as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object with tag key-value pairs
    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.tags)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize tags: {}", e)))
    }

    /// Get a position by identifier.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier of the position to locate
    ///
    /// # Returns
    ///
    /// The position if found, undefined otherwise
    #[wasm_bindgen(js_name = getPosition)]
    pub fn get_position(&self, position_id: &str) -> Option<JsPosition> {
        self.inner
            .get_position(position_id)
            .map(|p| JsPosition::from_inner(p.clone()))
    }

    /// Get all positions for a given entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Entity identifier used for filtering
    ///
    /// # Returns
    ///
    /// Array of positions for the entity
    #[wasm_bindgen(js_name = positionsForEntity)]
    pub fn positions_for_entity(&self, entity_id: &str) -> Array {
        let arr = Array::new();
        for position in self.inner.positions_for_entity(&entity_id.to_string()) {
            let js_position = JsPosition::from_inner(position.clone());
            arr.push(&JsValue::from(js_position));
        }
        arr
    }

    /// Get all positions with a specific tag value.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key to filter by
    /// * `value` - Tag value to match
    ///
    /// # Returns
    ///
    /// Array of positions with matching tag
    #[wasm_bindgen(js_name = positionsWithTag)]
    pub fn positions_with_tag(&self, key: &str, value: &str) -> Array {
        let arr = Array::new();
        for position in self.inner.positions_with_tag(key, value) {
            let js_position = JsPosition::from_inner(position.clone());
            arr.push(&JsValue::from(js_position));
        }
        arr
    }

    /// Validate the portfolio structure and references.
    ///
    /// Checks that all positions reference valid entities and that structural
    /// invariants are maintained.
    ///
    /// # Throws
    ///
    /// Error if validation fails
    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    ///
    /// Note: Positions cannot be serialized/deserialized directly due to instrument trait objects.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// Portfolio instance (without positions)
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPortfolio, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPortfolio { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize Portfolio: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// Note: Positions are excluded from serialization.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize Portfolio: {}", e)))
    }
}

impl JsPortfolio {
    pub(crate) fn from_inner(inner: Portfolio) -> Self {
        Self { inner }
    }
}
