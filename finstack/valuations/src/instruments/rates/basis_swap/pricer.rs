// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;

/// Basis Swap discounting pricer using the generic implementation.
pub type SimpleBasisSwapDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::rates::basis_swap::BasisSwap>;

impl Default for SimpleBasisSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::BasisSwap)
    }
}
