// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;
use crate::instruments::cds::types::CreditDefaultSwap;

/// CDS hazard rate pricer using the generic implementation.
pub type SimpleCdsDiscountingPricer = GenericInstrumentPricer<CreditDefaultSwap>;

impl Default for SimpleCdsDiscountingPricer {
    fn default() -> Self {
        Self::cds()
    }
}
