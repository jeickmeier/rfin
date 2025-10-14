//! Position types for holding instruments in a portfolio.

use crate::types::{EntityId, PositionId};
use finstack_core::prelude::*;
use finstack_valuations::instruments::common::traits::Instrument;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unit of position measurement.
///
/// The unit describes how the `quantity` on a [`Position`] should be interpreted.
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::PositionUnit;
/// use finstack_core::prelude::Currency;
///
/// let unit = PositionUnit::Notional(Some(Currency::USD));
/// match unit {
///     PositionUnit::Notional(Some(ccy)) => assert_eq!(ccy, Currency::USD),
///     _ => unreachable!(),
/// }
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionUnit {
    /// Number of units/shares (for equities, baskets)
    Units,

    /// Notional amount, optionally in a specific currency (for derivatives, FX)
    Notional(Option<Currency>),

    /// Face value of debt instruments (for bonds, loans)
    FaceValue,

    /// Percentage of ownership
    Percentage,
}

/// A position in an instrument.
///
/// Represents a holding of a specific quantity of an instrument,
/// belonging to an entity. Positions track the instrument reference,
/// quantity, and metadata for aggregation and analysis.
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::{Position, PositionUnit};
/// use finstack_core::prelude::*;
/// use std::sync::Arc;
///
/// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
/// #     .id("DEP".into())
/// #     .notional(Money::new(1.0, Currency::USD))
/// #     .start(time::macros::date!(2024 - 01 - 01))
/// #     .end(time::macros::date!(2024 - 02 - 01))
/// #     .day_count(finstack_core::dates::DayCount::Act360)
/// #     .disc_id("USD".into())
/// #     .build()
/// #     .unwrap());
/// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), 1.0, PositionUnit::Units);
/// assert!(position.is_long());
/// ```
#[derive(Clone)]
pub struct Position {
    /// Unique identifier for this position
    pub position_id: PositionId,

    /// Entity that owns this position
    pub entity_id: EntityId,

    /// Instrument identifier (for reference/lookup)
    pub instrument_id: String,

    /// The actual instrument being held
    pub instrument: Arc<dyn Instrument>,

    /// Signed quantity (positive=long, negative=short)
    pub quantity: f64,

    /// Unit of measurement for the quantity
    pub unit: PositionUnit,

    /// Position-level tags for attribute-based grouping
    pub tags: IndexMap<String, String>,

    /// Additional metadata
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
    /// * `quantity` - Signed quantity of the instrument.
    /// * `unit` - Interpretation of the quantity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), 1.0, PositionUnit::Units);
    /// assert_eq!(position.position_id, "POS_1");
    /// ```
    pub fn new(
        position_id: impl Into<PositionId>,
        entity_id: impl Into<EntityId>,
        instrument_id: impl Into<String>,
        instrument: Arc<dyn Instrument>,
        quantity: f64,
        unit: PositionUnit,
    ) -> Self {
        Self {
            position_id: position_id.into(),
            entity_id: entity_id.into(),
            instrument_id: instrument_id.into(),
            instrument,
            quantity,
            unit,
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }

    /// Add a tag to the position.
    ///
    /// Tags are stored in an [`indexmap::IndexMap`] to preserve insertion order.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key.
    /// * `value` - Tag value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), 1.0, PositionUnit::Units)
    ///     .with_tag("desk", "rates");
    /// assert_eq!(position.tags.get("desk"), Some(&"rates".into()));
    /// ```
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Add metadata.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key.
    /// * `value` - Arbitrary JSON value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use serde_json::json;
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), 1.0, PositionUnit::Units)
    ///     .with_meta("notes", json!({"owner": "desk"}));
    /// assert!(position.meta.contains_key("notes"));
    /// ```
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }

    /// Check if this position is long (positive quantity).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), 1.0, PositionUnit::Units);
    /// assert!(position.is_long());
    /// ```
    pub fn is_long(&self) -> bool {
        self.quantity > 0.0
    }

    /// Check if this position is short (negative quantity).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ENTITY_A", "DEP", Arc::clone(&instrument), -1.0, PositionUnit::Units);
    /// assert!(position.is_short());
    /// ```
    pub fn is_short(&self) -> bool {
        self.quantity < 0.0
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
            .field("tags", &self.tags)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_valuations::instruments::deposit::Deposit;
    use time::macros::date;

    #[test]
    fn test_position_creation() {
        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(date!(2024 - 01 - 01))
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .disc_id("USD".into())
            .build()
            .unwrap();

        let position = Position::new(
            "POS_001",
            "FUND_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .with_tag("type", "cash")
        .with_tag("rating", "AAA");

        assert_eq!(position.position_id, "POS_001");
        assert_eq!(position.entity_id, "FUND_A");
        assert_eq!(position.instrument_id, "DEP_1M");
        assert!(position.is_long());
        assert!(!position.is_short());
        assert_eq!(position.tags.get("type"), Some(&"cash".to_string()));
    }

    #[test]
    fn test_position_unit_serialization() {
        let unit = PositionUnit::Notional(Some(Currency::USD));
        let json = serde_json::to_string(&unit).unwrap();
        assert!(json.contains("notional"));
    }
}
