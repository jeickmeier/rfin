//! Position types for holding instruments in a portfolio.

use crate::book::BookId;
use crate::error::{Error, Result};
use crate::types::{AttributeValue, EntityId, PositionId};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::{DynInstrument, InstrumentJson};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unit of position measurement.
///
/// The unit describes how the `quantity` on a [`Position`] should be interpreted.
/// Callers should treat it as part of the valuation contract, not display-only
/// metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PositionUnit {
    /// Number of units/shares (for equities, baskets)
    Units,

    /// Notional amount, optionally in a specific currency (for derivatives, FX)
    Notional(Option<Currency>),

    /// Face value of debt instruments (for bonds, loans)
    FaceValue,

    /// Percentage of ownership where the value represents percentage points.
    ///
    /// For example, 50.0 means 50%, not 0.50. The scaling logic always divides
    /// by 100 to convert to a decimal multiplier.
    Percentage,
}

/// A position in an instrument.
///
/// Represents a holding of a specific quantity of an instrument,
/// belonging to an entity. Positions track the instrument reference,
/// quantity, and metadata for aggregation and analysis.
#[derive(Clone)]
pub struct Position {
    /// Unique identifier for this position
    pub position_id: PositionId,

    /// Entity that owns this position
    pub entity_id: EntityId,

    /// Instrument identifier (for reference/lookup)
    pub instrument_id: String,

    /// The actual instrument being held
    pub instrument: Arc<DynInstrument>,

    /// Signed quantity (positive=long, negative=short)
    pub quantity: f64,

    /// Unit of measurement for the quantity
    pub unit: PositionUnit,

    /// Optional book identifier for hierarchical organization
    pub book_id: Option<BookId>,

    /// Position-level attributes for grouping, filtering, and constraints
    pub attributes: IndexMap<String, AttributeValue>,

    /// Additional metadata
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Serializable position specification (without `Arc<DynInstrument>`).
///
/// This struct allows positions to be serialized and deserialized by storing
/// the instrument definition as JSON rather than a trait object.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionSpec {
    /// Position identifier
    pub position_id: PositionId,
    /// Entity identifier
    pub entity_id: EntityId,
    /// Instrument identifier (for reference/lookup)
    pub instrument_id: String,
    /// Instrument definition for full serialization (optional)
    ///
    /// If `None`, the position can still be serialized but cannot be
    /// reconstructed without an external instrument registry.
    pub instrument_spec: Option<InstrumentJson>,
    /// Signed quantity
    pub quantity: f64,
    /// Unit of measurement
    pub unit: PositionUnit,
    /// Optional book identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_id: Option<BookId>,
    /// Position-level attributes for grouping, filtering, and constraints
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<String, AttributeValue>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl Position {
    /// Create a new position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Unique identifier for the position.
    /// * `entity_id` - Owning entity identifier.
    /// * `instrument_id` - Identifier of the underlying instrument.
    /// * `instrument` - Shared pointer to the instrument implementation.
    /// * `quantity` - Signed quantity of the instrument (must be finite).
    /// * `unit` - Interpretation of the quantity.
    ///
    /// # Returns
    ///
    /// A fully constructed position with empty tags and metadata.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] if `quantity` is NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::instruments::rates::deposit::Deposit;
    /// use std::sync::Arc;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_portfolio::Result<()> {
    /// let instrument = Deposit::builder()
    ///     .id("DEP_1M".into())
    ///     .notional(Money::new(1_000_000.0, Currency::USD))
    ///     .start_date(date!(2024-01-01))
    ///     .maturity(date!(2024-02-01))
    ///     .day_count(finstack_core::dates::DayCount::Act360)
    ///     .discount_curve_id("USD".into())
    ///     .build()
    ///     .expect("example deposit should build");
    ///
    /// let position = Position::new(
    ///     "POS_001",
    ///     "ENTITY_A",
    ///     "DEP_1M",
    ///     Arc::new(instrument),
    ///     1.0,
    ///     PositionUnit::Units,
    /// )?;
    ///
    /// assert_eq!(position.instrument_id, "DEP_1M");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        position_id: impl Into<PositionId>,
        entity_id: impl Into<EntityId>,
        instrument_id: impl Into<String>,
        instrument: Arc<DynInstrument>,
        quantity: f64,
        unit: PositionUnit,
    ) -> Result<Self> {
        let pos_id: PositionId = position_id.into();

        // Validate quantity
        if !quantity.is_finite() {
            return Err(Error::invalid_input(format!(
                "Position quantity must be finite, got: {} (position_id: {})",
                quantity, pos_id
            )));
        }

        if quantity.abs() > 1e15 {
            tracing::warn!(
                position_id = %pos_id,
                quantity,
                "Unusually large position quantity"
            );
        }

        // Warn if percentage exceeds 100 (might indicate confusion with decimal form)
        if matches!(unit, PositionUnit::Percentage) && quantity > 100.0 {
            tracing::warn!(
                position_id = %pos_id,
                quantity,
                "Percentage quantity exceeds 100 - did you mean {}%?",
                quantity
            );
        }

        Ok(Self {
            position_id: pos_id,
            entity_id: entity_id.into(),
            instrument_id: instrument_id.into(),
            instrument,
            quantity,
            unit,
            book_id: None,
            attributes: IndexMap::new(),
            meta: IndexMap::new(),
        })
    }

    /// Assign this position to a book.
    ///
    /// # Arguments
    ///
    /// * `book_id` - Book identifier for hierarchical organization.
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    pub fn with_book(mut self, book_id: impl Into<BookId>) -> Self {
        self.book_id = Some(book_id.into());
        self
    }

    /// Add a text attribute to the position.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute key.
    /// * `value` - Text attribute value.
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    pub fn with_text_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes
            .insert(key.into(), AttributeValue::Text(value.into()));
        self
    }

    /// Add a numeric attribute to the position.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute key.
    /// * `value` - Numeric attribute value.
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    pub fn with_numeric_attribute(mut self, key: impl Into<String>, value: f64) -> Self {
        self.attributes
            .insert(key.into(), AttributeValue::Number(value));
        self
    }

    /// Add an attribute to the position.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute key.
    /// * `value` - Attribute value (text or numeric).
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    pub fn with_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add multiple text attributes at once.
    ///
    /// # Arguments
    ///
    /// * `attrs` - Iterator of (key, value) string pairs.
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::instruments::rates::deposit::Deposit;
    /// use std::sync::Arc;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_portfolio::Result<()> {
    /// let as_of = date!(2024-01-01);
    ///
    /// // Create an instrument to attach to the position (example: a simple deposit)
    /// let deposit = Deposit::builder()
    ///     .id("DEP_1M".into())
    ///     .notional(Money::new(1_000_000.0, Currency::USD))
    ///     .start_date(as_of)
    ///     .maturity(date!(2024-02-01))
    ///     .day_count(finstack_core::dates::DayCount::Act360)
    ///     .discount_curve_id("USD".into())
    ///     .build()
    ///     .expect("deposit builder should succeed");
    ///
    /// let position = Position::new(
    ///     "POS_001",
    ///     "ACME_CORP",
    ///     "DEP_1M",
    ///     Arc::new(deposit),
    ///     1.0,
    ///     PositionUnit::Units,
    /// )?
    /// .with_text_attributes([("sector", "Technology"), ("region", "US")]);
    ///
    /// assert_eq!(position.attributes.get("sector"), Some(&finstack_portfolio::AttributeValue::Text("Technology".to_string())));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_text_attributes<K, V, I>(mut self, attrs: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in attrs {
            self.attributes
                .insert(k.into(), AttributeValue::Text(v.into()));
        }
        self
    }

    /// Add metadata.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key.
    /// * `value` - Arbitrary JSON value.
    ///
    /// # Returns
    ///
    /// The updated position for fluent chaining.
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }

    /// Check if this position is long (positive quantity).
    ///
    /// # Returns
    ///
    /// `true` when the stored quantity is strictly greater than zero.
    pub fn is_long(&self) -> bool {
        self.quantity > 0.0
    }

    /// Check if this position is short (negative quantity).
    ///
    /// # Returns
    ///
    /// `true` when the stored quantity is strictly less than zero.
    pub fn is_short(&self) -> bool {
        self.quantity < 0.0
    }

    /// Scale a monetary value by this position's quantity, respecting the unit type.
    ///
    /// This function applies unit-aware scaling logic:
    /// - `Units`: Direct multiplication (quantity = number of units)
    /// - `Notional`: Direct multiplication (quantity = notional amount; instrument should return unit price)
    /// - `FaceValue`: Direct multiplication (quantity = face value; instrument typically returns full PV)
    /// - `Percentage`: Quantity represents percentage points (e.g., 50 = 50%), always divided by 100
    ///
    /// # Arguments
    ///
    /// * `value` - The monetary value to scale (typically from `instrument.value()`)
    ///
    /// # Returns
    ///
    /// The scaled monetary value in the same currency.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_valuations::instruments::rates::deposit::Deposit;
    /// use std::sync::Arc;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_portfolio::Result<()> {
    /// let instrument = Deposit::builder()
    ///     .id("DEP_1M".into())
    ///     .notional(Money::new(1_000_000.0, Currency::USD))
    ///     .start_date(date!(2024-01-01))
    ///     .maturity(date!(2024-02-01))
    ///     .day_count(finstack_core::dates::DayCount::Act360)
    ///     .discount_curve_id("USD".into())
    ///     .build()
    ///     .expect("example deposit should build");
    ///
    /// let position = Position::new(
    ///     "POS_001",
    ///     "ENTITY_A",
    ///     "DEP_1M",
    ///     Arc::new(instrument),
    ///     50.0,
    ///     PositionUnit::Percentage,
    /// )?;
    ///
    /// let scaled = position.scale_value(Money::new(200.0, Currency::USD));
    /// assert_eq!(scaled.amount(), 100.0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn scale_value(&self, value: Money) -> Money {
        let scale_factor = match self.unit {
            PositionUnit::Units => self.quantity,
            PositionUnit::Notional(unit_ccy) => {
                // Warn if notional currency differs from instrument currency
                if let Some(notional_ccy) = unit_ccy {
                    if notional_ccy != value.currency() {
                        tracing::warn!(
                            position_id = %self.position_id,
                            "Notional currency {} differs from instrument currency {}",
                            notional_ccy, value.currency()
                        );
                    }
                }
                self.quantity
            }
            PositionUnit::FaceValue => self.quantity,
            PositionUnit::Percentage => {
                // Percentage values are always in points: 50 = 50%
                self.quantity / 100.0
            }
        };
        Money::new(value.amount() * scale_factor, value.currency())
    }

    /// Convert this position to a serializable specification.
    ///
    /// Attempts to extract the instrument JSON representation if the instrument
    /// implements the conversion. Returns `None` for `instrument_spec` if conversion
    /// is not supported.
    ///
    /// # Returns
    ///
    /// A serializable `PositionSpec` carrying tags, metadata, and an optional
    /// instrument payload.
    pub fn to_spec(&self) -> PositionSpec {
        // Try to convert instrument to JSON (will be implemented in phase 5.3)
        let instrument_spec = self.instrument.to_instrument_json();

        PositionSpec {
            position_id: self.position_id.clone(),
            entity_id: self.entity_id.clone(),
            instrument_id: self.instrument_id.clone(),
            instrument_spec,
            quantity: self.quantity,
            unit: self.unit,
            book_id: self.book_id.clone(),
            attributes: self.attributes.clone(),
            meta: self.meta.clone(),
        }
    }

    /// Reconstruct a Position from a specification.
    ///
    /// # Arguments
    ///
    /// * `spec` - The position specification to reconstruct
    ///
    /// # Returns
    ///
    /// Reconstructed runtime position with a live instrument trait object.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if:
    /// - The quantity is invalid (NaN/Inf)
    /// - The instrument specification cannot be converted to an instrument
    pub fn from_spec(spec: PositionSpec) -> Result<Self> {
        let PositionSpec {
            position_id,
            entity_id,
            instrument_id,
            instrument_spec,
            quantity,
            unit,
            book_id,
            attributes,
            meta,
        } = spec;

        let instrument = if let Some(instr_json) = instrument_spec {
            Arc::from(instr_json.into_boxed().map_err(|e| {
                Error::invalid_input(format!("Failed to convert instrument JSON: {}", e))
            })?)
        } else {
            return Err(Error::invalid_input(
                "Cannot reconstruct position without instrument_spec".to_string(),
            ));
        };

        let mut position = Self::new(
            position_id,
            entity_id,
            instrument_id,
            instrument,
            quantity,
            unit,
        )?;
        position.book_id = book_id;
        position.attributes = attributes;
        position.meta = meta;
        Ok(position)
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Position")
            .field("position_id", &self.position_id)
            .field("entity_id", &self.entity_id)
            .field("instrument_id", &self.instrument_id)
            .field("quantity", &self.quantity)
            .field("unit", &self.unit)
            .field("attributes", &self.attributes)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.position_id == other.position_id
    }
}

impl Eq for Position {}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use time::macros::date;

    #[test]
    fn test_position_creation() {
        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start_date(date!(2024 - 01 - 01))
            .maturity(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .expect("test should succeed");

        let position = Position::new(
            "POS_001",
            "FUND_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed")
        .with_text_attribute("type", "cash")
        .with_text_attribute("rating", "AAA");

        assert_eq!(position.position_id, "POS_001");
        assert_eq!(position.entity_id, "FUND_A");
        assert_eq!(position.instrument_id, "DEP_1M");
        assert!(position.is_long());
        assert!(!position.is_short());
        assert_eq!(
            position.attributes.get("type"),
            Some(&AttributeValue::Text("cash".to_string()))
        );
    }

    #[test]
    fn test_position_unit_serialization() {
        let unit = PositionUnit::Notional(Some(Currency::USD));
        let json = serde_json::to_string(&unit).expect("test should succeed");
        assert!(json.contains("notional"));
    }
}
