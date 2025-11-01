//! Trait for instruments with pricing overrides.
//!
//! Provides a common interface for accessing and mutating pricing overrides,
//! which is needed for setting MC seed scenarios in generic FD greek calculators.

use crate::instruments::PricingOverrides;

/// Trait for instruments that have pricing overrides.
///
/// This trait allows generic metric calculators to set MC seed scenarios
/// and other pricing overrides for deterministic greek calculations.
pub trait HasPricingOverrides {
    /// Returns mutable access to pricing overrides.
    fn pricing_overrides_mut(&mut self) -> &mut PricingOverrides;
}
