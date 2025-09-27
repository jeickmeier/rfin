// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// FRA discounting pricer using the generic implementation.
pub type SimpleFraDiscountingPricer = GenericDiscountingPricer<crate::instruments::ForwardRateAgreement>;

impl Default for SimpleFraDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}
