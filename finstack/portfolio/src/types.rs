//! Core types for portfolio management.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Entity identifier (company, fund, etc.)
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::EntityId;
///
/// let id: EntityId = "ACME_CORP".to_string();
/// assert_eq!(id, "ACME_CORP");
/// ```
pub type EntityId = String;

/// Position identifier
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::PositionId;
///
/// let id: PositionId = "POS_123".to_string();
/// assert_eq!(id, "POS_123");
/// ```
pub type PositionId = String;

/// Constant for the dummy entity used for standalone instruments.
///
/// Standalone instruments (IRS, Deposits, FX, etc.) that don't belong
/// to a specific entity can reference this dummy entity ID.
pub const DUMMY_ENTITY_ID: &str = "_standalone";

/// An entity that can hold positions.
///
/// Entities represent companies, funds, or other legal entities that
/// own instruments. For standalone instruments (derivatives, FX), use
/// the dummy entity.
///
/// # Examples
///
/// ```rust
/// use finstack_portfolio::Entity;
///
/// let entity = Entity::new("ACME").with_name("Acme Corp");
/// assert_eq!(entity.id, "ACME");
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier for the entity
    pub id: EntityId,

    /// Human-readable name
    pub name: Option<String>,

    /// Entity-level tags for grouping and filtering
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub tags: IndexMap<String, String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl Entity {
    /// Create a new entity with the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique entity identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::Entity;
    ///
    /// let entity = Entity::new("ACME_CORP");
    /// assert_eq!(entity.id, "ACME_CORP");
    /// ```
    pub fn new(id: impl Into<EntityId>) -> Self {
        Self {
            id: id.into(),
            name: None,
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }

    /// Set the entity name.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::Entity;
    ///
    /// let entity = Entity::new("ACME").with_name("Acme Corporation");
    /// assert_eq!(entity.name.as_deref(), Some("Acme Corporation"));
    /// ```
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag.
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key.
    /// * `value` - Tag value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::Entity;
    ///
    /// let entity = Entity::new("ACME").with_tag("sector", "Technology");
    /// assert_eq!(entity.tags.get("sector"), Some(&"Technology".into()));
    /// ```
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Create the dummy entity for standalone instruments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::{Entity, DUMMY_ENTITY_ID};
    ///
    /// let dummy = Entity::dummy();
    /// assert_eq!(dummy.id, DUMMY_ENTITY_ID);
    /// ```
    pub fn dummy() -> Self {
        Self {
            id: DUMMY_ENTITY_ID.to_string(),
            name: Some("Standalone Instruments".to_string()),
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new("ACME_CORP")
            .with_name("Acme Corporation")
            .with_tag("sector", "Technology");

        assert_eq!(entity.id, "ACME_CORP");
        assert_eq!(entity.name, Some("Acme Corporation".to_string()));
        assert_eq!(entity.tags.get("sector"), Some(&"Technology".to_string()));
    }

    #[test]
    fn test_dummy_entity() {
        let dummy = Entity::dummy();
        assert_eq!(dummy.id, DUMMY_ENTITY_ID);
        assert!(dummy.name.is_some());
    }
}
