//! Portfolio struct and core operations.
//!
//! A portfolio represents a book of positions held by one or more entities.
//! The module provides helpers for traversing positions, filtering by tags, and
//! validating structural invariants before valuation takes place.

use crate::error::{PortfolioError, Result};
use crate::position::Position;
use crate::types::{Entity, EntityId, DUMMY_ENTITY_ID};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A portfolio of positions across multiple entities.
///
/// The portfolio holds a flat list of positions, each referencing an entity
/// and instrument. Positions can be grouped and aggregated by entity or by
/// arbitrary attributes (tags).
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

/// Serializable portfolio specification.
///
/// This struct allows portfolios to be serialized and deserialized by storing
/// positions as `PositionSpec` rather than `Position` (which contains non-serializable
/// `Arc<dyn Instrument>`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioSpec {
    /// Portfolio identifier
    pub id: String,
    /// Human-readable name
    pub name: Option<String>,
    /// Base currency for aggregation
    pub base_ccy: Currency,
    /// Valuation date
    pub as_of: Date,
    /// Entities that own positions
    pub entities: IndexMap<EntityId, Entity>,
    /// Positions as serializable specs
    pub positions: Vec<crate::position::PositionSpec>,
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
    pub fn get_position(&self, position_id: &str) -> Option<&Position> {
        self.positions.iter().find(|p| p.position_id == position_id)
    }

    /// Get all positions for a given entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Entity identifier used for filtering (accepts &str or &EntityId).
    pub fn positions_for_entity(&self, entity_id: &str) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| p.entity_id == entity_id)
            .collect()
    }

    /// Get all positions with a specific tag value.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key to inspect.
    /// * `value` - Desired tag value.
    pub fn positions_with_tag(&self, key: &str, value: &str) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| p.tags.get(key).map(|v| v.as_str()) == Some(value))
            .collect()
    }

    /// Validate the portfolio structure.
    ///
    /// Checks that:
    /// - All position IDs are unique
    /// - All positions reference valid entities
    /// - Dummy entity exists if needed
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError::ValidationFailed`] when duplicate position IDs are found,
    /// or [`PortfolioError::UnknownEntity`] when a position references an entity
    /// that is not present in [`Portfolio::entities`].
    pub fn validate(&self) -> Result<()> {
        use finstack_core::HashSet;

        let mut seen_ids: finstack_core::HashSet<_> = HashSet::default();
        for position in &self.positions {
            // Check for duplicate position IDs
            if !seen_ids.insert(&position.position_id) {
                return Err(PortfolioError::validation(format!(
                    "Duplicate position ID: {}",
                    position.position_id
                )));
            }

            // Check entity exists
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
    pub fn has_dummy_entity(&self) -> bool {
        self.entities.contains_key(DUMMY_ENTITY_ID)
    }

    /// Convert this portfolio to a serializable specification.
    ///
    /// Converts all positions to `PositionSpec` by extracting their instrument
    /// JSON representations. Positions whose instruments don't support serialization
    /// will have `instrument_spec: None` and cannot be fully reconstructed.
    ///
    /// # Returns
    ///
    /// A `PortfolioSpec` that can be serialized to JSON
    pub fn to_spec(&self) -> PortfolioSpec {
        PortfolioSpec {
            id: self.id.clone(),
            name: self.name.clone(),
            base_ccy: self.base_ccy,
            as_of: self.as_of,
            entities: self.entities.clone(),
            positions: self.positions.iter().map(|p| p.to_spec()).collect(),
            tags: self.tags.clone(),
            meta: self.meta.clone(),
        }
    }

    /// Reconstruct a Portfolio from a specification.
    ///
    /// # Arguments
    ///
    /// * `spec` - The portfolio specification to reconstruct
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError`] if:
    /// - Any position specification cannot be converted to a position
    /// - Portfolio validation fails (duplicate IDs, unknown entities)
    pub fn from_spec(spec: PortfolioSpec) -> Result<Self> {
        let positions: Result<Vec<_>> = spec
            .positions
            .into_iter()
            .map(crate::position::Position::from_spec)
            .collect();

        let portfolio = Self {
            id: spec.id,
            name: spec.name,
            base_ccy: spec.base_ccy,
            as_of: spec.as_of,
            entities: spec.entities,
            positions: positions?,
            tags: spec.tags,
            meta: spec.meta,
        };

        // Validate the reconstructed portfolio
        portfolio.validate()?;

        Ok(portfolio)
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
        let entity = Entity::new("ACME");
        portfolio.entities.insert(entity.id.clone(), entity);

        // Valid portfolio
        assert!(portfolio.validate().is_ok());
    }
}
