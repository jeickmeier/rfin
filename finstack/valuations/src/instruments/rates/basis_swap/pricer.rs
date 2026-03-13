// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Basis Swap discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<BasisSwap>::discounting(InstrumentType::BasisSwap)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<BasisSwap>::discounting(InstrumentType::BasisSwap)` directly"
)]
pub type SimpleBasisSwapDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::rates::basis_swap::BasisSwap>;

#[allow(deprecated)]
impl Default for SimpleBasisSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::BasisSwap)
    }
}
