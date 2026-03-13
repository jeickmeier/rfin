// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Deposit discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<Deposit>::discounting(InstrumentType::Deposit)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<Deposit>::discounting(InstrumentType::Deposit)` directly"
)]
pub type SimpleDepositDiscountingPricer = GenericInstrumentPricer<crate::instruments::Deposit>;

#[allow(deprecated)]
impl Default for SimpleDepositDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Deposit)
    }
}
