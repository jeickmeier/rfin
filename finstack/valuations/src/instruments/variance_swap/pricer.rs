use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::variance_swap::VarianceSwap;

// Use the generic discounting pricer for registry integration
pub type SimpleVarianceSwapDiscountingPricer = GenericDiscountingPricer<VarianceSwap>;

impl Default for SimpleVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::VarianceSwap)
    }
}

// Auto-register VarianceSwap discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleVarianceSwapDiscountingPricer::default()),
    }
}
