// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// FRA discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<ForwardRateAgreement>::discounting(InstrumentType::FRA)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<ForwardRateAgreement>::discounting(InstrumentType::FRA)` directly"
)]
pub type SimpleFraDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::ForwardRateAgreement>;

#[allow(deprecated)]
impl Default for SimpleFraDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::FRA)
    }
}
