//! Portfolio builder for WASM.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::portfolio::portfolio::JsPortfolio;
use crate::portfolio::types::{JsEntity, JsPosition};
use finstack_portfolio::{Entity, PortfolioBuilder};
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for constructing a Portfolio with validation.
///
/// The builder stores all intermediate values needed to construct a portfolio and checks
/// invariants such as base currency, valuation date, and entity references before the
/// final portfolio is produced.
///
/// # Examples
///
/// ```javascript
/// const portfolio = new PortfolioBuilder("FUND_A")
///     .name("Alpha Fund")
///     .baseCcy(Currency.USD)
///     .asOf(new FsDate(2024, 1, 1))
///     .entity(new Entity("ACME"))
///     .build();
/// ```
#[wasm_bindgen]
pub struct JsPortfolioBuilder {
    inner: PortfolioBuilder,
}

#[wasm_bindgen]
impl JsPortfolioBuilder {
    /// Create a new portfolio builder with the given identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the portfolio
    ///
    /// # Returns
    ///
    /// New PortfolioBuilder instance
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> JsPortfolioBuilder {
        JsPortfolioBuilder {
            inner: PortfolioBuilder::new(id),
        }
    }

    /// Set the portfolio's human-readable name.
    ///
    /// # Arguments
    ///
    /// * `name` - Display name stored alongside the portfolio identifier
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn name(mut self, name: String) -> JsPortfolioBuilder {
        self.inner = self.inner.name(name);
        self
    }

    /// Declare the portfolio's reporting currency.
    ///
    /// # Arguments
    ///
    /// * `ccy` - Currency to use when consolidating values and metrics
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen(js_name = baseCcy)]
    pub fn base_ccy(mut self, ccy: JsCurrency) -> JsPortfolioBuilder {
        self.inner = self.inner.base_ccy(ccy.inner());
        self
    }

    /// Assign the valuation date used for pricing and analytics.
    ///
    /// # Arguments
    ///
    /// * `date` - The as-of date for valuation and risk calculation
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen(js_name = asOf)]
    pub fn as_of(mut self, date: &FsDate) -> Result<JsPortfolioBuilder, JsValue> {
        self.inner = self.inner.as_of(date.inner());
        Ok(self)
    }

    /// Register entity with the builder.
    ///
    /// # Arguments
    ///
    /// * `entity` - Entity to register
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn entity(mut self, entity: &JsEntity) -> JsPortfolioBuilder {
        self.inner = self.inner.entity(entity.inner.clone());
        self
    }

    /// Register multiple entities with the builder.
    ///
    /// # Arguments
    ///
    /// * `entities` - Array of entities to register
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn entities(mut self, entities: &Array) -> Result<JsPortfolioBuilder, JsValue> {
        let mut entity_vec = Vec::new();
        for v in entities.iter() {
            let entity_data: Entity = serde_wasm_bindgen::from_value(v)
                .map_err(|e| JsValue::from_str(&format!("Expected Entity in array: {}", e)))?;
            entity_vec.push(entity_data);
        }
        self.inner = self.inner.entities(entity_vec);
        Ok(self)
    }

    /// Add position to the portfolio.
    ///
    /// # Arguments
    ///
    /// * `position` - Position to add
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn position(mut self, position: &JsPosition) -> JsPortfolioBuilder {
        self.inner = self.inner.position(position.inner.clone());
        self
    }

    /// Add multiple positions to the portfolio.
    ///
    /// Note: Positions cannot be easily serialized from JavaScript due to instrument trait objects.
    /// This method is mainly for Rust-side usage.
    ///
    /// # Arguments
    ///
    /// * `positions` - Array of positions to add
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn positions(self, _positions: &Array) -> Result<JsPortfolioBuilder, JsValue> {
        // Positions contain Arc<dyn Instrument> which can't be deserialized
        // This method is mainly for documentation; actual usage should use .position() repeatedly
        Err(JsValue::from_str("Positions cannot be deserialized from JSON. Use .position() method repeatedly instead."))
    }

    /// Add a portfolio-level tag.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key
    /// * `value` - Tag value
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn tag(mut self, key: String, value: String) -> JsPortfolioBuilder {
        self.inner = self.inner.tag(key, value);
        self
    }

    /// Add portfolio-level metadata.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - Metadata value (must be JSON-serializable)
    ///
    /// # Returns
    ///
    /// Self for chaining
    #[wasm_bindgen]
    pub fn meta(mut self, key: String, value: JsValue) -> Result<JsPortfolioBuilder, JsValue> {
        let json_value: serde_json::Value = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse metadata: {}", e)))?;
        self.inner = self.inner.meta(key, json_value);
        Ok(self)
    }

    /// Build and validate the portfolio.
    ///
    /// # Returns
    ///
    /// Validated portfolio instance
    ///
    /// # Throws
    ///
    /// Error if validation fails (missing base_ccy, as_of, or invalid references)
    #[wasm_bindgen]
    pub fn build(self) -> Result<JsPortfolio, JsValue> {
        self.inner
            .build()
            .map(JsPortfolio::from_inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
