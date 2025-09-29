// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Deposit discounting pricer using the generic implementation.
pub type SimpleDepositDiscountingPricer = GenericDiscountingPricer<crate::instruments::Deposit>;

impl Default for SimpleDepositDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

// Auto-register Deposit discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleDepositDiscountingPricer::new()),
    }
}
