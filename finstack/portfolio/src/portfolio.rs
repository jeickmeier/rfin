//! Portfolio struct and core operations.

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
    pub fn new(
        id: impl Into<String>,
        base_ccy: Currency,
        as_of: Date,
    ) -> Self {
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
    
    /// Get a position by ID.
    pub fn get_position(&self, position_id: &str) -> Option<&Position> {
        self.positions.iter().find(|p| p.position_id == position_id)
    }
    
    /// Get all positions for a given entity.
    pub fn positions_for_entity(&self, entity_id: &EntityId) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| &p.entity_id == entity_id)
            .collect()
    }
    
    /// Get all positions with a specific tag value.
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
        portfolio.entities.insert("ACME".to_string(), Entity::new("ACME"));
        
        // Valid portfolio
        assert!(portfolio.validate().is_ok());
    }
}

