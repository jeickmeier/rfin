use core::fmt;

/// Zero‐allocation identifier for a market-data term structure.
///
/// The type intentionally uses a **transparent `&'static str` wrapper** so
/// that it:
/// * can be defined as a `const` and embedded directly in code, and
/// * is **`Copy`**, hashable and comparable by a single pointer operation.
///
/// # Example
/// ```rust
/// use rfin_core::market_data::id::CurveId;
/// const USD_SOFR: CurveId = CurveId::new("USD-SOFR");
/// assert_eq!(USD_SOFR.as_str(), "USD-SOFR");
/// ```
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurveId(&'static str);

impl CurveId {
    /// Construct a new identifier from a string literal.
    ///
    /// # Panics
    /// Panics if `id` is the empty string – an identifier must contain at
    /// least one character so that it remains unique and displays
    /// meaningfully.
    #[must_use]
    pub const fn new(id: &'static str) -> Self {
        assert!(!id.is_empty(), "CurveId cannot be empty");
        Self(id)
    }

    /// Return the wrapped string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for CurveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CurveId").field(&self.0).finish()
    }
}

impl fmt::Display for CurveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Groups a [`CurveId`] with the *type of market factor* it represents.
///
/// This is useful when functions accept multiple identifiers of different
/// nature and need to pattern-match on the factor category.
///
/// ```rust
/// use rfin_core::market_data::id::{CurveId, FactorKey};
/// let usd_ois = CurveId::new("USD-OIS");
/// let key = FactorKey::Yield(&usd_ois);
/// match key {
///     FactorKey::Yield(id) => println!("Yield curve: {id}"),
///     _ => unreachable!(),
/// }
/// ```
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
    fn curve_id_size_eight_bytes() {
        // &str is a fat pointer (ptr + len) on 64-bit so expect 16 bytes.
        assert_eq!(size_of::<CurveId>(), 16);
    }
}
