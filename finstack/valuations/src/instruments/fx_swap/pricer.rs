// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// FX Swap discounting pricer using the generic implementation.
pub type SimpleFxSwapDiscountingPricer = GenericDiscountingPricer<crate::instruments::FxSwap>;

impl Default for SimpleFxSwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::FxSwap)
    }
}

// Auto-register FxSwap discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleFxSwapDiscountingPricer::default()),
    }
}
