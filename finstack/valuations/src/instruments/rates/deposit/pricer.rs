// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Deposit discounting pricer using the generic implementation.
pub type SimpleDepositDiscountingPricer = GenericInstrumentPricer<crate::instruments::Deposit>;

impl Default for SimpleDepositDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Deposit)
    }
}
