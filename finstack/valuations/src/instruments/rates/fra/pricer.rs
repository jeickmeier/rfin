// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// FRA discounting pricer using the generic implementation.
pub type SimpleFraDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::ForwardRateAgreement>;

impl Default for SimpleFraDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::FRA)
    }
}
