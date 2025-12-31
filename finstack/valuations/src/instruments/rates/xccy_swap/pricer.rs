// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;

/// XCCY swap discounting pricer using the generic implementation.
pub type SimpleXccySwapDiscountingPricer = GenericInstrumentPricer<crate::instruments::XccySwap>;

impl Default for SimpleXccySwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::XccySwap)
    }
}
