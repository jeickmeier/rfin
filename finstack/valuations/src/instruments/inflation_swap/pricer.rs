// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Inflation Swap discounting pricer using the generic implementation.
pub type SimpleInflationSwapDiscountingPricer =
    GenericDiscountingPricer<crate::instruments::InflationSwap>;

impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::InflationSwap)
    }
}

// Auto-register InflationSwap discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleInflationSwapDiscountingPricer::default()),
    }
}
