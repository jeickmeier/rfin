// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;
use crate::instruments::rates::repo::Repo;

/// Repo discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<Repo>::discounting(InstrumentType::Repo)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<Repo>::discounting(InstrumentType::Repo)` directly"
)]
pub type SimpleRepoDiscountingPricer = GenericInstrumentPricer<Repo>;

#[allow(deprecated)]
impl Default for SimpleRepoDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Repo)
    }
}
