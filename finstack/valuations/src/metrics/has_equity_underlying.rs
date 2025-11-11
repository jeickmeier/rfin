//! Trait for instruments with an equity underlying.
//!
//! Provides a common interface for accessing the spot price identifier
//! for instruments that depend on an equity underlying (e.g., equity options,
//! exotic options, convertible bonds, etc.).

/// Trait for instruments that have an equity underlying.
///
/// This trait allows generic finite difference greek calculators to work
/// with any instrument that has an equity spot price, regardless of the
/// specific instrument type.
pub trait HasEquityUnderlying {
    /// Returns the identifier for the spot price of the equity underlying.
    ///
    /// This is typically a `CurveId` or `String` that can be used to look up
    /// the spot price in the `MarketContext`.
    fn spot_id(&self) -> &str;
}
