use core::fmt;
extern crate alloc;
use alloc::sync::Arc;

/// Identifier for a market-data term structure.
///
/// Wraps an `Arc<str>` so it can be created dynamically at runtime while
/// remaining cheap to clone and share across the system.
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CurveId(Arc<str>);

impl CurveId {
    /// Construct a new identifier from a string-like value.
    ///
    /// # Panics
    /// Panics if `id` is the empty string – an identifier must contain at
    /// least one character so that it remains unique and displays
    /// meaningfully.
    #[must_use]
    pub fn new(id: impl AsRef<str>) -> Self {
        let s = id.as_ref();
        assert!(!s.is_empty(), "CurveId cannot be empty");
        Self(Arc::<str>::from(s))
    }

    /// Return the wrapped string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CurveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CurveId").field(&self.0).finish()
    }
}

impl fmt::Display for CurveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Groups a [`CurveId`] with the *type of market factor* it represents.
///
/// This is useful when functions accept multiple identifiers of different
/// nature and need to pattern-match on the factor category.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FactorKey<'a> {
    /// Discount / yield curve reference.
    Yield(&'a CurveId),
    /// Credit hazard curve reference.
    Hazard(&'a CurveId),
    /// Volatility surface reference.
    VolSurface(&'a CurveId),
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn curve_id_equality() {
        let a = CurveId::new("USD-OIS");
        let b = CurveId::new("USD-OIS");
        let c = CurveId::new("EUR-OIS");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn curve_id_is_small_and_cloneable() {
        // Arc<str> is a fat pointer (ptr + len) like &str on 64-bit → 16 bytes.
        assert_eq!(size_of::<CurveId>(), size_of::<&str>());
    }
}
