// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;

/// Inflation Swap discounting pricer using the generic implementation.
pub type SimpleInflationSwapDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::InflationSwap>;

impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InflationSwap)
    }
}
