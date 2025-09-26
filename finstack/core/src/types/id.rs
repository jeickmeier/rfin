//! Phantom-typed identifiers to prevent accidental mixing of different ID types
//!
//! This module provides a generic `Id<T>` type that uses phantom types to ensure
//! type safety when dealing with different kinds of identifiers. This prevents
//! common bugs like passing a user ID where an account ID was expected.
//!
//! See unit tests and `examples/` for usage.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Marker trait for types that can be used as ID tags
///
/// This trait is automatically implemented for all types, but serves as
/// documentation for the phantom type parameter of `Id<T>`.
pub trait TypeTag {}

// Blanket implementation for all types
impl<T> TypeTag for T {}

/// A phantom-typed identifier that prevents mixing different kinds of IDs
///
/// This type wraps a string identifier with a phantom type parameter to ensure
/// type safety at compile time. Different `Id<T>` types with different `T`
/// cannot be compared or mixed accidentally.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Id<T: TypeTag> {
    value: Arc<str>,
    #[cfg_attr(feature = "serde", serde(skip))]
    _marker: PhantomData<T>,
}

impl<T: TypeTag> Id<T> {
    /// Create a new ID with the given string value
    pub fn new(value: impl Into<String>) -> Self {
        let s: String = value.into();
        Self {
            value: Arc::<str>::from(s),
            _marker: PhantomData,
        }
    }

    /// Get the string representation of this ID
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Convert this ID into its string representation
    pub fn into_string(self) -> String {
        self.value.as_ref().to_owned()
    }

    /// Create an ID from a string slice
    pub fn from_string_slice(value: &str) -> Self {
        Self {
            value: Arc::<str>::from(value),
            _marker: PhantomData,
        }
    }

    /// Check if this ID is empty
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Get the length of the ID string
    pub fn len(&self) -> usize {
        self.value.len()
    }
}

// Implement common traits

impl<T: TypeTag> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: TypeTag> Eq for Id<T> {}

impl<T: TypeTag> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: TypeTag> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: TypeTag> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: TypeTag> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: TypeTag> From<String> for Id<T> {
    fn from(value: String) -> Self {
        Self {
            value: Arc::<str>::from(value),
            _marker: PhantomData,
        }
    }
}

impl<T: TypeTag> From<&str> for Id<T> {
    fn from(value: &str) -> Self {
        Self {
            value: Arc::<str>::from(value),
            _marker: PhantomData,
        }
    }
}

impl<T: TypeTag> AsRef<str> for Id<T> {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl<T: TypeTag> Deref for Id<T> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: TypeTag> std::borrow::Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        &self.value
    }
}

impl<T: TypeTag> std::str::FromStr for Id<T> {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            value: Arc::<str>::from(s),
            _marker: PhantomData,
        })
    }
}

// Common ID marker types for finstack domains

/// Marker type for curve identifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct CurveTag;

/// Marker type for instrument identifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct InstrumentTag;

/// Marker type for index identifiers (equity or fixed income)
#[derive(Debug, Clone, Copy, Default)]
pub struct IndexTag;

/// Marker type for price/market-scalar identifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct PriceTag;

/// Type aliases for common ID types
/// Type-safe identifier for market data curves
pub type CurveId = Id<CurveTag>;
/// Type-safe identifier for financial instruments
pub type InstrumentId = Id<InstrumentTag>;
/// Type-safe identifier for market indices
pub type IndexId = Id<IndexTag>;
/// Type-safe identifier for market prices/scalars
pub type PriceId = Id<PriceTag>;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct User;
    #[derive(Debug)]
    struct Account;

    #[test]
    fn id_creation_and_access() {
        let id = Id::<User>::new("user123");
        assert_eq!(id.as_str(), "user123");
        assert_eq!(id.to_string(), "user123");
        assert_eq!(id.len(), 7);
        assert!(!id.is_empty());
    }

    #[test]
    fn id_equality() {
        let id1 = Id::<User>::new("user123");
        let id2 = Id::<User>::new("user123");
        let id3 = Id::<User>::new("user456");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn id_ordering() {
        let id1 = Id::<User>::new("aaa");
        let id2 = Id::<User>::new("bbb");
        let id3 = Id::<User>::new("ccc");

        assert!(id1 < id2);
        assert!(id2 < id3);
        assert!(id1 < id3);
    }

    #[test]
    fn id_hashing() {
        use hashbrown::HashMap;

        let mut map = HashMap::new();
        let id1 = Id::<User>::new("user123");
        let id2 = Id::<User>::new("user123");

        map.insert(id1, "value1");
        assert_eq!(map.get(&id2), Some(&"value1"));
    }

    #[test]
    fn type_safety() {
        let user_id = Id::<User>::new("123");
        let account_id = Id::<Account>::new("123");

        // These should be different types even with same string value
        // Uncommenting this would cause a compile error:
        // assert_eq!(user_id, account_id);

        // But we can convert to string for comparison if needed
        assert_eq!(user_id.as_str(), account_id.as_str());
    }

    #[test]
    fn conversions() {
        let id1 = Id::<User>::new("test");
        let id2 = Id::<User>::from("test");
        let id3 = Id::<User>::from(String::from("test"));

        assert_eq!(id1, id2);
        assert_eq!(id2, id3);

        let string_val: &str = id1.as_ref();
        assert_eq!(string_val, "test");
    }

    #[test]
    fn common_id_types() {
        let curve_id = CurveId::new("USD-OIS");
        let instrument_id = InstrumentId::new("BOND123");
        let price_id = PriceId::new("AAPL");

        assert_eq!(curve_id.as_str(), "USD-OIS");
        assert_eq!(instrument_id.as_str(), "BOND123");
        assert_eq!(price_id.as_str(), "AAPL");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let id = Id::<User>::new("user123");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: Id<User> = serde_json::from_str(&json).unwrap();

        assert_eq!(id, deserialized);
    }
}
