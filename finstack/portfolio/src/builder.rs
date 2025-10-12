//! Builder pattern for constructing portfolios.

use crate::error::Result;
use crate::portfolio::Portfolio;
use crate::position::Position;
use crate::types::{Entity, EntityId, DUMMY_ENTITY_ID};
use finstack_core::prelude::*;
use indexmap::IndexMap;

/// Builder for constructing a portfolio with validation.
///
/// The builder ensures all positions reference valid entities and
/// automatically creates a dummy entity if any positions reference it.
#[derive(Debug)]
pub struct PortfolioBuilder {
    id: String,
    name: Option<String>,
    base_ccy: Option<Currency>,
    as_of: Option<Date>,
    entities: IndexMap<EntityId, Entity>,
    positions: Vec<Position>,
    tags: IndexMap<String, String>,
    meta: IndexMap<String, serde_json::Value>,
}

impl PortfolioBuilder {
    /// Create a new portfolio builder with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            base_ccy: None,
            as_of: None,
            entities: IndexMap::new(),
            positions: Vec::new(),
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }
    
    /// Set the portfolio name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set the base currency.
    pub fn base_ccy(mut self, ccy: Currency) -> Self {
        self.base_ccy = Some(ccy);
        self
    }
    
    /// Set the valuation date.
    pub fn as_of(mut self, date: Date) -> Self {
        self.as_of = Some(date);
        self
    }
    
    /// Add an entity.
    pub fn entity(mut self, entity: Entity) -> Self {
        self.entities.insert(entity.id.clone(), entity);
        self
    }
    
    /// Add multiple entities.
    pub fn entities(mut self, entities: impl IntoIterator<Item = Entity>) -> Self {
        for entity in entities {
            self.entities.insert(entity.id.clone(), entity);
        }
        self
    }
    
    /// Add a position.
    pub fn position(mut self, position: Position) -> Self {
        self.positions.push(position);
        self
    }
    
    /// Add multiple positions (accepts both Vec and array).
    pub fn positions(mut self, positions: impl IntoIterator<Item = Position>) -> Self {
        self.positions.extend(positions);
        self
    }
    
    /// Add a portfolio-level tag.
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
    
    /// Add metadata.
    pub fn meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }
    
    /// Build the portfolio with validation.
    ///
    /// This method:
    /// 1. Validates that base_ccy and as_of are set
    /// 2. Auto-creates dummy entity if any positions reference it
    /// 3. Validates all positions reference valid entities
    pub fn build(mut self) -> Result<Portfolio> {
        let base_ccy = self.base_ccy.ok_or_else(|| {
            crate::error::PortfolioError::ValidationFailed(
                "Base currency must be set".to_string()
            )
        })?;
        
        let as_of = self.as_of.ok_or_else(|| {
            crate::error::PortfolioError::ValidationFailed(
                "Valuation date (as_of) must be set".to_string()
            )
        })?;
        
        // Auto-create dummy entity if needed
        let needs_dummy = self.positions.iter()
            .any(|p| p.entity_id == DUMMY_ENTITY_ID);
        
        if needs_dummy && !self.entities.contains_key(DUMMY_ENTITY_ID) {
            tracing::debug!("Auto-creating dummy entity for standalone instruments");
            self.entities.insert(
                DUMMY_ENTITY_ID.to_string(),
                Entity::dummy(),
            );
        }
        
        let portfolio = Portfolio {
            id: self.id,
            name: self.name,
            base_ccy,
            as_of,
            entities: self.entities,
            positions: self.positions,
            tags: self.tags,
            meta: self.meta,
        };
        
        // Validate the portfolio
        portfolio.validate()?;
        
        Ok(portfolio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, PositionUnit};
    use crate::types::Entity;
    use finstack_valuations::instruments::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_builder_basic() {
        let portfolio = PortfolioBuilder::new("TEST_PORTFOLIO")
            .name("Test Portfolio")
            .base_ccy(Currency::USD)
            .as_of(date!(2024 - 01 - 01))
            .entity(Entity::new("ACME"))
            .tag("strategy", "fixed_income")
            .build()
            .unwrap();
        
        assert_eq!(portfolio.id, "TEST_PORTFOLIO");
        assert_eq!(portfolio.name, Some("Test Portfolio".to_string()));
        assert_eq!(portfolio.base_ccy, Currency::USD);
        assert!(portfolio.entities.contains_key("ACME"));
    }
    
    #[test]
    fn test_builder_dummy_entity_auto_creation() {
        let deposit = Deposit::builder()
            .id("DEP_1".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(date!(2024 - 01 - 01))
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .disc_id("USD".into())
            .build()
            .unwrap();
        
        let position = Position::new(
            "POS_001",
            DUMMY_ENTITY_ID,
            "DEP_1",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        );
        
        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(date!(2024 - 01 - 01))
            .position(position)
            .build()
            .unwrap();
        
        // Dummy entity should be auto-created
        assert!(portfolio.has_dummy_entity());
        assert_eq!(portfolio.positions.len(), 1);
    }
    
    #[test]
    fn test_builder_validation_fails_without_base_ccy() {
        let result = PortfolioBuilder::new("TEST")
            .as_of(date!(2024 - 01 - 01))
            .build();
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_builder_validation_fails_without_as_of() {
        let result = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .build();
        
        assert!(result.is_err());
    }
}

