// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// XCCY swap discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<XccySwap>::discounting(InstrumentType::XccySwap)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<XccySwap>::discounting(InstrumentType::XccySwap)` directly"
)]
pub type SimpleXccySwapDiscountingPricer = GenericInstrumentPricer<crate::instruments::XccySwap>;

#[allow(deprecated)]
impl Default for SimpleXccySwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::XccySwap)
    }
}
