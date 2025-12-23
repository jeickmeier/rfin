// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::repo::Repo;

/// Repo discounting pricer using the generic implementation.
pub type SimpleRepoDiscountingPricer = GenericDiscountingPricer<Repo>;

impl Default for SimpleRepoDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::Repo)
    }
}
