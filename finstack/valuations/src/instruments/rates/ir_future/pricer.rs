// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// IR Future discounting pricer using the generic implementation.
pub type SimpleIrFutureDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::rates::ir_future::InterestRateFuture>;

impl Default for SimpleIrFutureDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InterestRateFuture)
    }
}
