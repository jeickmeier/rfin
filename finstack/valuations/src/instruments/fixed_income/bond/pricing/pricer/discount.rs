//! Bond discounting pricer for the pricing registry.

use crate::instruments::fixed_income::bond::types::Bond;

pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Bond discounting pricer using the generic implementation.
///
/// This pricer uses the standard discount curve-based pricing engine for bonds
/// without embedded options or credit adjustments.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<Bond>::discounting(InstrumentType::Bond)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<Bond>::discounting(InstrumentType::Bond)` directly"
)]
pub type SimpleBondDiscountingPricer = GenericInstrumentPricer<Bond>;

#[allow(deprecated)]
impl Default for SimpleBondDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Bond)
    }
}
