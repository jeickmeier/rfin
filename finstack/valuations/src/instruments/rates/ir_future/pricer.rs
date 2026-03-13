// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// IR Future discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<InterestRateFuture>::discounting(InstrumentType::InterestRateFuture)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<InterestRateFuture>::discounting(InstrumentType::InterestRateFuture)` directly"
)]
pub type SimpleIrFutureDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::rates::ir_future::InterestRateFuture>;

#[allow(deprecated)]
impl Default for SimpleIrFutureDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InterestRateFuture)
    }
}
