// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericInstrumentPricer;
use crate::instruments::repo::Repo;

/// Repo discounting pricer using the generic implementation.
pub type SimpleRepoDiscountingPricer = GenericInstrumentPricer<Repo>;

impl Default for SimpleRepoDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Repo)
    }
}
