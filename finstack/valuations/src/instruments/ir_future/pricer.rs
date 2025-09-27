// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// IR Future discounting pricer using the generic implementation.
pub type SimpleIrFutureDiscountingPricer = GenericDiscountingPricer<crate::instruments::ir_future::InterestRateFuture>;

impl Default for SimpleIrFutureDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}
