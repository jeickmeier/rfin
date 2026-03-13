// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// FX Swap discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<FxSwap>::discounting(InstrumentType::FxSwap)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<FxSwap>::discounting(InstrumentType::FxSwap)` directly"
)]
pub type SimpleFxSwapDiscountingPricer = GenericInstrumentPricer<crate::instruments::FxSwap>;

#[allow(deprecated)]
impl Default for SimpleFxSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::FxSwap)
    }
}
