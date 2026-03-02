//! Bond discounting pricer for the pricing registry.

use crate::instruments::fixed_income::bond::types::Bond;

pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Bond discounting pricer using the generic implementation.
///
/// This pricer uses the standard discount curve-based pricing engine for bonds
/// without embedded options or credit adjustments.
pub type SimpleBondDiscountingPricer = GenericInstrumentPricer<Bond>;

impl Default for SimpleBondDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Bond)
    }
}
