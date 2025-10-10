// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::rmbs::Rmbs;

/// RMBS discounting pricer using the generic implementation.
pub type RmbsDiscountingPricer = GenericDiscountingPricer<Rmbs>;

impl Default for RmbsDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::RMBS)
    }
}

// Auto-register RMBS discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(RmbsDiscountingPricer::default()),
    }
}
