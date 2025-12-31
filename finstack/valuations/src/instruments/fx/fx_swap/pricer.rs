// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;

/// FX Swap discounting pricer using the generic implementation.
pub type SimpleFxSwapDiscountingPricer = GenericInstrumentPricer<crate::instruments::FxSwap>;

impl Default for SimpleFxSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::FxSwap)
    }
}
