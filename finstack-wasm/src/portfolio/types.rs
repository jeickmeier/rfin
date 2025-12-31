//! Core portfolio types for WASM.

use finstack_portfolio::{Entity, Position, PositionUnit};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// An entity that can hold positions.
///
/// Entities represent companies, funds, or other legal entities that own instruments.
/// For standalone instruments, use the dummy entity via `Entity.dummy()`.
///
/// # Examples
///
/// ```javascript
/// const entity = new Entity("ACME_CORP");
/// entity.withName("Acme Corporation");
/// entity.withTag("sector", "Technology");
/// ```
#[wasm_bindgen]
pub struct JsEntity {
    pub(crate) inner: Entity,
}

#[wasm_bindgen]
impl JsEntity {
    /// Create a new entity with the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique entity identifier
    ///
    /// # Returns
    ///
    /// New Entity instance
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> JsEntity {
        JsEntity {
            inner: Entity::new(id),
        }
    }

    /// Set the entity name.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name
    ///
    /// # Returns
    ///
    /// Self for method chaining (builder pattern)
    #[wasm_bindgen(js_name = withName)]
    pub fn with_name(mut self, name: String) -> JsEntity {
        self.inner = self.inner.with_name(name);
        self
    }

    /// Add a tag to the entity.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key
    /// * `value` - Tag value
    ///
    /// # Returns
    ///
    /// Self for method chaining (builder pattern)
    #[wasm_bindgen(js_name = withTag)]
    pub fn with_tag(mut self, key: String, value: String) -> JsEntity {
        self.inner = self.inner.with_tag(key, value);
        self
    }

    /// Create the dummy entity for standalone instruments.
    ///
    /// # Returns
    ///
    /// Dummy entity with special identifier '_standalone'
    #[wasm_bindgen]
    pub fn dummy() -> JsEntity {
        JsEntity {
            inner: Entity::dummy(),
        }
    }

    /// Get the entity identifier.
    ///
    /// # Returns
    ///
    /// Entity ID as string
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Get the entity name.
    ///
    /// # Returns
    ///
    /// Entity name if set, undefined otherwise
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Get the entity tags as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object with tag key-value pairs
    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.tags)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize tags: {}", e)))
    }

    /// Create from JSON representation.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// Entity instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsEntity, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsEntity { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize Entity: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize Entity: {}", e)))
    }
}

impl JsEntity {
    pub(crate) fn from_inner(inner: Entity) -> Self {
        Self { inner }
    }
}

/// Unit of position measurement.
///
/// Describes how the quantity on a position should be interpreted.
///
/// # Variants
///
/// - `UNITS`: Number of units/shares (for equities, baskets)
/// - `NOTIONAL`: Notional amount, optionally in a specific currency
/// - `FACE_VALUE`: Face value of debt instruments (for bonds, loans)
/// - `PERCENTAGE`: Percentage of ownership
///
/// # Examples
///
/// ```javascript
/// const unit = PositionUnit.UNITS;
/// const notional = PositionUnit.notional();
/// const notionalUsd = PositionUnit.notionalWithCcy("USD");
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct JsPositionUnit {
    inner: PositionUnit,
}

#[wasm_bindgen]
impl JsPositionUnit {
    /// Units position unit (for equities, shares).
    #[wasm_bindgen(getter, js_name = UNITS)]
    pub fn units() -> JsPositionUnit {
        JsPositionUnit {
            inner: PositionUnit::Units,
        }
    }

    /// Face value position unit (for bonds, loans).
    #[wasm_bindgen(getter, js_name = FACE_VALUE)]
    pub fn face_value() -> JsPositionUnit {
        JsPositionUnit {
            inner: PositionUnit::FaceValue,
        }
    }

    /// Percentage position unit.
    #[wasm_bindgen(getter, js_name = PERCENTAGE)]
    pub fn percentage() -> JsPositionUnit {
        JsPositionUnit {
            inner: PositionUnit::Percentage,
        }
    }

    /// Create a notional position unit without specific currency.
    ///
    /// # Returns
    ///
    /// Notional position unit
    #[wasm_bindgen]
    pub fn notional() -> JsPositionUnit {
        JsPositionUnit {
            inner: PositionUnit::Notional(None),
        }
    }

    /// Create a notional position unit with specific currency.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "USD", "EUR")
    ///
    /// # Returns
    ///
    /// Notional position unit with currency
    #[wasm_bindgen(js_name = notionalWithCcy)]
    pub fn notional_with_ccy(currency: &str) -> Result<JsPositionUnit, JsValue> {
        let ccy = currency
            .parse::<finstack_core::currency::Currency>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse currency: {}", e)))?;
        Ok(JsPositionUnit {
            inner: PositionUnit::Notional(Some(ccy)),
        })
    }

    /// Create from JSON representation.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// PositionUnit instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPositionUnit, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPositionUnit { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize PositionUnit: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize PositionUnit: {}", e)))
    }
}

impl JsPositionUnit {
    pub(crate) fn from_inner(inner: PositionUnit) -> Self {
        Self { inner }
    }
}

/// A position in an instrument.
///
/// Represents a holding of a specific quantity of an instrument, belonging to an entity.
/// Positions track the instrument reference, quantity, and metadata for aggregation.
///
/// # Examples
///
/// ```javascript
/// const deposit = new Deposit(...);
/// const position = new Position(
///     "POS_001",
///     "ENTITY_A",
///     "DEP_1M",
///     deposit,
///     1.0,
///     PositionUnit.UNITS
/// );
/// position.isLong();  // true
/// ```
#[wasm_bindgen]
pub struct JsPosition {
    pub(crate) inner: Position,
}

impl JsPosition {
    /// Create a new position from Rust types.
    ///
    /// Note: This constructor cannot be directly called from JavaScript.
    /// Positions must be created through builder methods that handle
    /// instrument type conversions properly.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Unique identifier for the position
    /// * `entity_id` - Owning entity identifier
    /// * `instrument_id` - Instrument identifier (for reference/lookup)
    /// * `instrument` - The actual instrument being held (as Arc<dyn Instrument>)
    /// * `quantity` - Signed quantity (positive=long, negative=short)
    /// * `unit` - Unit of measurement for the quantity
    ///
    /// # Returns
    ///
    /// New Position instance
    pub(crate) fn new_with_instrument(
        position_id: String,
        entity_id: String,
        instrument_id: String,
        instrument: Arc<dyn finstack_valuations::instruments::Instrument>,
        quantity: f64,
        unit: PositionUnit,
    ) -> Result<JsPosition, JsValue> {
        Ok(JsPosition {
            inner: Position::new(
                position_id,
                entity_id,
                instrument_id,
                instrument,
                quantity,
                unit,
            )
            .map_err(|e| JsValue::from_str(&format!("Failed to create position: {}", e)))?,
        })
    }
}

#[wasm_bindgen]
impl JsPosition {
    /// Add a tag to the position.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key
    /// * `value` - Tag value
    ///
    /// # Returns
    ///
    /// Self for method chaining (builder pattern)
    #[wasm_bindgen(js_name = withTag)]
    pub fn with_tag(mut self, key: String, value: String) -> JsPosition {
        self.inner = self.inner.with_tag(key, value);
        self
    }

    /// Check if the position is long (positive quantity).
    ///
    /// # Returns
    ///
    /// True if quantity is positive
    #[wasm_bindgen(js_name = isLong)]
    pub fn is_long(&self) -> bool {
        self.inner.is_long()
    }

    /// Check if the position is short (negative quantity).
    ///
    /// # Returns
    ///
    /// True if quantity is negative
    #[wasm_bindgen(js_name = isShort)]
    pub fn is_short(&self) -> bool {
        self.inner.is_short()
    }

    /// Get the position identifier.
    ///
    /// # Returns
    ///
    /// Position ID as string
    #[wasm_bindgen(getter, js_name = positionId)]
    pub fn position_id(&self) -> String {
        self.inner.position_id.to_string()
    }

    /// Get the entity identifier.
    ///
    /// # Returns
    ///
    /// Entity ID as string
    #[wasm_bindgen(getter, js_name = entityId)]
    pub fn entity_id(&self) -> String {
        self.inner.entity_id.to_string()
    }

    /// Get the instrument identifier.
    ///
    /// # Returns
    ///
    /// Instrument ID as string
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    /// Get the quantity.
    ///
    /// # Returns
    ///
    /// Signed quantity value
    #[wasm_bindgen(getter)]
    pub fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Get the position unit.
    ///
    /// # Returns
    ///
    /// Position unit
    #[wasm_bindgen(getter)]
    pub fn unit(&self) -> JsPositionUnit {
        JsPositionUnit::from_inner(self.inner.unit)
    }

    /// Get position tags as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object with tag key-value pairs
    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.tags)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize tags: {}", e)))
    }
}

impl JsPosition {
    pub(crate) fn from_inner(inner: Position) -> Self {
        Self { inner }
    }
}

// Note: Due to WASM limitations with trait objects, we need specific factory
// functions for each instrument type. The PortfolioBuilder accepts JsPosition instances.

/// Create a position from a Deposit instrument.
///
/// # Arguments
///
/// * `position_id` - Unique identifier for the position
/// * `entity_id` - Entity that owns this position
/// * `deposit` - The deposit instrument
/// * `quantity` - Signed quantity (positive=long, negative=short)
/// * `unit` - Position unit type
///
/// # Examples
///
/// ```javascript
/// const deposit = new finstack.Deposit(...);
/// const position = finstack.createPositionFromDeposit(
///     "POS_001",
///     "ENTITY_A",
///     deposit,
///     1.0,
///     finstack.JsPositionUnit.UNITS
/// );
/// ```
#[wasm_bindgen(js_name = createPositionFromDeposit)]
pub fn js_create_position_from_deposit(
    position_id: String,
    entity_id: String,
    deposit: &crate::valuations::instruments::Deposit,
    quantity: f64,
    unit: JsPositionUnit,
) -> JsPosition {
    use crate::valuations::instruments::InstrumentWrapper;
    use finstack_valuations::instruments::Instrument;
    use std::sync::Arc;

    let instrument_id = deposit.instrument_id();
    let rust_deposit = deposit.inner();
    let arc_inst: Arc<dyn Instrument> = Arc::new(rust_deposit);

    JsPosition::new_with_instrument(
        position_id,
        entity_id,
        instrument_id,
        arc_inst,
        quantity,
        unit.inner,
    )
    .unwrap_or_else(|e| {
        wasm_bindgen::throw_str(&format!("Failed to create position: {:?}", e));
    })
}

/// Create a position from a Bond instrument.
///
/// # Arguments
///
/// * `position_id` - Unique identifier for the position
/// * `entity_id` - Entity that owns this position
/// * `bond` - The bond instrument
/// * `quantity` - Signed quantity (positive=long, negative=short)
/// * `unit` - Position unit type
///
/// # Examples
///
/// ```javascript
/// const bond = finstack.Bond.fixedSemiannual(...);
/// const position = finstack.createPositionFromBond(
///     "POS_001",
///     "ENTITY_A",
///     bond,
///     1.0,
///     finstack.JsPositionUnit.UNITS
/// );
/// ```
#[wasm_bindgen(js_name = createPositionFromBond)]
pub fn js_create_position_from_bond(
    position_id: String,
    entity_id: String,
    bond: &crate::valuations::instruments::Bond,
    quantity: f64,
    unit: JsPositionUnit,
) -> JsPosition {
    use crate::valuations::instruments::InstrumentWrapper;
    use finstack_valuations::instruments::Instrument;
    use std::sync::Arc;

    let instrument_id = bond.instrument_id();
    let rust_bond = bond.inner();
    let arc_inst: Arc<dyn Instrument> = Arc::new(rust_bond);

    JsPosition::new_with_instrument(
        position_id,
        entity_id,
        instrument_id,
        arc_inst,
        quantity,
        unit.inner,
    )
    .unwrap_or_else(|e| {
        wasm_bindgen::throw_str(&format!("Failed to create position: {:?}", e));
    })
}
