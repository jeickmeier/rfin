//! Builder pattern for assembling [`Portfolio`] instances.
//!
//! This module provides a fluent interface for constructing validated portfolios.
//! The builder keeps track of entities, positions, metadata and validates the resulting
//! [`Portfolio`] to ensure it is internally consistent.
//! Typical usage starts by creating a builder with [`PortfolioBuilder::new`], chaining
//! configuration methods, and finalizing with [`PortfolioBuilder::build`].

use crate::error::Result;
use crate::portfolio::Portfolio;
use crate::position::Position;
use crate::types::{Entity, EntityId, DUMMY_ENTITY_ID};
use finstack_core::prelude::*;
use indexmap::IndexMap;

/// Builder for constructing a [`Portfolio`] with validation.
///
/// The builder stores all intermediate values needed to construct a portfolio and checks
/// invariants such as base currency, valuation date, and entity references before the
/// final portfolio is produced.
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::{PortfolioBuilder, Entity};
/// use finstack_core::prelude::*;
/// use time::macros::date;
///
/// let portfolio = PortfolioBuilder::new("FUND_A")
///     .name("Alpha Fund")
///     .base_ccy(Currency::USD)
///     .as_of(date!(2024 - 01 - 01))
///     .entity(Entity::new("ACME"))
///     .build()
///     .unwrap();
///
/// assert_eq!(portfolio.id, "FUND_A");
/// ```
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
    /// Create a new portfolio builder with the given identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the portfolio; converted into an owned `String`.
    ///
    /// # Examples
    ///
/// ```rust
/// use finstack_portfolio::PortfolioBuilder;
///
/// let builder = PortfolioBuilder::new("FUND_A");
/// // Additional configuration can be chained before calling `build`.
/// # let _ = builder;
/// ```
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
    
    /// Set the portfolio's human-readable name.
    ///
    /// # Arguments
    ///
    /// * `name` - Display name stored alongside the portfolio identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::PortfolioBuilder;
    ///
    /// let portfolio = PortfolioBuilder::new("FUND_A")
    ///     .name("Alpha Fund");
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Declare the portfolio's reporting currency.
    ///
    /// # Arguments
    ///
    /// * `ccy` - Currency to use when consolidating values and metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::PortfolioBuilder;
    /// use finstack_core::prelude::Currency;
    ///
    /// let builder = PortfolioBuilder::new("FUND_A").base_ccy(Currency::USD);
    /// # let _ = builder;
    /// ```
    pub fn base_ccy(mut self, ccy: Currency) -> Self {
        self.base_ccy = Some(ccy);
        self
    }
    
    /// Assign the valuation date used for pricing and analytics.
    ///
    /// # Arguments
    ///
    /// * `date` - The as-of date for valuation and risk calculation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::PortfolioBuilder;
    /// use time::macros::date;
    ///
    /// let builder = PortfolioBuilder::new("FUND_A").as_of(date!(2024 - 01 - 01));
    /// # let _ = builder;
    /// ```
    pub fn as_of(mut self, date: Date) -> Self {
        self.as_of = Some(date);
        self
    }
    
    /// Register a single [`Entity`] with the builder.
    ///
    /// # Arguments
    ///
    /// * `entity` - Entity definition including identifier, tags and metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{PortfolioBuilder, Entity};
    ///
    /// let builder = PortfolioBuilder::new("FUND_A")
    ///     .entity(Entity::new("ACME_CORP"));
    /// # let _ = builder;
    /// ```
    pub fn entity(mut self, entity: Entity) -> Self {
        self.entities.insert(entity.id.clone(), entity);
        self
    }
    
    /// Register multiple entities in a single call.
    ///
    /// # Arguments
    ///
    /// * `entities` - Iterator yielding entities to insert into the portfolio.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{PortfolioBuilder, Entity};
    ///
    /// let builder = PortfolioBuilder::new("FUND_A")
    ///     .entities([Entity::new("ACME"), Entity::new("MEGA_CORP")]);
    /// # let _ = builder;
    /// ```
    pub fn entities(mut self, entities: impl IntoIterator<Item = Entity>) -> Self {
        for entity in entities {
            self.entities.insert(entity.id.clone(), entity);
        }
        self
    }
    
    /// Add a single position to the portfolio.
    ///
    /// # Arguments
    ///
    /// * `position` - Fully constructed [`Position`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
    /// use std::sync::Arc;
    ///
    /// # let instrument = Arc::new(finstack_valuations::instruments::deposit::Deposit::builder()
    /// #     .id("DEP".into())
    /// #     .notional(finstack_core::prelude::Money::new(1.0, finstack_core::prelude::Currency::USD))
    /// #     .start(time::macros::date!(2024 - 01 - 01))
    /// #     .end(time::macros::date!(2024 - 02 - 01))
    /// #     .day_count(finstack_core::dates::DayCount::Act360)
    /// #     .disc_id("USD".into())
    /// #     .build()
    /// #     .unwrap());
    /// let position = Position::new("POS_1", "ACME", "INST_1", Arc::clone(&instrument), 1.0, PositionUnit::Units);
    /// let builder = PortfolioBuilder::new("FUND_A").position(position);
    /// ```
    pub fn position(mut self, position: Position) -> Self {
        self.positions.push(position);
        self
    }
    
    /// Add multiple positions from any iterator.
    ///
    /// # Arguments
    ///
    /// * `positions` - Iterator yielding positions to append to the builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{PortfolioBuilder, Position};
    ///
    /// let positions: Vec<Position> = Vec::new();
    /// let builder = PortfolioBuilder::new("FUND_A").positions(positions);
    /// ```
    pub fn positions(mut self, positions: impl IntoIterator<Item = Position>) -> Self {
        self.positions.extend(positions);
        self
    }
    
    /// Apply a portfolio-level tag.
    ///
    /// Tags allow categorisation and filtering of portfolios.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag identifier.
    /// * `value` - Tag value stored as a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::PortfolioBuilder;
    ///
    /// let builder = PortfolioBuilder::new("FUND_A")
    ///     .tag("strategy", "fixed_income");
    /// ```
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
    
    /// Attach an arbitrary JSON metadata entry.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key.
    /// * `value` - JSON payload stored alongside the portfolio.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::PortfolioBuilder;
    /// use serde_json::json;
    ///
    /// let builder = PortfolioBuilder::new("FUND_A")
    ///     .meta("notes", json!({"owner": "front-office"}));
    /// ```
    pub fn meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }
    
    /// Build the portfolio with validation and entity reconciliation.
    ///
    /// This method performs several checks before returning a portfolio:
    /// 1. Ensures the base currency and valuation date are configured.
    /// 2. Injects a dummy entity when positions reference [`DUMMY_ENTITY_ID`].
    /// 3. Delegates to [`Portfolio::validate`](crate::portfolio::Portfolio::validate) to confirm
    ///    entity references and portfolio structure.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError`](crate::error::PortfolioError) when the configuration is incomplete
    /// or validation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{PortfolioBuilder, Entity};
    /// use finstack_core::prelude::*;
    /// use time::macros::date;
    ///
    /// let portfolio = PortfolioBuilder::new("FUND_A")
    ///     .base_ccy(Currency::USD)
    ///     .as_of(date!(2024 - 01 - 01))
    ///     .entity(Entity::new("ACME"))
    ///     .build()
    ///     .unwrap();
    /// ```
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
