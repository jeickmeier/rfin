//! Portfolio struct and core operations.
//!
//! A portfolio represents a book of positions held by one or more entities.
//! The module provides helpers for traversing positions, filtering by tags, and
//! validating structural invariants before valuation takes place.

use crate::error::{PortfolioError, Result};
use crate::position::Position;
use crate::types::{Entity, EntityId, DUMMY_ENTITY_ID};
use finstack_core::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A portfolio of positions across multiple entities.
///
/// The portfolio holds a flat list of positions, each referencing an entity
/// and instrument. Positions can be grouped and aggregated by entity or by
/// arbitrary attributes (tags).
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::{Portfolio, Entity};
/// use finstack_core::prelude::*;
/// use time::macros::date;
///
/// let mut portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));
/// portfolio.entities.insert("ACME".into(), Entity::new("ACME"));
/// assert_eq!(portfolio.base_ccy, Currency::USD);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portfolio {
    /// Unique identifier for this portfolio
    pub id: String,

    /// Human-readable name
    pub name: Option<String>,

    /// Base currency for aggregation
    pub base_ccy: Currency,

    /// Valuation date
    pub as_of: Date,

    /// Entities that own positions
    pub entities: IndexMap<EntityId, Entity>,

    /// Flat list of positions (not serialized directly due to Instrument trait)
    #[serde(skip)]
    pub positions: Vec<Position>,

    /// Portfolio-level tags
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub tags: IndexMap<String, String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl Portfolio {
    /// Create a new empty portfolio.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique portfolio identifier.
    /// * `base_ccy` - Reporting currency.
    /// * `as_of` - Valuation date.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::Portfolio;
    /// use finstack_core::prelude::*;
    /// use time::macros::date;
    ///
    /// let portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));
    /// assert_eq!(portfolio.id, "FUND_A");
    /// ```
    pub fn new(id: impl Into<String>, base_ccy: Currency, as_of: Date) -> Self {
        Self {
            id: id.into(),
            name: None,
            base_ccy,
            as_of,
            entities: IndexMap::new(),
            positions: Vec::new(),
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }

    /// Get a position by identifier.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier of the position to locate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Portfolio, Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let mut portfolio = Portfolio::new("FUND_A", Currency::USD, time::macros::date!(2024 - 01 - 01));
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
/// #     .build()
/// #     .unwrap());
/// # portfolio.positions.push(Position::new("POS_1", "ACME", "DEP", instrument.clone(), 1.0, PositionUnit::Units));
/// assert!(portfolio.get_position("POS_1").is_some());
/// ```
pub fn get_position(&self, position_id: &str) -> Option<&Position> {
        self.positions.iter().find(|p| p.position_id == position_id)
    }

    /// Get all positions for a given entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Entity identifier used for filtering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Portfolio, Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let mut portfolio = Portfolio::new("FUND_A", Currency::USD, time::macros::date!(2024 - 01 - 01));
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
/// #     .build()
/// #     .unwrap());
/// # portfolio.positions.push(Position::new("POS_1", "ENTITY_A", "DEP", instrument.clone(), 1.0, PositionUnit::Units));
/// let entity_id = "ENTITY_A".to_string();
/// assert_eq!(portfolio.positions_for_entity(&entity_id).len(), 1);
/// ```
pub fn positions_for_entity(&self, entity_id: &EntityId) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| &p.entity_id == entity_id)
            .collect()
    }

    /// Get all positions with a specific tag value.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key to inspect.
    /// * `value` - Desired tag value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Portfolio, Position, PositionUnit};
    /// use finstack_core::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # let mut portfolio = Portfolio::new("FUND_A", Currency::USD, time::macros::date!(2024 - 01 - 01));
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(Money::new(1.0, Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
/// #     .build()
/// #     .unwrap());
/// # let position = Position::new("POS_1", "ENTITY_A", "DEP", instrument.clone(), 1.0, PositionUnit::Units)
/// #     .with_tag("desk", "rates");
/// # portfolio.positions.push(position);
/// assert_eq!(portfolio.positions_with_tag("desk", "rates").len(), 1);
/// ```
pub fn positions_with_tag(&self, key: &str, value: &str) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| p.tags.get(key).map(|v| v.as_str()) == Some(value))
            .collect()
    }

    /// Validate the portfolio structure.
    ///
    /// Checks that:
    /// - All positions reference valid entities  
    /// - Dummy entity exists if needed
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError::UnknownEntity`] when a position references an entity
    /// that is not present in [`Portfolio::entities`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Portfolio, Entity};
    /// use finstack_core::prelude::*;
    /// use time::macros::date;
    ///
    /// let mut portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));
    /// portfolio.entities.insert("ACME".into(), Entity::new("ACME"));
    /// assert!(portfolio.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        for position in &self.positions {
            if !self.entities.contains_key(&position.entity_id) {
                return Err(PortfolioError::UnknownEntity {
                    position_id: position.position_id.clone(),
                    entity_id: position.entity_id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Check if the portfolio uses the dummy entity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Portfolio, Entity, DUMMY_ENTITY_ID};
    /// use finstack_core::prelude::*;
    /// use time::macros::date;
    ///
    /// let mut portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));
    /// portfolio.entities.insert(DUMMY_ENTITY_ID.into(), Entity::dummy());
    /// assert!(portfolio.has_dummy_entity());
    /// ```
    pub fn has_dummy_entity(&self) -> bool {
        self.entities.contains_key(DUMMY_ENTITY_ID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;
    use time::macros::date;

    #[test]
    fn test_portfolio_creation() {
        let portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));

        assert_eq!(portfolio.id, "FUND_A");
        assert_eq!(portfolio.base_ccy, Currency::USD);
        assert_eq!(portfolio.as_of, date!(2024 - 01 - 01));
        assert!(portfolio.entities.is_empty());
        assert!(portfolio.positions.is_empty());
    }

    #[test]
    fn test_portfolio_validation() {
        let mut portfolio = Portfolio::new("FUND_A", Currency::USD, date!(2024 - 01 - 01));

        // Add entity
        portfolio
            .entities
            .insert("ACME".to_string(), Entity::new("ACME"));

        // Valid portfolio
        assert!(portfolio.validate().is_ok());
    }
}
