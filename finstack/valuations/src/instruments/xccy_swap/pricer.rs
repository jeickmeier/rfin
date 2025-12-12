// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// XCCY swap discounting pricer using the generic implementation.
pub type SimpleXccySwapDiscountingPricer = GenericDiscountingPricer<crate::instruments::XccySwap>;

impl Default for SimpleXccySwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::XccySwap)
    }
}

