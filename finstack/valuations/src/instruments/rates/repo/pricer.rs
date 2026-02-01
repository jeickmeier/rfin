// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;
use crate::instruments::rates::repo::Repo;

/// Repo discounting pricer using the generic implementation.
pub type SimpleRepoDiscountingPricer = GenericInstrumentPricer<Repo>;

impl Default for SimpleRepoDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Repo)
    }
}
