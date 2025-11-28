//! Core types for portfolio management.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;

/// Entity identifier (company, fund, etc.)
///
/// A newtype wrapper around `String` that provides type safety for entity identifiers,
/// preventing accidental misuse of position IDs where entity IDs are expected.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EntityId(String);

impl EntityId {
    /// Create a new entity identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the identifier as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for EntityId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for EntityId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Borrow<str> for EntityId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for EntityId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for EntityId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<str> for EntityId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

/// Position identifier.
///
/// A newtype wrapper around `String` that provides type safety for position identifiers,
/// preventing accidental misuse of entity IDs where position IDs are expected.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PositionId(String);

impl PositionId {
    /// Create a new position identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the identifier as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for PositionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for PositionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Borrow<str> for PositionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PositionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for PositionId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<str> for PositionId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

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
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Add multiple tags at once.
    ///
    /// # Arguments
    ///
    /// * `tags` - Iterator of (key, value) pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_portfolio::Entity;
    ///
    /// let entity = Entity::new("ACME")
    ///     .with_tags([("sector", "Technology"), ("region", "US")]);
    ///
    /// assert_eq!(entity.tags.get("sector"), Some(&"Technology".to_string()));
    /// assert_eq!(entity.tags.get("region"), Some(&"US".to_string()));
    /// ```
    pub fn with_tags<K, V, I>(mut self, tags: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in tags {
            self.tags.insert(k.into(), v.into());
        }
        self
    }

    /// Create the dummy entity for standalone instruments.
    pub fn dummy() -> Self {
        Self {
            id: EntityId::new(DUMMY_ENTITY_ID),
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

        assert_eq!(entity.id.as_str(), "ACME_CORP");
        assert_eq!(entity.name, Some("Acme Corporation".to_string()));
        assert_eq!(entity.tags.get("sector"), Some(&"Technology".to_string()));
    }

    #[test]
    fn test_dummy_entity() {
        let dummy = Entity::dummy();
        assert_eq!(dummy.id.as_str(), DUMMY_ENTITY_ID);
        assert!(dummy.name.is_some());
    }

    #[test]
    fn test_entity_id_newtype() {
        let id1 = EntityId::new("ENTITY_1");
        let id2: EntityId = "ENTITY_2".into();
        let id3: EntityId = String::from("ENTITY_3").into();

        assert_eq!(id1.as_str(), "ENTITY_1");
        assert_eq!(format!("{}", id2), "ENTITY_2");
        assert_eq!(id3.to_string(), "ENTITY_3");

        // Test equality
        let id1_clone = EntityId::new("ENTITY_1");
        assert_eq!(id1, id1_clone);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_position_id_newtype() {
        let id1 = PositionId::new("POS_001");
        let id2: PositionId = "POS_002".into();

        assert_eq!(id1.as_str(), "POS_001");
        assert_eq!(format!("{}", id2), "POS_002");
    }

    #[test]
    fn test_with_tags() {
        let entity = Entity::new("ACME").with_tags([("sector", "Tech"), ("region", "NA")]);

        assert_eq!(entity.tags.len(), 2);
        assert_eq!(entity.tags.get("sector"), Some(&"Tech".to_string()));
        assert_eq!(entity.tags.get("region"), Some(&"NA".to_string()));
    }
}
