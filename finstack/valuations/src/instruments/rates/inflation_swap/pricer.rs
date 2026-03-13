// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Inflation Swap discounting pricer using the generic implementation.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<InflationSwap>::discounting(InstrumentType::InflationSwap)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<InflationSwap>::discounting(InstrumentType::InflationSwap)` directly"
)]
pub type SimpleInflationSwapDiscountingPricer =
    GenericInstrumentPricer<crate::instruments::InflationSwap>;

#[allow(deprecated)]
impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InflationSwap)
    }
}
