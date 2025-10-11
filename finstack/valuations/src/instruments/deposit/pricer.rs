// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Deposit discounting pricer using the generic implementation.
pub type SimpleDepositDiscountingPricer = GenericDiscountingPricer<crate::instruments::Deposit>;

impl Default for SimpleDepositDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::Deposit)
    }
}
